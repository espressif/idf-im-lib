use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
pub struct IdfInstallation {
    pub activation_script: String,
    pub id: String,
    #[serde(rename = "idfToolsPath")]
    pub idf_tools_path: String,
    pub name: String,
    pub path: String,
    pub python: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IdfConfig {
    #[serde(rename = "gitPath")]
    pub git_path: String,
    #[serde(rename = "idfInstalled")]
    pub idf_installed: Vec<IdfInstallation>,
    #[serde(rename = "idfSelectedId")]
    pub idf_selected_id: String,
}

impl IdfConfig {
    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path)?;
        let config: IdfConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    // Helper method to get the currently selected installation
    pub fn get_selected_installation(&self) -> Option<&IdfInstallation> {
        self.idf_installed
            .iter()
            .find(|install| install.id == self.idf_selected_id)
    }
}

// Example usage function
pub fn parse_idf_config<P: AsRef<Path>>(path: P) -> Result<IdfConfig> {
    IdfConfig::from_file(path)
}
