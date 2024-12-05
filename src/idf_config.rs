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
    pub fn to_file<P: AsRef<Path>>(&mut self, path: P, pretty: bool) -> Result<()> {
        // Create parent directories if they don't exist
        ensure_path(path.as_ref().parent().unwrap().to_str().unwrap())?;

        // Convert to JSON string
        let json_string = if pretty {
            serde_json::to_string_pretty(self)
        } else {
            serde_json::to_string(self)
        }
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        if path.as_ref().exists() {
            let existing_config = IdfConfig::from_file(path.as_ref())?;
            let existing_version = existing_config.idf_installed;
            self.idf_installed.extend(existing_version);
        }
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;

        file.write_all(json_string.as_bytes())
            .with_context(|| anyhow!("writing to file esp_ide.json failed"))
    }

    /// Reads and parses an IDF configuration from a file.
    ///
    /// # Arguments
    ///
    /// * `path` - A value that can be converted into a Path, representing the location of the configuration file.
    ///
    /// # Returns
    ///
    /// Returns a `Result` containing the parsed `IdfConfig` if successful, or an error if the file
    /// cannot be read or parsed.
    ///
    /// # Errors
    ///
    /// This function will return an error if:
    /// - The file cannot be read
    /// - The file contents cannot be parsed as valid JSON
    /// - The JSON structure does not match the `IdfConfig` structure
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

    /// Updates the name of an IDF installation in the configuration.
    ///
    /// This function searches for an installation matching the given identifier
    /// (either by ID or name) and updates its name to the provided new name.
    ///
    /// # Arguments
    ///
    /// * `identifier` - A string slice that holds the ID or current name of the installation to update.
    /// * `new_name` - A String that will be set as the new name for the matched installation.
    ///
    /// # Returns
    ///
    /// Returns a boolean:
    /// * `true` if an installation was found and its name was updated.
    /// * `false` if no matching installation was found.
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

    /// Selects an IDF installation in the configuration.
    ///
    /// This function searches for an installation matching the given identifier
    /// (either by ID or name) and sets it as the selected installation.
    ///
    /// # Arguments
    ///
    /// * `identifier` - A string slice that holds the ID or name of the installation to select.
    ///
    /// # Returns
    ///
    /// Returns a boolean:
    /// * `true` if a matching installation was found and selected.
    /// * `false` if no matching installation was found.
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

    /// Removes an IDF installation from the configuration.
    ///
    /// This function searches for an installation matching the given identifier
    /// (either by ID or name) and removes it from the list of installed IDFs.
    /// If the removed installation was the currently selected one, it clears the selection.
    ///
    /// # Arguments
    ///
    /// * `identifier` - A string slice that holds the ID or name of the installation to remove.
    ///
    /// # Returns
    ///
    /// Returns a boolean:
    /// * `true` if a matching installation was found and removed.
    /// * `false` if no matching installation was found.
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

pub fn parse_idf_config<P: AsRef<Path>>(path: P) -> Result<IdfConfig> {
    IdfConfig::from_file(path)
}
