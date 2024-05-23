use decompress::{self, DecompressError, Decompression, ExtractOptsBuilder};
use reqwest::Client;

pub mod idf_tools;
pub mod idf_versions;
pub mod python_utils;
pub mod system_dependencies;
use std::{
    env,
    fs::{self, File},
    io::Write,
    path::Path,
};

pub async fn download_file(
    url: &str,
    destination_path: &str,
    show_progress: &dyn Fn(u64, u64),
) -> Result<(), std::io::Error> {
    let client = Client::new();
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
    let total_size = response.content_length().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::Other, "Failed to get content length")
    })?;
    let filename = Path::new(&url).file_name().unwrap().to_str().unwrap();
    let mut file = File::create(Path::new(&destination_path).join(Path::new(filename)))?;
    let mut amount_downloaded: u64 = 0;

    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
    {
        amount_downloaded += chunk.len() as u64;
        file.write_all(&chunk)?;
        show_progress(amount_downloaded, total_size);
    }

    Ok(())
}

pub fn decompress_archive(
    archive_path: &str,
    destination_path: &str,
) -> Result<Decompression, DecompressError> {
    let opts = &ExtractOptsBuilder::default().strip(0).build().unwrap();
    decompress::decompress(archive_path, destination_path, opts)
}

pub fn ensure_path(directory_path: &str) -> std::io::Result<()> {
    let path = Path::new(directory_path);
    if !path.exists() {
        fs::create_dir_all(directory_path)?;
    }
    Ok(())
}

pub fn add_path_to_path(directory_path: &str) {
    let current_path = match env::var("PATH") {
        Ok(path) => path,
        Err(_) => String::new(),
    };
    let new_path = format!("{}:{}", directory_path, current_path);
    env::set_var("PATH", &new_path);
}

pub fn get_rustpython_fork(custom_path: &str) -> Result<String, std::io::Error> {
    let output = std::process::Command::new("git")
        .current_dir(custom_path)
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--branch")
        .arg("test-rust-build")
        .arg("https://github.com/Hahihula/RustPython.git")
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(std::str::from_utf8(&out.stdout).unwrap().to_string())
            } else {
                Ok(std::str::from_utf8(&out.stderr).unwrap().to_string())
            }
        }
        Err(e) => Err(e),
    }
}

pub fn run_idf_tools_using_rustpython(custom_path: &str) -> Result<String, std::io::Error> {
    let script_path = "esp-idf/tools/idf_tools.py";
    // env::set_var("RUSTPYTHONPATH", "/tmp/test-directory/RustPython/Lib"); // this is not needed as the standart library is bakend into the binary
    let output = std::process::Command::new("rustpython") // this works only on my machine (needs to point to the rustpython executable)
        .current_dir(custom_path)
        .arg(script_path)
        .arg("--idf-path")
        .arg(format!("{}/esp-idf", custom_path))
        .arg("--tools-json")
        .arg(format!("{}/esp-idf/tools/tools.json", custom_path))
        .arg("install")
        .arg("--targets")
        .arg("all")
        .arg("all")
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(std::str::from_utf8(&out.stdout).unwrap().to_string())
            } else {
                Ok(std::str::from_utf8(&out.stderr).unwrap().to_string())
            }
        }
        Err(e) => Err(e),
    }
}

pub fn get_esp_idf_by_tag_name(custom_path: &str, tag: &str) -> Result<String, std::io::Error> {
    let _ = ensure_path(custom_path);
    let output = std::process::Command::new("git")
        .current_dir(custom_path)
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg("--branch")
        .arg(tag)
        .arg("https://github.com/espressif/esp-idf.git")
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(std::str::from_utf8(&out.stdout).unwrap().to_string())
            } else {
                Ok(std::str::from_utf8(&out.stderr).unwrap().to_string())
            }
        }
        Err(e) => Err(e),
    }
}

pub fn apply_patchset(base_path: &str, patchset_name: &str) -> Result<String, std::io::Error> {
    let custom_path = format!("{}/esp-idf", base_path);
    let patchset_name = patchset_name; //"manual_522.patch";
    if let Err(e) = fs::copy(
        format!("../{}", patchset_name),
        format!("{}/{}", custom_path, patchset_name),
    ) {
        println!("Failed to copy file: {}", e);
    } else {
        println!("File copied successfully");
    }
    let output = std::process::Command::new("git")
        .current_dir(custom_path)
        .arg("apply")
        .arg(patchset_name)
        .output();
    match output {
        Ok(out) => {
            if out.status.success() {
                Ok(std::str::from_utf8(&out.stdout).unwrap().to_string())
            } else {
                Ok(std::str::from_utf8(&out.stderr).unwrap().to_string())
            }
        }
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
