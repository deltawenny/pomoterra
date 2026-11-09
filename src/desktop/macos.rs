//! AppKit status item and close-to-background behavior, always on the main queue.
use super::native_tray::{Action, NativeTray};
use nsh_platform_macos::{
    objc2::{
        MainThreadMarker, MainThreadOnly, define_class, msg_send, rc::Retained,
        runtime::ProtocolObject,
    },
    objc2_app_kit::{NSApplication, NSWindow, NSWindowDelegate},
    objc2_foundation::{NSObject, NSObjectProtocol, NSSize},
};
use std::{
    cell::{Cell, RefCell},
    sync::Once,
    time::Duration,
};

static START: Once = Once::new();
thread_local! {
    static STATE: RefCell<Option<State>> = const { RefCell::new(None) };
    static SIZED: Cell<bool> = const { Cell::new(false) };
}

struct State {
    tray: NativeTray,
    window: Retained<NSWindow>,
    _delegate: Retained<WindowDelegate>,
    // AppKit's delegate property is weak. Preserve the original delegate, too.
    _original: Option<Retained<ProtocolObject<dyn NSWindowDelegate>>>,
}

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "PomoterraBackgroundWindowDelegate"]
    struct WindowDelegate;

    unsafe impl NSObjectProtocol for WindowDelegate {}
    unsafe impl NSWindowDelegate for WindowDelegate {
        #[unsafe(method(windowShouldClose:))]
        fn should_close(&self, window: &NSWindow) -> bool {
            window.orderOut(None);
            false
        }
    }
);

pub(crate) fn install() {
    START.call_once(|| {
        // The Rust root runs on NSH's engine thread. Every AppKit interaction,
        // including creation and destruction of the status item, stays on main.
        std::thread::spawn(|| {
            loop {
                dispatch2::DispatchQueue::main().exec_async(poll);
                std::thread::sleep(Duration::from_millis(100));
            }
        });
    });
}

fn poll() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let app = NSApplication::sharedApplication(mtm);
    STATE.with_borrow_mut(|slot| {
        if slot.is_none() {
            let Some(window) = app.windows().iter().find(|window| window.isVisible()) else {
                return;
            };
            // Sizing must still work when no status item can be created.
            if !SIZED.replace(true) {
                let (width, height) = crate::MIN_WINDOW_SIZE;
                let size = NSSize::new(f64::from(width), f64::from(height));
                window.setContentMinSize(size);
                window.setContentSize(size);
            }
            match NativeTray::new() {
                Ok(tray) => {
                    let delegate: Retained<WindowDelegate> =
                        unsafe { msg_send![WindowDelegate::alloc(mtm), init] };
                    let original = window.delegate();
                    window.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
                    *slot = Some(State {
                        tray,
                        window,
                        _delegate: delegate,
                        _original: original,
                    });
                }
                Err(error) => {
                    eprintln!("Pomoterra tray unavailable; close will exit: {error}");
                    return;
                }
            }
        }
        let state = slot.as_mut().unwrap();
        state.tray.update();
        for action in state.tray.actions() {
            match action {
                Action::Restore => {
                    if state.window.isMiniaturized() {
                        state.window.deminiaturize(None);
                    }
                    state.window.makeKeyAndOrderFront(None);
                    #[allow(deprecated)]
                    app.activateIgnoringOtherApps(true);
                }
                Action::Quit => {
                    // NSH stops its engine and exits its event loop cleanly.
                    nsh_platform_macos::quit_app();
                }
            }
        }
    });
}
