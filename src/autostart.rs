use anyhow::Result;
use winreg::enums::*;
use winreg::RegKey;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APP_NAME: &str = "PearDesktopHotkeys";

pub fn set_autostart(enabled: bool) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run, _) = hkcu.create_subkey(RUN_KEY)?;
    if enabled {
        let exe_path = std::env::current_exe()?;
        run.set_value(APP_NAME, &format!("\"{}\"", exe_path.display()))?;
    } else {
        let _ = run.delete_value(APP_NAME);
    }
    Ok(())
}
