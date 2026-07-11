mod api_client;
mod constants;
mod icon_renderer;
mod poll;
mod tray;
mod usage;

fn main() {
    #[cfg(target_os = "linux")]
    tray::linux::run();

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    tray::desktop::run();
}
