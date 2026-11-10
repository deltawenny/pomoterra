//! Win32 hooks marshal setup onto NSH's UI thread (not its engine thread).
use super::native_tray::{Action, NativeTray};
use std::{
    cell::{Cell, RefCell},
    sync::{
        Once,
        atomic::{AtomicIsize, Ordering},
    },
    time::Duration,
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    System::Threading::GetCurrentProcessId,
    UI::{
        HiDpi::{AdjustWindowRectExForDpi, GetDpiForWindow},
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};
use windows::core::BOOL;

static START: Once = Once::new();
static WINDOW: AtomicIsize = AtomicIsize::new(0);
thread_local! {
    static TRAY: RefCell<Option<NativeTray>> = const { RefCell::new(None) };
    static SIZED: Cell<bool> = const { Cell::new(false) };
}
const SUBCLASS: usize = 0x504f4d4f;
const TIMER: usize = 0x504f4d4f;

pub(crate) fn install() {
    START.call_once(|| {
        std::thread::spawn(|| {
            // The root can mount before the top-level window becomes visible.
            for _ in 0..100 {
                unsafe {
                    let _ = EnumWindows(Some(find_window), LPARAM(0));
                }
                let raw = WINDOW.load(Ordering::Acquire);
                if raw != 0 {
                    let hwnd = HWND(raw as *mut _);
                    // A thread hook runs inside the owner's message loop, allowing
                    // tray creation and subclassing without cross-thread UI access.
                    unsafe {
                        let thread = GetWindowThreadProcessId(hwnd, None);
                        match SetWindowsHookExW(WH_CALLWNDPROC, Some(setup_hook), None, thread) {
                            Ok(hook) => {
                                let _ = SendMessageW(hwnd, WM_NULL, None, None);
                                let _ = UnhookWindowsHookEx(hook);
                            }
                            Err(error) => eprintln!("Pomoterra tray setup failed: {error}"),
                        }
                    }
                    return;
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            eprintln!("Pomoterra tray: native window was not found");
        });
    });
}

unsafe extern "system" fn find_window(hwnd: HWND, _: LPARAM) -> BOOL {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if pid == GetCurrentProcessId()
            && IsWindowVisible(hwnd).as_bool()
            && GetWindow(hwnd, GW_OWNER).map_or(true, |owner| owner.0.is_null())
        {
            WINDOW.store(hwnd.0 as isize, Ordering::Release);
            return BOOL(0);
        }
        BOOL(1)
    }
}

unsafe extern "system" fn setup_hook(code: i32, w: WPARAM, l: LPARAM) -> LRESULT {
    unsafe {
        if code >= 0 {
            let message = &*(l.0 as *const CWPSTRUCT);
            if message.hwnd.0 as isize == WINDOW.load(Ordering::Acquire) {
                // The sizing subclass is independent of tray availability.
                // Mark setup before SetWindowPos, which sends messages reentrantly.
                if !SIZED.replace(true) {
                    if !SetWindowSubclass(message.hwnd, Some(window_proc), SUBCLASS, 0).as_bool() {
                        SIZED.set(false);
                        return CallNextHookEx(None, code, w, l);
                    }
                    if let Some((width, height)) = minimum_outer_size(message.hwnd) {
                        let _ = SetWindowPos(
                            message.hwnd,
                            None,
                            0,
                            0,
                            width,
                            height,
                            SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
                        );
                    }
                }
                TRAY.with(|cell| {
                    // Tray creation sends Win32 messages synchronously and can
                    // re-enter this hook. Do not borrow the slot twice.
                    let Ok(mut slot) = cell.try_borrow_mut() else {
                        return;
                    };
                    if slot.is_none() {
                        match NativeTray::new() {
                            Ok(tray) => {
                                if SetTimer(Some(message.hwnd), TIMER, 100, None) != 0 {
                                    *slot = Some(tray);
                                }
                            }
                            Err(error) => {
                                eprintln!("Pomoterra tray unavailable; close will exit: {error}")
                            }
                        }
                    }
                });
            }
        }
        CallNextHookEx(None, code, w, l)
    }
}

/// Convert the shared logical content minimum to a decorated, physical size.
/// Query DPI for each request so moving between monitors keeps the same limits.
unsafe fn minimum_outer_size(hwnd: HWND) -> Option<(i32, i32)> {
    unsafe {
        let dpi = GetDpiForWindow(hwnd).max(96);
        let (width, height) = crate::MIN_WINDOW_SIZE;
        let mut rect = RECT {
            left: 0,
            top: 0,
            right: (f64::from(width) * f64::from(dpi) / 96.0).ceil() as i32,
            bottom: (f64::from(height) * f64::from(dpi) / 96.0).ceil() as i32,
        };
        AdjustWindowRectExForDpi(
            &mut rect,
            WINDOW_STYLE(GetWindowLongW(hwnd, GWL_STYLE) as u32),
            !GetMenu(hwnd).0.is_null(),
            WINDOW_EX_STYLE(GetWindowLongW(hwnd, GWL_EXSTYLE) as u32),
            dpi,
        )
        .ok()?;
        Some((rect.right - rect.left, rect.bottom - rect.top))
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    w: WPARAM,
    l: LPARAM,
    _: usize,
    _: usize,
) -> LRESULT {
    unsafe {
        match msg {
            WM_GETMINMAXINFO => {
                let result = DefSubclassProc(hwnd, msg, w, l);
                if let Some((width, height)) = minimum_outer_size(hwnd) {
                    let limits = &mut *(l.0 as *mut MINMAXINFO);
                    limits.ptMinTrackSize.x = limits.ptMinTrackSize.x.max(width);
                    limits.ptMinTrackSize.y = limits.ptMinTrackSize.y.max(height);
                }
                return result;
            }
            WM_CLOSE
                if super::close_to_tray()
                    && TRAY.with(|tray| tray.try_borrow().is_ok_and(|tray| tray.is_some())) =>
            {
                let _ = ShowWindow(hwnd, SW_HIDE);
                return LRESULT(0);
            }
            WM_TIMER if w.0 == TIMER => {
                if super::take_show_request() {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    let _ = SetForegroundWindow(hwnd);
                }
                let actions = TRAY.with_borrow_mut(|tray| {
                    tray.as_mut()
                        .map(|tray| {
                            tray.update();
                            tray.actions()
                        })
                        .unwrap_or_default()
                });
                for action in actions {
                    match action {
                        Action::Restore => {
                            let _ = ShowWindow(hwnd, SW_RESTORE);
                            let _ = SetForegroundWindow(hwnd);
                        }
                        Action::Quit => {
                            let _ = DestroyWindow(hwnd);
                        }
                    }
                }
                return LRESULT(0);
            }
            WM_NCDESTROY => {
                let _ = KillTimer(Some(hwnd), TIMER);
                let _ = RemoveWindowSubclass(hwnd, Some(window_proc), SUBCLASS);
                TRAY.with_borrow_mut(|tray| {
                    tray.take();
                });
                SIZED.set(false);
            }
            _ => {}
        }
        DefSubclassProc(hwnd, msg, w, l)
    }
}
