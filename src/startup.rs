use anyhow::{anyhow, Context, Result};
use winreg::enums::*;
use winreg::RegKey;

pub fn set_startup(enabled: bool) -> Result<()> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key_path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let (run_key, _disp) = hkcu.create_subkey(run_key_path)?;

    if enabled {
        let exe_path = std::env::current_exe()?
            .to_str()
            .ok_or_else(|| anyhow!("Failed to convert exe path to string"))?
            .to_owned();
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

pub fn is_startup_enabled() -> Result<bool> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let run_key_path = r"Software\Microsoft\Windows\CurrentVersion\Run";
    let run_key = hkcu
        .open_subkey_with_flags(run_key_path, KEY_READ)
        .map_err(|e| anyhow!("Failed to open HKEY_CURRENT_USER\\...\\Run - {e}"))?;

    match run_key.get_value::<String, _>("CapsGlow") {
        Ok(value) => {
            let exe_path = std::env::current_exe()
                .context("Failed to get exe path")?
                .to_str()
                .ok_or_else(|| anyhow!("Failed to convert exe path to string"))?
                .to_owned();
            Ok(value == exe_path)
        }
        Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(anyhow::Error::new(e).context("Failed to read the autostart registry key")),
    }
}
