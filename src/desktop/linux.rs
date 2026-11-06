//! Linux StatusNotifier integration. GTK objects never leave the UI thread.
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc::{self, Sender},
    time::Duration,
};

use ksni::blocking::TrayMethods;
use nsh_platform_linux::gtk4::{self as gtk, glib, prelude::*};

thread_local! {
    static WINDOWS: RefCell<Vec<glib::WeakRef<gtk::ApplicationWindow>>> = const { RefCell::new(Vec::new()) };
}

enum Event {
    Ready(Result<ksni::blocking::Handle<Tray>, String>),
    Restore,
    Quit,
    Online(bool),
}

struct Tray {
    events: Sender<Event>,
    icons: Vec<ksni::Icon>,
}

impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "com.example.pomoterra".into()
    }

    fn title(&self) -> String {
        "Pomoterra".into()
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        self.icons.clone()
    }

    fn activate(&mut self, _: i32, _: i32) {
        let _ = self.events.send(Event::Restore);
    }

    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        vec![
            StandardItem {
                label: "Restore the app".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.events.send(Event::Restore);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: "Quit".into(),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.events.send(Event::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]
    }

    fn watcher_online(&self) {
        let _ = self.events.send(Event::Online(true));
    }

    fn watcher_offline(&self, _: ksni::OfflineReason) -> bool {
        let _ = self.events.send(Event::Online(false));
        true
    }
}

/// Render the same bundled Lucide catalog used by the UI into SNI's ARGB pixels.
fn tray_icons() -> Result<Vec<ksni::Icon>, String> {
    [24, 32, 48]
        .into_iter()
        .map(|size| {
            let mut data = super::icon_rgba(size)?;
            for pixel in data.chunks_exact_mut(4) {
                pixel.rotate_right(1); // SNI uses network-order ARGB, not RGBA.
            }
            Ok(ksni::Icon {
                width: size as i32,
                height: size as i32,
                data,
            })
        })
        .collect()
}

/// Install once per native window, including windows created by NSH runners.
/// Until a tray host confirms registration, closing retains normal GTK behavior.
pub(crate) fn install(window: &gtk::ApplicationWindow) {
    let already_installed = WINDOWS.with_borrow_mut(|windows| {
        windows.retain(|weak| weak.upgrade().is_some());
        if windows
            .iter()
            .any(|weak| weak.upgrade().as_ref() == Some(window))
        {
            return true;
        }
        windows.push(window.downgrade());
        false
    });
    if already_installed {
        return;
    }
    let Some(app) = window.application() else {
        return;
    };

    let available = Rc::new(Cell::new(false));
    let can_hide = available.clone();
    window.connect_close_request(move |window| {
        if can_hide.get() {
            window.set_visible(false);
            glib::Propagation::Stop
        } else {
            glib::Propagation::Proceed
        }
    });

    // Replace NSH's quit action (which otherwise closes and would hide instead).
    let quit = gtk::gio::SimpleAction::new("quit", None);
    let weak_app = app.downgrade();
    quit.connect_activate(move |_, _| {
        if let Some(app) = weak_app.upgrade() {
            app.quit();
        }
    });
    app.add_action(&quit);
    app.set_accels_for_action("app.quit", &["<Control>q"]);

    let (events, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = tray_icons().and_then(|icons| {
            Tray {
                events: events.clone(),
                icons,
            }
            .spawn()
            .map_err(|error| error.to_string())
        });
        let _ = events.send(Event::Ready(result));
    });

    let weak_window = window.downgrade();
    let mut handle: Option<ksni::blocking::Handle<Tray>> = None;
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let Some(window) = weak_window.upgrade() else {
            if let Some(handle) = handle.take() {
                handle.shutdown();
            }
            return glib::ControlFlow::Break;
        };
        for event in receiver.try_iter() {
            match event {
                Event::Ready(Ok(tray)) => {
                    available.set(true);
                    handle = Some(tray);
                }
                Event::Ready(Err(error)) => {
                    eprintln!("Pomoterra tray unavailable; close will exit: {error}");
                    return glib::ControlFlow::Break;
                }
                Event::Restore => window.present(),
                Event::Quit => {
                    if let Some(app) = window.application() {
                        app.quit();
                    }
                }
                Event::Online(online) => {
                    available.set(online);
                    if !online {
                        window.present();
                    }
                }
            }
        }
        if handle.as_ref().is_some_and(|tray| tray.is_closed()) {
            available.set(false);
            window.present();
            return glib::ControlFlow::Break;
        }
        glib::ControlFlow::Continue
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a display and a session D-Bus StatusNotifier host"]
    fn native_window_hides_restores_and_quits() {
        let app = gtk::Application::builder()
            .application_id("com.example.pomoterra.TrayTest")
            .flags(gtk::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        let checked = Rc::new(Cell::new(false));
        let result = checked.clone();
        app.connect_activate(move |app| {
            let window = gtk::ApplicationWindow::builder().application(app).build();
            window.present();
            install(&window);
            install(&window); // Repeated component setup must not add another tray.
            let app = app.clone();
            let result = result.clone();
            glib::timeout_add_local_once(Duration::from_secs(3), move || {
                window.close();
                assert!(!window.is_visible());
                assert!(
                    window.application().is_some(),
                    "window must not be destroyed"
                );
                window.present();
                assert!(window.is_visible());
                result.set(true);
                app.activate_action("quit", None);
            });
        });
        assert_eq!(app.run_with_args::<&str>(&[]), glib::ExitCode::SUCCESS);
        assert!(checked.get());
    }

    #[test]
    fn tray_icons_have_visible_argb_pixels_at_each_size() {
        let icons = tray_icons().unwrap();
        assert_eq!(icons.len(), 3);
        for icon in icons {
            assert_eq!(icon.data.len(), (icon.width * icon.height * 4) as usize);
            assert!(icon.data.chunks_exact(4).any(|pixel| pixel[0] > 0));
        }
    }

    #[test]
    fn tray_activation_and_menu_send_ui_events() {
        use ksni::Tray as _;
        let (events, receiver) = mpsc::channel();
        let mut tray = Tray {
            events,
            icons: vec![],
        };
        tray.activate(0, 0);
        assert!(matches!(receiver.recv().unwrap(), Event::Restore));
        let menu = tray.menu();
        assert_eq!(menu.len(), 2);
        for (item, label) in menu.into_iter().zip(["Restore the app", "Quit"]) {
            let ksni::MenuItem::Standard(item) = item else {
                panic!("expected a standard menu item");
            };
            assert_eq!(item.label, label);
            (item.activate)(&mut tray);
        }
        assert!(matches!(receiver.recv().unwrap(), Event::Restore));
        assert!(matches!(receiver.recv().unwrap(), Event::Quit));
        assert!(tray.watcher_offline(ksni::OfflineReason::No));
        assert!(matches!(receiver.recv().unwrap(), Event::Online(false)));
        tray.watcher_online();
        assert!(matches!(receiver.recv().unwrap(), Event::Online(true)));
    }
}
