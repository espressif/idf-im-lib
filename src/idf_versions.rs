use serde_derive::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Clone)]
pub struct Version {
    pub name: String,
    #[serde(default)]
    pub pre_release: bool,
    #[serde(default)]
    pub old: bool,
    #[serde(default)]
    pub end_of_life: bool,
    #[serde(default)]
    pub has_targets: bool,
    #[serde(default)]
    pub supported_targets: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IDFTarget {
    pub text: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Release {
    pub start_date: String,
    pub end_date: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Releases {
    pub VERSIONS: Vec<Version>,
    pub IDF_TARGETS: Vec<IDFTarget>,
    pub RELEASES: std::collections::HashMap<String, Release>,
}

// TODO: handle the possibility of multiple downloads
pub async fn get_idf_versions() -> Result<Releases, String> {
    return Ok(download_idf_versions().await.unwrap());
}

pub async fn get_avalible_targets() -> Result<Vec<String>, String> {
    let versions = get_idf_versions().await;
    match versions {
        Ok(releases) => {
            let mut avalible_targets = vec![];
            for target in &releases.IDF_TARGETS {
                avalible_targets.push(target.value.clone());
            }
            return Ok(avalible_targets);
        }
        Err(err) => Err(err),
    }
}

pub async fn download_idf_versions() -> Result<Releases, Box<dyn std::error::Error>> {
    let url = "https://dl.espressif.com/dl/esp-idf/idf_versions.json".to_string();
    let client = reqwest::Client::builder()
        .user_agent("esp-idf-installer")
        .build()?;
    let response = client.get(&url).send().await?;
    let json_versions_file = response.text().await?;
    let versions: Releases = serde_json::from_str(&json_versions_file)?;

    Ok(versions)
}

pub fn get_idf_versions_by_target(versions: &Releases) -> HashMap<String, Vec<Version>> {
    let mut versions_by_target = HashMap::new();

    for target in &versions.IDF_TARGETS {
        let version_list = versions
            .VERSIONS
            .iter()
            .filter(|v| v.supported_targets.contains(&target.value))
            .cloned()
            .collect();
        versions_by_target.insert(target.value.clone(), version_list);
    }
    versions_by_target
}

pub async fn get_idf_name_by_target(target: &String) -> Vec<String> {
    let versions = get_idf_versions().await;
    let versions_by_target = get_idf_versions_by_target(&versions.unwrap());
    let mut selected_versions = vec![];
    if let Some(versions) = versions_by_target.get(target) {
        for v in versions {
            if v.end_of_life || v.pre_release || v.old {
                continue;
            }
            selected_versions.push(v.name.clone());
        }
    }
    selected_versions
}
