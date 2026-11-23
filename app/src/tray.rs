//! The tray icon and its menu. Clicks are handed to the window thread as actions.

use slint::ComponentHandle;
use tray_icon::menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};

use crate::{AppWindow, Ui};

pub struct Tray {
    _icon: TrayIcon,
}

impl Tray {
    /// `None` when Windows refuses the icon; the window then simply quits on close.
    pub fn new(weak: slint::Weak<AppWindow>) -> Option<Tray> {
        let open = MenuItem::new("Open", true, None);
        let add = MenuItem::new("Add a link", true, None);
        let pause = MenuItem::new("Pause all", true, None);
        let resume = MenuItem::new("Resume all", true, None);
        let quit = MenuItem::new("Quit", true, None);
        let menu = Menu::new();
        menu.append_items(&[&open, &add, &PredefinedMenuItem::separator(), &pause, &resume, &PredefinedMenuItem::separator(), &quit])
            .ok()?;

        let actions = [
            (open.id().clone(), "show"),
            (add.id().clone(), "add"),
            (pause.id().clone(), "pause-all"),
            (resume.id().clone(), "resume-all"),
            (quit.id().clone(), "quit"),
        ];
        let menu_weak = weak.clone();
        MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
            if let Some((_, name)) = actions.iter().find(|(id, _)| *id == event.id) {
                send(&menu_weak, name);
            }
        }));
        TrayIconEvent::set_event_handler(Some(move |event: TrayIconEvent| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. }
            | TrayIconEvent::DoubleClick { button: MouseButton::Left, .. } = event
            {
                send(&weak, "show");
            }
        }));

        let icon = Icon::from_resource(1, None).ok()?;
        let tray = TrayIconBuilder::new()
            .with_tooltip("Seamless Download Manager")
            .with_icon(icon)
            .with_menu(Box::new(menu))
            .with_menu_on_left_click(false)
            .build()
            .ok()?;
        Some(Tray { _icon: tray })
    }
}

fn send(weak: &slint::Weak<AppWindow>, name: &'static str) {
    let _ = weak.upgrade_in_event_loop(move |app| app.global::<Ui>().invoke_action(name.into()));
}
