//! Linux StatusNotifier integration. GTK objects never leave the UI thread.
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::mpsc::{self, Sender},
    time::Duration,
};

use super::{tray_image::TrayState, tray_menu::TrayMenu};
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
    menu: TrayMenu,
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
        use ksni::menu::{CheckmarkItem, StandardItem, SubMenu};
        let mut profiles: Vec<ksni::MenuItem<Self>> = self
            .menu
            .profiles
            .iter()
            .map(|profile| {
                let id = profile.id;
                CheckmarkItem {
                    label: profile.name.replace('_', "__"),
                    checked: profile.selected,
                    activate: Box::new(move |_: &mut Self| super::tray_menu::request_profile(id)),
                    ..Default::default()
                }
                .into()
            })
            .collect();
        if profiles.is_empty() {
            profiles.push(
                StandardItem {
                    label: self.menu.translate("Choose quick profiles in Settings"),
                    enabled: false,
                    ..Default::default()
                }
                .into(),
            );
        }
        let mut menu: Vec<ksni::MenuItem<Self>> =
            [&self.menu.profile, &self.menu.phase, &self.menu.time]
                .into_iter()
                .map(|label| {
                    StandardItem {
                        label: label.replace('_', "__"),
                        enabled: false,
                        ..Default::default()
                    }
                    .into()
                })
                .collect();
        menu.push(ksni::MenuItem::Separator);
        let running = !self.menu.running;
        menu.push(
            StandardItem {
                label: self.menu.playback_label().into(),
                activate: Box::new(move |_: &mut Self| super::tray_menu::request_playback(running)),
                ..Default::default()
            }
            .into(),
        );
        menu.push(
            SubMenu {
                label: self.menu.translate("Switch profile"),
                submenu: profiles,
                ..Default::default()
            }
            .into(),
        );
        menu.push(ksni::MenuItem::Separator);
        menu.extend(vec![
            StandardItem {
                label: self.menu.translate("Restore the app"),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.events.send(Event::Restore);
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: self.menu.translate("Quit"),
                activate: Box::new(|tray: &mut Self| {
                    let _ = tray.events.send(Event::Quit);
                }),
                ..Default::default()
            }
            .into(),
        ]);
        menu
    }

    fn watcher_online(&self) {
        let _ = self.events.send(Event::Online(true));
    }

    fn watcher_offline(&self, _: ksni::OfflineReason) -> bool {
        let _ = self.events.send(Event::Online(false));
        true
    }
}

