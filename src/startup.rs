use anyhow::{Context, Result, anyhow};
use winreg::RegKey;
use winreg::enums::*;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";

fn get_exe_path() -> Result<String> {
    let exe_path = std::env::current_exe()?
        .to_str()
        .ok_or_else(|| anyhow!("Failed to convert exe path to string"))?
        .to_owned();
    Ok(exe_path)
}

pub fn set_startup(enabled: bool) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let (run_key, _disp) = hkcu.create_subkey(RUN_KEY)?;

    if enabled {
        let exe_path = get_exe_path()?;
        run_key
            .set_value("CapsGlow", &exe_path)
            .context("Failed to set the autostart registry key")?;
    } else {
        run_key
            .delete_value("CapsGlow")
            .context("Failed to delete the autostart registry key")?;
    }

    Ok(())
}

pub fn get_startup_status() -> Result<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key = hkcu
        .open_subkey_with_flags(RUN_KEY, KEY_READ)
        .map_err(|e| anyhow!("Failed to open HKEY_CURRENT_USER\\...\\Run - {e}"))?;

    match run_key.get_value::<String, _>("CapsGlow") {
        Ok(value) => {
            let exe_path = get_exe_path()?;
            Ok(value == exe_path)
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(anyhow::Error::new(e).context("Failed to read the autostart registry key")),
    }
}
