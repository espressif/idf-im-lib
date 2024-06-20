use serde::Deserialize;
use std::collections::HashMap;
use std::env::consts::ARCH;
use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use crate::python_utils::get_python_platform_definition;

#[derive(Deserialize, Debug)]
pub struct Tool {
    pub description: String,
    pub export_paths: Vec<Vec<String>>,
    pub export_vars: HashMap<String, String>,
    pub info_url: String,
    pub install: String,
    #[serde(default)]
    pub license: Option<String>,
    pub name: String,
    #[serde(default)]
    pub platform_overrides: Option<Vec<PlatformOverride>>,
    #[serde(default)]
    pub supported_targets: Option<Vec<String>>,
    #[serde(default)]
    pub strip_container_dirs: Option<u8>,
    pub version_cmd: Vec<String>,
    pub version_regex: String,
    #[serde(default)]
    pub version_regex_replace: Option<String>,
    pub versions: Vec<Version>,
}

#[derive(Deserialize, Debug)]
pub struct PlatformOverride {
    #[serde(default)]
    pub install: Option<String>,
    pub platforms: Vec<String>,
    #[serde(default)]
    pub export_paths: Option<Vec<Vec<String>>>,
}

#[derive(Deserialize, Debug)]
pub struct Version {
    pub name: String,
    pub status: String,
    #[serde(flatten)]
    pub downloads: HashMap<String, Download>,
}

#[derive(Deserialize, Debug)]
pub struct Download {
    pub sha256: String,
    pub size: u64,
    pub url: String,
    #[serde(default)]
    pub rename_dist: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ToolsFile {
    pub tools: Vec<Tool>,
    pub version: u8,
}

pub fn read_and_parse_tools_file(path: &str) -> Result<ToolsFile, Box<dyn std::error::Error>> {
    // Read the file contents
    let path = Path::new(path);
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;

    // Parse the file contents into a ToolsFile struct using serde
    let tools_file: ToolsFile = serde_json::from_str(&contents)?;

    Ok(tools_file)
}

pub fn filter_tools_by_target(tools: Vec<Tool>, target: &String) -> Vec<Tool> {
    tools
        .into_iter()
        .filter(|tool| {
            if let Some(supported_targets) = &tool.supported_targets {
                supported_targets.contains(target) || supported_targets.contains(&"all".to_string())
            } else {
                true
            }
        })
        .collect()
}

// TODO: maybe get this by direct calling the idf_tool.py so the hashtable is not duplicate
pub fn get_platform_identification() -> Result<String, String> {
    let mut platform_from_name = HashMap::new();

    // Windows
    platform_from_name.insert("win32", "win32");
    platform_from_name.insert("Windows-i686", "win32");
    platform_from_name.insert("Windows-x86", "win32");
    platform_from_name.insert("i686-w64-mingw32", "win32");
    platform_from_name.insert("win64", "win64");
    platform_from_name.insert("Windows-x86_64", "win64");
    platform_from_name.insert("Windows-AMD64", "win64");
    platform_from_name.insert("x86_64-w64-mingw32", "win64");
    platform_from_name.insert("Windows-ARM64", "win64");

    // macOS
    platform_from_name.insert("macos", "macos");
    platform_from_name.insert("osx", "macos");
    platform_from_name.insert("darwin", "macos");
    platform_from_name.insert("Darwin-x86_64", "macos");
    platform_from_name.insert("x86_64-apple-darwin", "macos");
    platform_from_name.insert("macos-arm64", "macos-arm64");
    platform_from_name.insert("macos-aarch64", "macos-arm64");
    platform_from_name.insert("Darwin-arm64", "macos-arm64");
    platform_from_name.insert("Darwin-aarch64", "macos-arm64");
    platform_from_name.insert("aarch64-apple-darwin", "macos-arm64");
    platform_from_name.insert("arm64-apple-darwin", "macos-arm64");
    platform_from_name.insert("aarch64-apple-darwin", "macos-arm64");

    // Linux
    platform_from_name.insert("linux-amd64", "linux-amd64");
    platform_from_name.insert("linux64", "linux-amd64");
    platform_from_name.insert("Linux-x86_64", "linux-amd64");
    platform_from_name.insert("FreeBSD-amd64", "linux-amd64");
    platform_from_name.insert("x86_64-linux-gnu", "linux-amd64");
    platform_from_name.insert("linux-i686", "linux-i686");
    platform_from_name.insert("linux32", "linux-i686");
    platform_from_name.insert("Linux-i686", "linux-i686");
    platform_from_name.insert("FreeBSD-i386", "linux-i686");
    platform_from_name.insert("i586-linux-gnu", "linux-i686");
    platform_from_name.insert("i686-linux-gnu", "linux-i686");
    platform_from_name.insert("linux-arm64", "linux-arm64");
    platform_from_name.insert("Linux-arm64", "linux-arm64");
    platform_from_name.insert("Linux-aarch64", "linux-arm64");
    platform_from_name.insert("Linux-armv8l", "linux-arm64");
    platform_from_name.insert("aarch64", "linux-arm64");
    platform_from_name.insert("linux-armhf", "linux-armhf");
    platform_from_name.insert("arm-linux-gnueabihf", "linux-armhf");
    platform_from_name.insert("linux-armel", "linux-armel");
    platform_from_name.insert("arm-linux-gnueabi", "linux-armel");
    platform_from_name.insert("Linux-armv7l", "linux-armel");
    platform_from_name.insert("Linux-arm", "linux-armel");

    let python_platform_string = get_python_platform_definition(None).trim().to_string();

    let platform = match platform_from_name.get(&python_platform_string.as_str()) {
        Some(platform) => platform,
        None => return Err(format!("Unsupported platform: {}", python_platform_string)),
    };
    Ok(platform.to_string())
}

pub fn get_download_link_by_platform(
    tools: Vec<Tool>,
    platform: &String,
) -> HashMap<String, String> {
    // println!("{:#?}", tools);
    let mut tool_links = HashMap::new();
    for tool in tools {
        // tool.name
        tool.versions.iter().for_each(|version| {
            let download_link = match version.downloads.get(platform) {
                Some(download) => Some(download.url.clone()),
                None => None,
            };
            if let Some(download_link) = download_link {
                tool_links.insert(tool.name.clone(), download_link.clone());
            }
        });
    }
    // println!("{:#?}", tool_links);
    tool_links
}

// only known mirror at the time is: "dl.espressif.com/github_assets"
pub fn change_links_donwanload_mirror(
    tools: HashMap<String, String>,
    mirror: Option<&str>,
) -> HashMap<String, String> {
    let new_tools: HashMap<String, String> = tools
        .iter()
        .map(|(name, link)| {
            let new_link = match mirror {
                Some(mirror) => link.replace("https://github.com", mirror),
                None => link.to_string(),
            };
            (name.to_string(), new_link)
        })
        .collect();
    new_tools
}