/// Convert the timer glyph into SNI's ARGB pixels.
fn tray_icons(state: TrayState) -> Result<Vec<ksni::Icon>, String> {
    [24, 32, 48]
        .into_iter()
        .map(|size| {
            let mut data = super::tray_image::icon_rgba(size, state)?;
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

/// Make the app icon available without requiring an installed icon theme entry.
fn install_app_icon(window: &gtk::ApplicationWindow) {
    static REGISTER: std::sync::Once = std::sync::Once::new();
    REGISTER.call_once(|| {
        gtk::gio::resources_register_include!("pomoterra.gresource")
            .expect("register bundled Pomoterra icon");
    });
    let theme = gtk::IconTheme::for_display(&gtk::prelude::WidgetExt::display(window));
    let path = "/com/deltawenny/pomoterra/icons";
    if !theme
        .resource_path()
        .iter()
        .any(|existing| existing == path)
    {
        theme.add_resource_path(path);
    }
    window.set_icon_name(Some("com.deltawenny.pomoterra"));
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
    install_app_icon(window);
    // The app paints an opaque rectangular surface. Theme-provided window
    // rounding otherwise clips its extreme corners and exposes black pixels
    // on X11 captures and at the content/title-bar seam on some desktops.
    // Leave the window manager's outer decorations and all UI controls alone.
    window.add_css_class("pomoterra-window");
    let provider = gtk::CssProvider::new();
    provider.load_from_string(
        "window.pomoterra-window, window.pomoterra-window > decoration {\n\
         border-radius: 0; border: none; box-shadow: none; outline: none;\n\
         background-color: #ffffff; }",
    );
    gtk::style_context_add_provider_for_display(
        &gtk::prelude::WidgetExt::display(window),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_USER + 1,
    );

    let (width, height) = crate::MIN_WINDOW_SIZE;
    window.set_size_request(width, height);
    window.set_default_size(width, height);

    let Some(app) = window.application() else {
        return;
    };

    let available = Rc::new(Cell::new(false));
    let can_hide = available.clone();
    window.connect_close_request(move |window| {
        if can_hide.get() && super::close_to_tray() {
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

    // The appearance portal follows the desktop's system preference on GNOME,
    // KDE and other portal-enabled desktops, independently of our white UI.
    let appearance = std::sync::Arc::new(std::sync::atomic::AtomicU8::new(0));
    let portal_appearance = appearance.clone();
    std::thread::spawn(move || {
        if let Ok(bus) =
            gtk::gio::bus_get_sync(gtk::gio::BusType::Session, gtk::gio::Cancellable::NONE)
        {
            loop {
                let result = bus.call_sync(
                    Some("org.freedesktop.portal.Desktop"),
                    "/org/freedesktop/portal/desktop",
                    "org.freedesktop.portal.Settings",
                    "Read",
                    Some(&("org.freedesktop.appearance", "color-scheme").to_variant()),
                    None,
                    gtk::gio::DBusCallFlags::NONE,
                    1000,
                    gtk::gio::Cancellable::NONE,
                );
                let preference = result
                    .ok()
                    .and_then(|reply| reply.child_value(0).as_variant())
                    .and_then(|value| value.get::<u32>());
                portal_appearance.store(
                    match preference {
                        Some(1) => 2,
                        Some(2) => 1,
                        _ => 0,
                    },
                    std::sync::atomic::Ordering::Relaxed,
                );
                std::thread::sleep(Duration::from_secs(2));
            }
        }
    });

    let (events, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let result = tray_icons(TrayState::current()).and_then(|icons| {
            Tray {
                events: events.clone(),
                icons,
                menu: TrayMenu::current(),
            }
            .spawn()
            .map_err(|error| error.to_string())
        });
        let _ = events.send(Event::Ready(result));
    });

    let weak_window = window.downgrade();
    let mut handle: Option<ksni::blocking::Handle<Tray>> = None;
    let mut rendered_state = None;
    let mut rendered_menu = None;
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let Some(window) = weak_window.upgrade() else {
            if let Some(handle) = handle.take() {
                handle.shutdown();
            }
            return glib::ControlFlow::Break;
        };
        if super::take_show_request() {
            window.present();
        }
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
        if let Some(tray) = &handle {
            let menu = TrayMenu::current();
            if rendered_menu.as_ref() != Some(&menu) {
                rendered_menu = Some(menu.clone());
                tray.update(move |tray| tray.menu = menu);
            }
            let dark = match appearance.load(std::sync::atomic::Ordering::Relaxed) {
                2 => true,
                1 => false,
                _ => gtk::Settings::default().is_some_and(|settings| {
                    settings.is_gtk_application_prefer_dark_theme()
                        || settings
                            .gtk_theme_name()
                            .is_some_and(|name| name.to_lowercase().contains("dark"))
                }),
            };
            let state = TrayState::current().with_dark(dark);
            if rendered_state != Some(state) {
                if let Ok(icons) = tray_icons(state) {
                    tray.update(move |tray| tray.icons = icons);
                    rendered_state = Some(state);
                }
            }
        }
        glib::ControlFlow::Continue
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a display; run with xvfb-run"]
    fn bundled_app_icon_is_available_without_installation() {
        gtk::init().expect("initialize GTK");
        let window = gtk::ApplicationWindow::builder().build();
        install_app_icon(&window);
        install_app_icon(&window);
        assert_eq!(
            window.icon_name().as_deref(),
            Some("com.deltawenny.pomoterra")
        );
        let theme = gtk::IconTheme::for_display(&gtk::prelude::WidgetExt::display(&window));
        assert!(theme.has_icon("com.deltawenny.pomoterra"));
        let bytes = gtk::gio::resources_lookup_data(
            "/com/deltawenny/pomoterra/icons/256x256/apps/com.deltawenny.pomoterra.png",
            gtk::gio::ResourceLookupFlags::NONE,
        )
        .expect("bundled logo exists");
        assert_eq!(
            bytes.as_ref(),
            include_bytes!("../../assets/icons/pomoterra.png")
        );
        window.close();
    }

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
        let icons = tray_icons(TrayState::current()).unwrap();
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
            menu: TrayMenu::default(),
        };
        tray.activate(0, 0);
        assert!(matches!(receiver.recv().unwrap(), Event::Restore));
        let menu = tray.menu();
        assert_eq!(menu.len(), 9);
        let ksni::MenuItem::Standard(playback) = &menu[4] else {
            panic!("expected playback action");
        };
        assert_eq!(playback.label, "Play");
        assert!(playback.enabled);
        (playback.activate)(&mut tray);
        assert_eq!(super::super::tray_menu::take_playback_request(), Some(true));
        tray.menu.running = true;
        let running_menu = tray.menu();
        let ksni::MenuItem::Standard(playback) = &running_menu[4] else {
            panic!("expected playback action");
        };
        assert_eq!(playback.label, "Stop");
        (playback.activate)(&mut tray);
        assert_eq!(
            super::super::tray_menu::take_playback_request(),
            Some(false)
        );
        for item in &menu[..3] {
            let ksni::MenuItem::Standard(item) = item else {
                panic!("expected status item");
            };
            assert!(!item.enabled);
        }
        let ksni::MenuItem::SubMenu(profiles) = &menu[5] else {
            panic!("expected profile submenu");
        };
        assert_eq!(profiles.label, "Switch profile");
        assert_eq!(profiles.submenu.len(), 1);
        for (item, label) in menu.into_iter().skip(7).zip(["Restore the app", "Quit"]) {
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
