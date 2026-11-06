//! Native tray/menu objects must be created and retained on the UI thread.
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{Menu, MenuEvent, MenuItem},
};

pub(super) struct NativeTray {
    _icon: TrayIcon,
    restore: MenuItem,
    quit: MenuItem,
}

pub(super) enum Action {
    Restore,
    Quit,
}

impl NativeTray {
    pub(super) fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let menu = Menu::new();
        let restore = MenuItem::new("Restore the app", true, None);
        let quit = MenuItem::new("Quit", true, None);
        menu.append_items(&[&restore, &quit])?;
        let icon = tray_icon::Icon::from_rgba(super::icon_rgba(32)?, 32, 32)?;
        let icon = TrayIconBuilder::new()
            .with_tooltip("Pomoterra")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(true)
            .build()?;
        Ok(Self {
            _icon: icon,
            restore,
            quit,
        })
    }

    pub(super) fn actions(&self) -> Vec<Action> {
        MenuEvent::receiver()
            .try_iter()
            .filter_map(|event| {
                if event.id == *self.restore.id() {
                    Some(Action::Restore)
                } else if event.id == *self.quit.id() {
                    Some(Action::Quit)
                } else {
                    None
                }
            })
            .collect()
    }
}
