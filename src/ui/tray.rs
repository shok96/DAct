use std::sync::Mutex;

use tray_icon::menu::{CheckMenuItem, Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::ui::icon::tray_rgba;

pub enum TrayCmd {
    Show,
    OpenFile,
    Settings,
    Welcome,
    ToggleEnabled(bool),
    Quit,
}

pub struct Tray {
    _tray: TrayIcon,
    enabled: CheckMenuItem,
    show_id: MenuId,
    open_id: MenuId,
    settings_id: MenuId,
    welcome_id: MenuId,
    quit_id: MenuId,
    enabled_id: MenuId,
    trigger_label: Mutex<String>,
    activity: Mutex<Option<String>>,
}

impl Tray {
    pub fn new(trigger_label: &str) -> anyhow::Result<Self> {
        let menu = Menu::new();
        let show = MenuItem::new("Show DAct", true, None);
        let enabled = CheckMenuItem::new("Enabled", true, true, None);
        let open = MenuItem::new("Open a file…", true, None);
        let settings = MenuItem::new("Settings…", true, None);
        let welcome = MenuItem::new("How to use", true, None);
        let quit = MenuItem::new("Quit DAct", true, None);
        menu.append_items(&[
            &show,
            &enabled,
            &PredefinedMenuItem::separator(),
            &open,
            &settings,
            &welcome,
            &PredefinedMenuItem::separator(),
            &quit,
        ])?;

        let (rgba, w, h) = tray_rgba();
        let icon = Icon::from_rgba(rgba, w, h)?;
        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip(idle_tip(trigger_label))
            .with_icon(icon)
            .with_menu_on_left_click(true)
            .build()?;

        Ok(Self {
            show_id: show.id().clone(),
            open_id: open.id().clone(),
            settings_id: settings.id().clone(),
            welcome_id: welcome.id().clone(),
            quit_id: quit.id().clone(),
            enabled_id: enabled.id().clone(),
            _tray: tray,
            enabled,
            trigger_label: Mutex::new(trigger_label.to_string()),
            activity: Mutex::new(None),
        })
    }

    pub fn set_enabled(&self, on: bool) {
        self.enabled.set_checked(on);
    }

    pub fn set_trigger_label(&self, label: &str) {
        if let Ok(mut g) = self.trigger_label.lock() {
            *g = label.to_string();
        }
        self.refresh_tooltip();
    }

    pub fn set_activity(&self, text: Option<String>) {
        if let Ok(mut g) = self.activity.lock() {
            *g = text;
        }
        self.refresh_tooltip();
    }

    fn refresh_tooltip(&self) {
        let label = self
            .trigger_label
            .lock()
            .ok()
            .map(|g| g.clone())
            .unwrap_or_else(|| "Alt".into());
        let activity = self.activity.lock().ok().and_then(|g| g.clone());
        let tip = match activity {
            Some(a) => format!("DAct — {a}"),
            None => idle_tip(&label),
        };
        let _ = self._tray.set_tooltip(Some(tip));
    }

    pub fn poll(&self) -> Vec<TrayCmd> {
        let mut out = Vec::new();
        while let Ok(ev) = TrayIconEvent::receiver().try_recv() {
            match ev {
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
                | TrayIconEvent::DoubleClick {
                    button: MouseButton::Left,
                    ..
                } => out.push(TrayCmd::Show),
                _ => {}
            }
        }
        while let Ok(ev) = MenuEvent::receiver().try_recv() {
            let cmd = if ev.id == self.show_id {
                TrayCmd::Show
            } else if ev.id == self.open_id {
                TrayCmd::OpenFile
            } else if ev.id == self.settings_id {
                TrayCmd::Settings
            } else if ev.id == self.welcome_id {
                TrayCmd::Welcome
            } else if ev.id == self.quit_id {
                TrayCmd::Quit
            } else if ev.id == self.enabled_id {
                TrayCmd::ToggleEnabled(self.enabled.is_checked())
            } else {
                continue;
            };
            out.push(cmd);
        }
        out
    }
}

fn idle_tip(trigger_label: &str) -> String {
    format!("DAct — {trigger_label} + left click on a file")
}
