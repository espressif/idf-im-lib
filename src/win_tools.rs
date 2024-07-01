use log::error;
use std::ptr;
use winapi::shared::minwindef::*;
use winapi::um::winuser::{
    SendMessageTimeoutA, HWND_BROADCAST, SMTO_ABORTIFHUNG, WM_SETTINGCHANGE,
};
use winreg::{
    enums::{HKEY_CURRENT_USER, KEY_READ, KEY_WRITE},
    RegKey,
};

pub fn set_env_variable(key: &str, value: &str) -> Result<(), String> {
    if std::env::consts::OS == "windows" {
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let environment_key = hkcu
            .open_subkey_with_flags("Environment", KEY_WRITE)
            .map_err(|_| "Error opening environment registry key")?;
        environment_key
            .set_value(key, &value)
            .map_err(|_| "Error setting environment variable to registry")?;

        // Tell other processes to update their environment
        #[allow(clippy::unnecessary_cast)]
        unsafe {
            SendMessageTimeoutA(
                HWND_BROADCAST,
                WM_SETTINGCHANGE,
                0 as WPARAM,
                "Environment\0".as_ptr() as LPARAM,
                SMTO_ABORTIFHUNG,
                5000,
                ptr::null_mut(),
            );
        }

        Ok(())
    } else {
        error!("set_env_variable is win dows platform specific. Skipping setting environment variables.");
        Err("set_env_variable is win dows platform specific. Skipping setting environment variables.".to_string())
    }
}

// Get the windows PATH variable out of the registry as a String.
pub fn get_windows_path_var() -> Result<String, String> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let env = hkcu
        .open_subkey("Environment")
        .map_err(|_| "Error opening environment registry key")?;
    let path: String = env
        .get_value("Path")
        .map_err(|_| "Error getting PATH variable")
        .unwrap();
    Ok(path)
}

pub fn add_to_win_path(directory_path: &str) -> Result<(), String> {
    let mut path = match get_windows_path_var() {
        Ok(path) => path,
        Err(err) => {
            error!("Error getting Windows PATH variable: {}", err);
            return Err("Error getting Windows PATH variable: {}".to_string());
        }
    };
    if path.contains(format!("{};", directory_path).as_str()) {
        return Ok(());
    } else {
        path = format!("{};{}", path, directory_path);
    }
    if !path.ends_with(';') {
        path.push(';');
    }
    set_env_variable("PATH", &path).map_err(|_| "Error setting PATH variable in registry")?;
    Ok(())
}
