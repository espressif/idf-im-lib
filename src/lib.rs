use decompress::{self, DecompressError, Decompression, ExtractOptsBuilder};
use git2::{FetchOptions, ObjectType, Progress, RemoteCallbacks, Repository};
use reqwest::Client;
use sha2::{Digest, Sha256};

pub mod idf_tools;
pub mod idf_versions;
pub mod python_utils;
pub mod system_dependencies;
use std::{
    env,
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};

pub fn verify_file_checksum(expected_checksum: &str, file_path: &str) -> Result<bool, io::Error> {
    if !Path::new(file_path).exists() {
        return Ok(false);
    }

    let mut file = File::open(file_path)?;

    let mut hasher = Sha256::new();

    let mut buffer = [0; 1024];
    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    // Get the final hash
    let result = hasher.finalize();

    // Convert the hash to a hexadecimal string
    let computed_checksum = format!("{:x}", result);

    // Compare the computed checksum with the expected checksum
    Ok(computed_checksum == expected_checksum)
}

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

// this work on windows only
pub fn add_path_to_path(directory_path: &str) {
    let current_path = env::var("PATH").unwrap_or_default();
    if !current_path.contains(directory_path) {
        let new_path = if current_path.is_empty() {
            directory_path.to_owned()
        } else {
            format!("{};{}", current_path, directory_path)
        };

        env::set_var("PATH", new_path);
    }
}

fn shallow_clone(
    url: &str,
    path: &str,
    branch: Option<&str>,
    tag: Option<&str>,
    progress_function: impl FnMut(Progress<'_>) -> bool,
) -> Result<Repository, git2::Error> {
    let mut fo = FetchOptions::new();
    if tag.is_none() {
        fo.depth(1);
    }

    let mut callbacks = RemoteCallbacks::new();
    callbacks.transfer_progress(progress_function);
    fo.remote_callbacks(callbacks);

    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);
    if let Some(branch) = branch {
        builder.branch(branch);
    };

    let repo = builder.clone(url, Path::new(path))?;
    if let Some(tag) = tag {
        // Look up the tag
        let tag_ref = repo.find_reference(&format!("refs/tags/{}", tag))?;
        let tag_obj = tag_ref.peel(ObjectType::Commit)?;

        // Checkout the commit that the tag points to
        repo.checkout_tree(&tag_obj, None)?;
        repo.set_head_detached(tag_obj.id())?;
    };
    if let Some(branch) = branch {
        let obj = repo.revparse_single(&format!("origin/{}", branch))?;
        repo.checkout_tree(&obj, None)?;
        repo.set_head(&format!("refs/heads/{}", branch))?;
    };
    Ok(repo)
}

pub fn get_rustpython_fork(
    custom_path: &str,
    progress_function: impl FnMut(Progress<'_>) -> bool,
) -> Result<String, git2::Error> {
    let output = shallow_clone(
        "https://github.com/Hahihula/RustPython.git",
        custom_path,
        Some("test-rust-build"),
        None,
        progress_function,
    );
    match output {
        Ok(repo) => Ok(repo.path().to_str().unwrap().to_string()),
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

pub fn get_esp_idf_by_tag_name(
    custom_path: &str,
    tag: &str,
    progress_function: impl FnMut(Progress<'_>) -> bool,
) -> Result<String, git2::Error> {
    let _ = ensure_path(custom_path);
    let output = shallow_clone(
        "https://github.com/espressif/esp-idf.git",
        custom_path,
        None,
        Some(tag),
        progress_function,
    );
    match output {
        Ok(repo) => Ok(repo.path().to_str().unwrap().to_string()),
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
