use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::env::consts::OS;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;

use crate::utils::get_git_path;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Settings {
    pub path: Option<PathBuf>,
    pub idf_path: Option<PathBuf>, // TOOD: These are actually multiple because of multiple version --> remove from config alltogether or changed it to computed property
    pub esp_idf_json_path: Option<String>,
    pub tool_download_folder_name: Option<String>,
    pub tool_install_folder_name: Option<String>,
    pub target: Option<Vec<String>>,
    pub idf_versions: Option<Vec<String>>,
    pub tools_json_file: Option<String>,
    pub idf_tools_path: Option<String>,
    pub config_file: Option<PathBuf>,
    pub non_interactive: Option<bool>,
    pub wizard_all_questions: Option<bool>,
    pub mirror: Option<String>,
    pub idf_mirror: Option<String>,
    pub recurse_submodules: Option<bool>,
}

impl Default for Settings {
    fn default() -> Self {
        let esp_idf_json_path_value = match OS {
            "windows" => "C:\\Espressif\\tools".to_string(),
            _ => dirs::home_dir()
                .unwrap()
                .join(".espressif")
                .join("tools")
                .to_str()
                .unwrap()
                .to_string(),
        };
        Self {
            path: None,
            idf_path: None,
            esp_idf_json_path: Some(esp_idf_json_path_value),
            tool_download_folder_name: Some("dist".to_string()),
            tool_install_folder_name: Some("tools".to_string()),
            target: None,
            idf_versions: None,
            tools_json_file: Some("tools/tools.json".to_string()),
            idf_tools_path: Some("tools/idf_tools.py".to_string()),
            config_file: None,
            non_interactive: Some(false),
            wizard_all_questions: Some(false),
            mirror: None,
            idf_mirror: None,
            recurse_submodules: Some(false),
        }
    }
}

impl Settings {
    pub fn new(
        config_path: Option<PathBuf>,
        cli_settings: impl IntoIterator<Item = (String, Option<config::Value>)>,
    ) -> Result<Self, ConfigError> {
        let mut builder = Config::builder()
            .add_source(File::with_name("config/default").required(false))
            .add_source(File::with_name("config/development").required(false));

        if let Some(config_path) = config_path {
            builder = builder.add_source(File::from(config_path));
        }

        builder = builder.add_source(config::Environment::with_prefix("ESP").separator("_"));

        let mut cfg = builder.build()?;

        for (key, value) in cli_settings {
            if let Some(v) = value {
                if key != "config" {
                    cfg.set(&key, v)?;
                }
            }
        }

        cfg.try_deserialize()
    }

    pub fn save(&self, file_path: &str) -> Result<(), ConfigError> {
        let toml_value = toml::to_string(self).map_err(|e| ConfigError::Message(e.to_string()))?;
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(file_path)
            .map_err(|e| ConfigError::Message(e.to_string()))?;
        file.write_all(toml_value.as_bytes())
            .map_err(|e| ConfigError::Message(e.to_string()))?;

        Ok(())
    }

    /// Saves the ESP IDE configuration as a JSON file to the specified file path.
    ///
    /// This function collects information about the installed ESP-IDF versions, including paths to various tools
    /// and scripts associated with each version. It then constructs a JSON object containing these details and writes
    /// it to a file. If any errors occur during the process, a `Result` with an error message is returned.
    ///
    /// # Arguments
    ///
    /// * `file_path`: A string slice that holds the path where the JSON file will be saved.
    ///
    /// # Returns
    ///
    /// This function returns a `Result` which is `Ok(())` on success or an error message as a `String` on failure.
    pub fn save_esp_ide_json(&self, file_path: &str) -> Result<(), String> {
        let mut idf_installed = json!({});
        if let Some(versions) = &self.idf_versions {
            for version in versions {
                let id = format!("esp-idf-{}", Uuid::new_v4().to_string().replace("-", "")); //todo: use hash of path or something stable
                let base_path = self.path.as_ref().unwrap();
                let idf_path = base_path.clone().join(version).join("esp-idf");
                let tools_path = base_path
                    .clone()
                    .join(version)
                    .join(self.tool_install_folder_name.as_ref().unwrap());
                let python_path = match std::env::consts::OS {
                    "windows" => tools_path
                        .clone()
                        .join("python")
                        .join("Scripts")
                        .join("Python.exe"),
                    _ => tools_path
                        .clone()
                        .join("python")
                        .join("bin")
                        .join("python3"),
                };
                let activation_script = match std::env::consts::OS {
                    "windows" => base_path
                        .clone()
                        .join(version)
                        .join("Microsoft.PowerShell_profile.ps1"),
                    _ => base_path
                        .clone()
                        .join(format!("activate_idf_{}.sh", version)),
                };

                idf_installed[&id] = json!({
                    "name": version,
                    "path": idf_path,
                    "python": python_path,
                    "idfToolsPath": tools_path,
                    "activationScript": activation_script,
                });
            }
        }

        let git_path = get_git_path();

        let esp_ide_json = json!({
          "gitPath": git_path.unwrap(),
          "idfSelectedId": idf_installed.as_object().unwrap().keys().next().unwrap_or(&String::new()),
          "idfInstalled": idf_installed,
      })
      .to_string();
        let mut file = OpenOptions::new()
            .write(true)
            .truncate(true)
            .create(true)
            .open(file_path)
            .map_err(|e| e.to_string())?;
        file.write_all(esp_ide_json.as_bytes())
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
