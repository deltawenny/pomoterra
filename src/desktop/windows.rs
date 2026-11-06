//! Win32 hooks marshal setup onto NSH's UI thread (not its engine thread).
use super::native_tray::{Action, NativeTray};
use std::{
    cell::RefCell,
    sync::{
        Once,
        atomic::{AtomicIsize, Ordering},
    },
    time::Duration,
};
use windows::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentProcessId,
    UI::{
        Shell::{DefSubclassProc, RemoveWindowSubclass, SetWindowSubclass},
        WindowsAndMessaging::*,
    },
};
use windows::core::BOOL;

static START: Once = Once::new();
static WINDOW: AtomicIsize = AtomicIsize::new(0);
thread_local! { static TRAY: RefCell<Option<NativeTray>> = const { RefCell::new(None) }; }
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
                TRAY.with(|cell| {
                    // Tray creation sends Win32 messages synchronously and can
                    // re-enter this hook. Do not borrow the slot twice.
                    let Ok(mut slot) = cell.try_borrow_mut() else {
                        return;
                    };
                    if slot.is_none() {
                        match NativeTray::new() {
                            Ok(tray) => {
                                if SetWindowSubclass(message.hwnd, Some(window_proc), SUBCLASS, 0)
                                    .as_bool()
                                {
                                    if SetTimer(Some(message.hwnd), TIMER, 100, None) != 0 {
                                        *slot = Some(tray);
                                    } else {
                                        let _ = RemoveWindowSubclass(
                                            message.hwnd,
                                            Some(window_proc),
                                            SUBCLASS,
                                        );
                                    }
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
            WM_CLOSE => {
                let _ = ShowWindow(hwnd, SW_HIDE);
                return LRESULT(0);
            }
            WM_TIMER if w.0 == TIMER => {
                let actions = TRAY.with_borrow(|tray| {
                    tray.as_ref().map(|tray| tray.actions()).unwrap_or_default()
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
            }
            _ => {}
        }
        DefSubclassProc(hwnd, msg, w, l)
    }
}
