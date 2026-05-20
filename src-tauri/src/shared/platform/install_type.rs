//! Detect how the running app was installed.
//!
//! Affects whether the in-app updater can self-update:
//! - macOS / Windows / AppImage: yes, Tauri's updater can replace the binary.
//! - DEB / RPM: no, those are owned by the system package manager — updater
//!   would either need sudo or break the package's contents. We hide the
//!   "Check for updates" UI for these installs.
//! - Dev / unknown: the updater is fine but rarely meaningful in dev.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum InstallType {
    /// macOS .app bundle (DMG installed). Updater works.
    Macos,
    /// Windows NSIS or MSI install. Updater works.
    Windows,
    /// Linux AppImage. Updater works.
    AppImage,
    /// Linux DEB or RPM package install. Updater should be disabled.
    LinuxPackage,
    /// Running from `cargo run` or otherwise unidentified.
    Dev,
}

impl InstallType {
    /// True when Tauri's self-update flow is meaningful and safe to call.
    pub fn supports_self_update(&self) -> bool {
        matches!(self, Self::Macos | Self::Windows | Self::AppImage)
    }
}

#[cfg(target_os = "macos")]
fn detect() -> InstallType {
    InstallType::Macos
}

#[cfg(target_os = "windows")]
fn detect() -> InstallType {
    InstallType::Windows
}

#[cfg(target_os = "linux")]
fn detect() -> InstallType {
    // The AppImage runtime sets $APPIMAGE to the path of the running .AppImage.
    // No other install type sets this.
    if std::env::var("APPIMAGE").is_ok() {
        return InstallType::AppImage;
    }

    // DEB and RPM both install to /usr/bin/ (or /opt/ for some packagers).
    // Anything else is most likely a `cargo run` / dev build.
    if let Ok(exe) = std::env::current_exe() {
        let path = exe.to_string_lossy();
        if path.starts_with("/usr/bin/") || path.starts_with("/usr/local/bin/")
            || path.starts_with("/opt/")
        {
            return InstallType::LinuxPackage;
        }
    }

    InstallType::Dev
}

/// Tauri command — returns the runtime install type as a kebab-case string
/// (e.g. "macos", "windows", "app-image", "linux-package", "dev").
#[tauri::command]
pub fn get_install_type() -> InstallType {
    detect()
}

/// Tauri command — convenience flag for the updater UI.
#[tauri::command]
pub fn supports_self_update() -> bool {
    detect().supports_self_update()
}
