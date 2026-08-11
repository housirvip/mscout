mod traits;
pub use traits::*;

#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "macos")]
pub type NativeProcess = macos::MacOsProcess;

#[cfg(target_os = "windows")]
mod windows;
#[cfg(target_os = "windows")]
pub type NativeProcess = self::windows::WindowsProcess;

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "linux")]
pub type NativeProcess = linux::LinuxProcess;
