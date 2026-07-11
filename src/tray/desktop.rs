//! Windows/macOS tray backend.
//!
//! Unlike the Linux `ksni` backend, native Win32/Cocoa tray objects must only
//! be touched from the thread that owns the platform event loop, so the
//! polling logic runs on a background thread and hands results to the main
//! thread via an `EventLoopProxy`, mirroring the official tray-icon + tao +
//! muda example.

use muda::{Menu, MenuEvent, MenuId, MenuItem, PredefinedMenuItem};
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoop};
use tray_icon::{Icon, TrayIconBuilder, TrayIconEvent};

use crate::constants;
use crate::icon_renderer::{IconRenderer, State};
use crate::poll::{PollOutcome, Poller};

enum UserEvent {
    Tray(TrayIconEvent),
    Menu(MenuEvent),
    Update(PollOutcome),
}

fn build_icon(renderer: &IconRenderer, percent: Option<f64>, state: State) -> Icon {
    let rendered = renderer.render(percent, state);
    Icon::from_rgba(rendered.rgba, rendered.width, rendered.height)
        .expect("rendered icon buffer must be a valid RGBA image")
}

fn build_menu(lines: &[String], quit_id: &MenuId) -> Menu {
    let menu = Menu::new();
    for line in lines {
        menu.append(&MenuItem::new(line, false, None))
            .expect("failed to append a usage line to the tray menu");
    }
    menu.append(&PredefinedMenuItem::separator())
        .expect("failed to append the menu separator");
    menu.append(&MenuItem::with_id(quit_id.clone(), "Quit", true, None))
        .expect("failed to append the Quit menu item");
    menu
}

pub fn run() {
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("failed to create the platform event loop");
    let proxy = event_loop.create_proxy();

    let tray_proxy = proxy.clone();
    TrayIconEvent::set_event_handler(Some(move |event| {
        let _ = tray_proxy.send_event(UserEvent::Tray(event));
    }));

    let menu_proxy = proxy.clone();
    MenuEvent::set_event_handler(Some(move |event| {
        let _ = menu_proxy.send_event(UserEvent::Menu(event));
    }));

    let quit_id = MenuId::new("quit");
    let initial_lines = vec!["Loading Claude usage...".to_string()];
    let renderer = IconRenderer::new();

    let mut tray_icon = TrayIconBuilder::new()
        .with_icon(build_icon(&renderer, None, State::Loading))
        .with_tooltip(constants::APP_NAME)
        .with_menu(Box::new(build_menu(&initial_lines, &quit_id)))
        .build()
        .expect("failed to create the tray icon");

    let poll_proxy = proxy.clone();
    std::thread::spawn(move || {
        let mut poller = Poller::new();
        loop {
            let (outcome, wait) = poller.poll_once();
            if poll_proxy.send_event(UserEvent::Update(outcome)).is_err() {
                return;
            }
            std::thread::sleep(wait);
        }
    });

    event_loop.run(move |event, _target, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::UserEvent(UserEvent::Update(outcome)) => {
                let _ = tray_icon.set_icon(Some(build_icon(
                    &renderer,
                    outcome.percent,
                    outcome.state,
                )));
                let _ = tray_icon.set_tooltip(Some(outcome.lines.join("\n")));
                tray_icon.set_menu(Some(Box::new(build_menu(&outcome.lines, &quit_id))));
            }
            Event::UserEvent(UserEvent::Menu(menu_event)) => {
                if menu_event.id == quit_id {
                    *control_flow = ControlFlow::Exit;
                }
            }
            _ => {}
        }
    });
}
