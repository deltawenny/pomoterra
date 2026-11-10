//! Native tray/menu objects must be created and retained on the UI thread.
use super::{
    tray_image::{TrayState, icon_rgba},
    tray_menu::TrayMenu,
};
use tray_icon::{
    TrayIcon, TrayIconBuilder,
    menu::{CheckMenuItem, Menu, MenuEvent, MenuItem, PredefinedMenuItem, Submenu},
};

#[cfg(target_os = "windows")]
fn system_dark() -> bool {
    use windows::{
        Win32::System::Registry::{HKEY_CURRENT_USER, RRF_RT_REG_DWORD, RegGetValueW},
        core::w,
    };
    let mut light = 1_u32;
    let mut size = std::mem::size_of_val(&light) as u32;
    let result = unsafe {
        RegGetValueW(
            HKEY_CURRENT_USER,
            w!("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize"),
            w!("SystemUsesLightTheme"),
            RRF_RT_REG_DWORD,
            None,
            Some((&mut light as *mut u32).cast()),
            Some(&mut size),
        )
    };
    result.is_ok() && light == 0
}

#[cfg(target_os = "macos")]
fn system_dark() -> bool {
    use nsh_platform_macos::{
        objc2::{MainThreadMarker, msg_send},
        objc2_app_kit::NSApplication,
        objc2_foundation::NSString,
    };
    let Some(mtm) = MainThreadMarker::new() else {
        return false;
    };
    let app = NSApplication::sharedApplication(mtm);
    // Query effective appearance on the AppKit thread, including live changes.
    let appearance: nsh_platform_macos::objc2::rc::Retained<
        nsh_platform_macos::objc2::runtime::AnyObject,
    > = unsafe { msg_send![&*app, effectiveAppearance] };
    let name: nsh_platform_macos::objc2::rc::Retained<NSString> =
        unsafe { msg_send![&*appearance, name] };
    name.to_string().contains("Dark")
}

pub(super) struct NativeTray {
    icon: TrayIcon,
    state: TrayState,
    restore: MenuItem,
    quit: MenuItem,
    playback: MenuItem,
    profile: MenuItem,
    phase: MenuItem,
    time: MenuItem,
    profiles: Submenu,
    choices: Vec<(i64, CheckMenuItem)>,
    rendered_menu: TrayMenu,
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
        let rendered_menu = TrayMenu::current();
        let playback = MenuItem::new(rendered_menu.playback_label(), true, None);
        let profile = MenuItem::new(rendered_menu.profile.replace('&', "&&"), false, None);
        let phase = MenuItem::new(&rendered_menu.phase, false, None);
        let time = MenuItem::new(&rendered_menu.time, false, None);
        let profiles = Submenu::new("Switch profile", true);
        let choices = Self::populate_profiles(&profiles, &rendered_menu)?;
        menu.append_items(&[
            &profile,
            &phase,
            &time,
            &PredefinedMenuItem::separator(),
            &playback,
            &profiles,
            &PredefinedMenuItem::separator(),
            &restore,
            &quit,
        ])?;
        let state = TrayState::current().with_dark(system_dark());
        let icon = tray_icon::Icon::from_rgba(icon_rgba(32, state)?, 32, 32)?;
        let icon = TrayIconBuilder::new()
            .with_tooltip("Pomoterra")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(true)
            .build()?;
        Ok(Self {
            icon,
            state,
            restore,
            quit,
            playback,
            profile,
            phase,
            time,
            profiles,
            choices,
            rendered_menu,
        })
    }

    fn populate_profiles(
        submenu: &Submenu,
        menu: &TrayMenu,
    ) -> Result<Vec<(i64, CheckMenuItem)>, Box<dyn std::error::Error>> {
        while submenu.remove_at(0).is_some() {}
        let mut choices = Vec::new();
        for profile in &menu.profiles {
            let item = CheckMenuItem::new(
                profile.name.replace('&', "&&"),
                true,
                profile.selected,
                None,
            );
            submenu.append(&item)?;
            choices.push((profile.id, item));
        }
        if choices.is_empty() {
            submenu.append(&MenuItem::new(
                "Choose quick profiles in Settings",
                false,
                None,
            ))?;
        }
        Ok(choices)
    }

    pub(super) fn update(&mut self) {
        let menu = TrayMenu::current();
        if menu != self.rendered_menu {
            self.profile.set_text(menu.profile.replace('&', "&&"));
            self.phase.set_text(&menu.phase);
            self.time.set_text(&menu.time);
            self.playback.set_text(menu.playback_label());
            if menu.profiles != self.rendered_menu.profiles {
                match Self::populate_profiles(&self.profiles, &menu) {
                    Ok(choices) => self.choices = choices,
                    Err(error) => {
                        eprintln!("Pomoterra tray profile update failed: {error}");
                        return;
                    }
                }
            }
            self.rendered_menu = menu;
        }
        let state = TrayState::current().with_dark(system_dark());
        if state == self.state {
            return;
        }
        let result = icon_rgba(32, state)
            .map_err(|error| error.to_string())
            .and_then(|pixels| {
                tray_icon::Icon::from_rgba(pixels, 32, 32).map_err(|error| error.to_string())
            })
            .and_then(|icon| {
                self.icon
                    .set_icon(Some(icon))
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(()) => self.state = state,
            Err(error) => eprintln!("Pomoterra tray icon update failed: {error}"),
        }
    }

    pub(super) fn actions(&self) -> Vec<Action> {
        MenuEvent::receiver()
            .try_iter()
            .filter_map(|event| {
                if event.id == *self.restore.id() {
                    Some(Action::Restore)
                } else if event.id == *self.playback.id() {
                    super::tray_menu::request_playback(!self.rendered_menu.running);
                    None
                } else if event.id == *self.quit.id() {
                    Some(Action::Quit)
                } else {
                    if let Some((id, _)) =
                        self.choices.iter().find(|(_, item)| event.id == *item.id())
                    {
                        super::tray_menu::request_profile(*id);
                        // Native check items toggle eagerly; the app snapshot is authoritative.
                        for (id, item) in &self.choices {
                            item.set_checked(
                                self.rendered_menu
                                    .profiles
                                    .iter()
                                    .any(|p| p.id == *id && p.selected),
                            );
                        }
                    }
                    None
                }
            })
            .collect()
    }
}
