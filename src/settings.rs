use config::{Config, ConfigError, File};
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct Settings {
    pub path: Option<PathBuf>,
    pub idf_path: Option<PathBuf>,
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
}
