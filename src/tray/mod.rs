#[cfg(target_os = "linux")]
pub mod linux;

#[cfg(any(target_os = "windows", target_os = "macos"))]
pub mod desktop;
