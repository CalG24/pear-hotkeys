use tauri_winrt_notification::Toast;
use winreg::enums::*;
use winreg::RegKey;
use windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID;

pub const AUMID: &str = "PearDesktop.Hotkeys";

/// Registers a proper display name/icon so toasts show "Pear Desktop"
/// instead of borrowing another app's identity. Call once at startup.
pub fn register_aumid() -> anyhow::Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(format!(r"Software\Classes\AppUserModelId\{AUMID}"))?;
    key.set_value("DisplayName", &"Pear Desktop")?;

    let exe_path = std::env::current_exe()?;
    let icon_path = exe_path.with_file_name("pear-icon.ico");
    if !icon_path.exists() {
        std::fs::write(&icon_path, include_bytes!("../assets/icon.ico"))?;
    }
    key.set_value("IconUri", &icon_path.to_string_lossy().to_string())?;
    Ok(())
}

pub fn set_process_aumid() {
    let wide: Vec<u16> = AUMID.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe { SetCurrentProcessExplicitAppUserModelID(wide.as_ptr()); }
}

pub fn show(title: &str, body: &str) {
    if let Err(e) = Toast::new(AUMID).title(title).text1(body).show() {
        tracing::warn!("failed to show toast notification: {e:?}");
    }
}
