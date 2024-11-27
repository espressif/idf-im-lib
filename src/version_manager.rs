use anyhow::anyhow;
use anyhow::Context;
use anyhow::Result;
use std::path::PathBuf;

use log::warn;

use crate::{
    idf_config::{IdfConfig, IdfInstallation},
    settings::Settings,
};

fn get_default_config_path() -> PathBuf {
    let default_settings = Settings::default();
    PathBuf::from(default_settings.esp_idf_json_path.unwrap_or_default()).join("esp_ide.json")
}

// todo: add optional path parameter enabling the user to specify a custom config file
// or to search for it in a different location ( or whole filesystem)
pub fn list_installed_versions() -> Result<Vec<IdfInstallation>> {
    let config_path = get_default_config_path();
    get_installed_versions_from_config_file(&config_path)
}

pub fn get_installed_versions_from_config_file(
    config_path: &PathBuf,
) -> Result<Vec<IdfInstallation>> {
    if config_path.is_file() {
        let ide_config = IdfConfig::from_file(config_path)?;
        return Ok(ide_config.idf_installed);
    }
    Err(anyhow!("Config file not found"))
}

pub fn get_esp_ide_config() -> Result<IdfConfig> {
    let config_path = get_default_config_path();
    IdfConfig::from_file(&config_path)
}

pub fn get_selected_version() -> Option<IdfInstallation> {
    let config_path = get_default_config_path();
    let ide_config = IdfConfig::from_file(config_path).ok();
    if let Some(config) = ide_config {
        match config.get_selected_installation() {
            Some(selected) => return Some(selected.clone()),
            None => {
                warn!("No selected version found in config file");
                return None;
            }
        }
    }
    None
}

pub fn select_idf_version(identifier: &str) -> Result<String> {
    let config_path = get_default_config_path();
    let mut ide_config = IdfConfig::from_file(&config_path)?;
    if ide_config.select_installation(identifier) {
        ide_config.to_file(config_path, true)?;
        return Ok(format!("Version {} selected", identifier));
    }
    Err(anyhow!("Version {} not installed", identifier))
}

pub fn rename_idf_version(identifier: &str, new_name: String) -> Result<String> {
    let config_path = get_default_config_path();
    let mut ide_config = IdfConfig::from_file(&config_path)?;
    let res = ide_config.update_installation_name(identifier, new_name.to_string());
    if res {
        ide_config.to_file(config_path, true)?;
        Ok(format!("Version {} renamed to {}", identifier, new_name))
    } else {
        Err(anyhow!("Version {} not installed", identifier))
    }
}
