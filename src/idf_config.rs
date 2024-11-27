use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::Path;

use crate::ensure_path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdfInstallation {
    #[serde(rename = "activationScript")]
    pub activation_script: String,
    pub id: String,
    #[serde(rename = "idfToolsPath")]
    pub idf_tools_path: String,
    pub name: String,
    pub path: String,
    pub python: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct IdfConfig {
    #[serde(rename = "gitPath")]
    pub git_path: String,
    #[serde(rename = "idfInstalled")]
    pub idf_installed: Vec<IdfInstallation>,
    #[serde(rename = "idfSelectedId")]
    pub idf_selected_id: String,
}

impl IdfConfig {
    /// Saves the configuration to a file.
    ///
    /// # Arguments
    ///
    /// * `path` - The path where to save the configuration file
    /// * `pretty` - If true, the JSON will be pretty-printed
    ///
    /// # Returns
    ///
    /// Returns `io::Result<()>` which is Ok if the file was successfully written
    ///
    /// # Examples
    ///
    /// ```rust
    /// let config = IdfConfig { ... };
    /// config.to_file("esp_ide.json", true)?;
    /// ```
    pub fn to_file<P: AsRef<Path>>(&self, path: P, pretty: bool) -> Result<()> {
        // Create parent directories if they don't exist
        ensure_path(path.as_ref().parent().unwrap().to_str().unwrap())?;

        // Convert to JSON string
        let json_string = if pretty {
            serde_json::to_string_pretty(self)
        } else {
            serde_json::to_string(self)
        }
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        // Write to file
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        file.write_all(json_string.as_bytes())
            .with_context(|| anyhow!("writing to file esp_ide.json failed"))
    }

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
    // Updates the name of an IDF installation identified by either name or id
    /// Returns true if a matching installation was found and updated
    pub fn update_installation_name(&mut self, identifier: &str, new_name: String) -> bool {
        if let Some(installation) = self
            .idf_installed
            .iter_mut()
            .find(|install| install.id == identifier || install.name == identifier)
        {
            installation.name = new_name;
            true
        } else {
            false
        }
    }
    /// Changes the currently selected IDF version using either name or id as identifier
    /// Returns true if a matching installation was found and selected
    pub fn select_installation(&mut self, identifier: &str) -> bool {
        if let Some(installation) = self
            .idf_installed
            .iter()
            .find(|install| install.id == identifier || install.name == identifier)
        {
            self.idf_selected_id = installation.id.clone();
            true
        } else {
            false
        }
    }
    /// Removes an IDF installation from the config using either name or id as identifier
    /// Returns true if a matching installation was found and removed
    /// If the removed installation was selected, clears the selected_id
    pub fn remove_installation(&mut self, identifier: &str) -> bool {
        if let Some(index) = self
            .idf_installed
            .iter()
            .position(|install| install.id == identifier || install.name == identifier)
        {
            // If we're removing the currently selected installation, clear the selection
            if self.idf_selected_id == self.idf_installed[index].id {
                self.idf_selected_id.clear();
                // TODO: prompt user to select a new installation if there are any left
            }

            // Remove the installation
            self.idf_installed.remove(index);
            true
        } else {
            false
        }
    }
}

// Example usage function
pub fn parse_idf_config<P: AsRef<Path>>(path: P) -> Result<IdfConfig> {
    IdfConfig::from_file(path)
}
