use ksni::blocking::TrayMethods;
use ksni::menu::{MenuItem, StandardItem};

use crate::constants;
use crate::icon_renderer::{IconRenderer, RenderedIcon, State};
use crate::poll::Poller;

struct AppTray {
    renderer: IconRenderer,
    percent: Option<f64>,
    state: State,
    lines: Vec<String>,
}

impl ksni::Tray for AppTray {
    // Left-click sends `Activate`, which does nothing by default; this makes
    // it open the same menu as a right-click, matching the old pystray UX
    // where any click opened the menu.
    const MENU_ON_ACTIVATE: bool = true;

    fn id(&self) -> String {
        "claude-usage-tray".to_string()
    }

    fn title(&self) -> String {
        if self.lines.is_empty() {
            constants::APP_NAME.to_string()
        } else {
            self.lines.join("\n")
        }
    }

    fn icon_pixmap(&self) -> Vec<ksni::Icon> {
        vec![rgba_to_argb_icon(
            self.renderer.render(self.percent, self.state),
        )]
    }

    fn tool_tip(&self) -> ksni::ToolTip {
        ksni::ToolTip {
            title: constants::APP_NAME.to_string(),
            description: self.lines.join("\n"),
            ..Default::default()
        }
    }

    fn menu(&self) -> Vec<MenuItem<Self>> {
        let mut items: Vec<MenuItem<Self>> = self
            .lines
            .iter()
            .map(|line| {
                MenuItem::Standard(StandardItem {
                    label: line.clone(),
                    enabled: false,
                    ..Default::default()
                })
            })
            .collect();

        items.push(MenuItem::Separator);
        items.push(MenuItem::Standard(StandardItem {
            label: "Quit".to_string(),
            activate: Box::new(|_| std::process::exit(0)),
            ..Default::default()
        }));

        items
    }
}

fn rgba_to_argb_icon(rendered: RenderedIcon) -> ksni::Icon {
    let mut data = rendered.rgba;
    for pixel in data.chunks_exact_mut(4) {
        // ksni::Icon expects ARGB32, network (big-endian) byte order; our
        // renderer produces RGBA8, so rotate each pixel's bytes right by one.
        pixel.rotate_right(1);
    }
    ksni::Icon {
        width: rendered.width as i32,
        height: rendered.height as i32,
        data,
    }
}

pub fn run() {
    let tray = AppTray {
        renderer: IconRenderer::new(),
        percent: None,
        state: State::Loading,
        lines: vec!["Loading Claude usage...".to_string()],
    };

    let handle = tray
        .spawn()
        .expect("failed to start the StatusNotifierItem service");

    let mut poller = Poller::new();
    loop {
        let (outcome, wait) = poller.poll_once();
        handle.update(move |tray: &mut AppTray| {
            tray.percent = outcome.percent;
            tray.state = outcome.state;
            tray.lines = outcome.lines;
        });
        std::thread::sleep(wait);
    }
}
