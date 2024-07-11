use decompress::{self, DecompressError, Decompression, ExtractOptsBuilder};
use git2::{FetchOptions, ObjectType, Progress, RemoteCallbacks, Repository};
use reqwest::Client;
use sha2::{Digest, Sha256};

pub mod idf_tools;
pub mod idf_versions;
pub mod python_utils;
pub mod system_dependencies;
#[cfg(windows)]
pub mod win_tools;
use std::{
    env,
    fs::{self, File},
    io::{self, Read, Write},
    path::Path,
};

/// Verifies the SHA256 checksum of a file against an expected checksum.
///
/// # Arguments
///
/// * `expected_checksum` - A string representing the expected SHA256 checksum.
/// * `file_path` - A string representing the path to the file to be verified.
///
/// # Returns
///
/// * `Ok(true)` if the file's checksum matches the expected checksum.
/// * `Ok(false)` if the file does not exist or its checksum does not match the expected checksum.
/// * `Err(io::Error)` if an error occurs while opening or reading the file.
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

/// Asynchronously downloads a file from a given URL to a specified destination path.
///
/// # Arguments
///
/// * `url` - A string representing the URL from which to download the file.
/// * `destination_path` - A string representing the path to which the file should be downloaded.
/// * `show_progress` - A function pointer to a function that will be called to show the progress of the download.
///
/// # Returns
///
/// * `Ok(())` if the file was successfully downloaded.
/// * `Err(std::io::Error)` if an error occurred during the download process.
///
/// # Example
///
/// ```rust
/// use std::io::Write;
///
/// async fn download_progress_callback(downloaded: u64, total: u64) {
///     let percentage = (downloaded as f64 / total as f64) * 100.0;
///     print!("\rDownloading... {:.2}%", percentage);
///     io::stdout().flush().unwrap();
/// }
///
/// #[tokio::main]
/// async fn main() {
///     let url = "https://example.com/file.zip";
///     let destination_path = "/path/to/destination";
///
///     match download_file(url, destination_path, &download_progress_callback).await {
///         Ok(()) => println!("\nDownload completed successfully"),
///         Err(e) => eprintln!("Error during download: {}", e),
///     }
/// }
/// ```
pub async fn download_file(
    url: &str,
    destination_path: &str,
    show_progress: &dyn Fn(u64, u64),
) -> Result<(), std::io::Error> {
    // Create a new HTTP client
    let client = Client::new();

    // Send a GET request to the specified URL
    let mut response = client
        .get(url)
        .send()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    // Get the total size of the file being downloaded
    let total_size = response.content_length().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::Other, "Failed to get content length")
    })?;

    // Extract the filename from the URL
    let filename = Path::new(&url).file_name().unwrap().to_str().unwrap();

    // Create a new file at the specified destination path
    let mut file = File::create(Path::new(&destination_path).join(Path::new(filename)))?;

    // Initialize the amount downloaded
    let mut amount_downloaded: u64 = 0;

    // Download the file in chunks
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?
    {
        // Update the amount downloaded
        amount_downloaded += chunk.len() as u64;

        // Write the chunk to the file
        file.write_all(&chunk)?;

        // Call the progress callback function
        show_progress(amount_downloaded, total_size);
    }

    // Return Ok(()) if the download was successful
    Ok(())
}

/// Decompresses an archive file to a specified destination directory.
///
/// # Arguments
///
/// * `archive_path` - A string representing the path to the archive file to be decompressed.
/// * `destination_path` - A string representing the path to the directory where the decompressed files should be placed.
///
/// # Returns
///
/// * `Ok(Decompression)` if the archive was successfully decompressed.
/// * `Err(DecompressError)` if an error occurred during the decompression process.
///
/// # Example
///
/// ```rust
/// use decompress::{self, DecompressError, Decompression, ExtractOptsBuilder};
///
/// fn main() {
///     let archive_path = "/path/to/archive.zip";
///     let destination_path = "/path/to/destination";
///
///     match decompress_archive(archive_path, destination_path) {
///         Ok(decompression) => println!("Archive decompressed successfully"),
///         Err(e) => eprintln!("Error during decompression: {}", e),
///     }
/// }
/// ```
pub fn decompress_archive(
    archive_path: &str,
    destination_path: &str,
) -> Result<Decompression, DecompressError> {
    let opts = &ExtractOptsBuilder::default().strip(0).build().unwrap();
    decompress::decompress(archive_path, destination_path, opts)
}

/// Ensures that a directory exists at the specified path.
/// If the directory does not exist, it will be created.
///
/// # Arguments
///
/// * `directory_path` - A string representing the path to the directory to be ensured.
///
/// # Returns
///
/// * `Ok(())` if the directory was successfully created or already exists.
/// * `Err(std::io::Error)` if an error occurred while creating the directory.
pub fn ensure_path(directory_path: &str) -> std::io::Result<()> {
    let path = Path::new(directory_path);
    if !path.exists() {
        // If the directory does not exist, create it
        fs::create_dir_all(directory_path)?;
    }
    Ok(())
}

/// Adds a directory to the system's PATH environment variable.
/// If the directory is already present in the PATH, it will not be added again.
///
/// # Arguments
///
/// * `directory_path` - A string representing the path of the directory to be added to the PATH.
///
/// # Example
///
/// ```rust
/// add_path_to_path("/usr/local/bin");
/// ```
pub fn add_path_to_path(directory_path: &str) {
    // Retrieve the current PATH environment variable.
    // If it does not exist, use an empty string as the default value.
    let current_path = env::var("PATH").unwrap_or_default();

    // Check if the directory path is already present in the PATH.
    // If it is not present, construct a new PATH string with the directory path added.
    if !current_path.contains(directory_path) {
        let new_path = if current_path.is_empty() {
            directory_path.to_owned()
        } else {
            format!("{};{}", current_path, directory_path)
        };

        // Set the new PATH environment variable.
        env::set_var("PATH", new_path);
    }
}

/// Performs a shallow clone of a Git repository.
///
/// # Arguments
///
/// * `url` - A string representing the URL of the Git repository to clone.
/// * `path` - A string representing the local path where the repository should be cloned.
/// * `branch` - An optional string representing the branch to checkout after cloning. If `None`, the default branch will be checked out.
/// * `tag` - An optional string representing the tag to checkout after cloning. If `None`, the repository will be cloned at the specified branch.
/// * `progress_function` - A closure or function that will be called to report progress during the cloning process.
///
/// # Returns
///
/// * `Ok(Repository)` if the cloning process is successful and the repository is opened.
/// * `Err(git2::Error)` if an error occurs during the cloning process.
///
fn shallow_clone(
    url: &str,
    path: &str,
    branch: Option<&str>,
    tag: Option<&str>,
    progress_function: impl FnMut(Progress<'_>) -> bool,
) -> Result<Repository, git2::Error> {
    // Initialize fetch options with depth 1 for shallow cloning
    let mut fo = FetchOptions::new();
    if tag.is_none() {
        fo.depth(1);
    }

    // Set up remote callbacks for progress reporting
    let mut callbacks = RemoteCallbacks::new();
    callbacks.transfer_progress(progress_function);
    fo.remote_callbacks(callbacks);

    // Create a new repository builder with the fetch options
    let mut builder = git2::build::RepoBuilder::new();
    builder.fetch_options(fo);

    // Set the branch to checkout if specified
    if let Some(branch) = branch {
        builder.branch(branch);
    };

    // Clone the repository
    let repo = builder.clone(url, Path::new(path))?;

    // If a tag is specified, checkout the corresponding commit
    if let Some(tag) = tag {
        // Look up the tag reference
        let tag_ref = repo.find_reference(&format!("refs/tags/{}", tag))?;
        // Peel the tag reference to get the commit object
        let tag_obj = tag_ref.peel(ObjectType::Commit)?;

        // Checkout the commit that the tag points to
        repo.checkout_tree(&tag_obj, None)?;
        repo.set_head_detached(tag_obj.id())?;
    };

    // If a branch is specified, checkout the corresponding branch
    if let Some(branch) = branch {
        // Rev-parse the branch reference to get the commit object
        let obj = repo.revparse_single(&format!("origin/{}", branch))?;
        // Checkout the commit that the branch points to
        repo.checkout_tree(&obj, None)?;
        repo.set_head(&format!("refs/heads/{}", branch))?;
    };

    // Return the opened repository
    Ok(repo)
}

// This function is not used right now  because of limited scope of the POC
// It gets specific fork of rustpython with build in libraries needed for IDF
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

// kept for pure reference how the IDF tools shouldc be runned using rustpython
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

/// Retrieves the ESP-IDF repository by cloning it from GitHub using a specific tag.
///
/// # Arguments
///
/// * `custom_path` - A string representing the local path where the ESP-IDF repository should be cloned.
/// * `tag` - A string representing the tag of the ESP-IDF repository to clone.
/// * `progress_function` - A closure or function that will be called to report progress during the cloning process.
///
/// # Returns
///
/// * `Ok(String)` if the cloning process is successful and the path to the cloned repository is returned.
/// * `Err(git2::Error)` if an error occurs during the cloning process.
///
/// # Example
///
/// ```rust
/// use git2::Progress;
///
/// fn progress_callback(progress: Progress) -> bool {
///     // Implement progress callback logic here
///     // Return true to continue the cloning process, or false to cancel it
///     true
/// }
///
/// fn main() {
///     let custom_path = "/path/to/esp-idf";
///     let tag = "v4.3";
///
///     match get_esp_idf_by_tag_name(custom_path, tag, progress_callback) {
///         Ok(repo_path) => println!("ESP-IDF repository cloned successfully at: {}", repo_path),
///         Err(e) => eprintln!("Error during cloning: {}", e),
///     }
/// }
/// ```
pub fn get_esp_idf_by_tag_name(
    custom_path: &str,
    tag: Option<&str>,
    progress_function: impl FnMut(Progress<'_>) -> bool,
    mirror: Option<&str>,
    group_name: Option<&str>,
) -> Result<String, git2::Error> {
    let group = match group_name {
        Some(group) => group,
        None => "espressif",
    };
    let url = match mirror {
        Some(url) => {
            format!("https://github.com/{}/esp-idf.git", group).replace("https://github.com", url)
        }
        None => "https://github.com/espressif/esp-idf.git".to_string(),
    };

    let _ = ensure_path(custom_path);
    let output = match tag {
        Some(tag) => shallow_clone(&url, custom_path, None, Some(tag), progress_function),
        None => shallow_clone(&url, custom_path, Some("master"), None, progress_function),
    };
    match output {
        Ok(repo) => Ok(repo.path().to_str().unwrap().to_string()),
        Err(e) => Err(e),
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_verify_file_checksum_with_valid_file() {
        let file_path = "test_file.txt";
        let expected_checksum = "e2d0fe1585a63ec6009c8016ff8dda8b17719a637405a4e23c0ff81339148249";

        // Create a test file with the expected content
        fs::write(file_path, "This is a test file").unwrap();

        let result = verify_file_checksum(expected_checksum, file_path);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // Clean up the test file
        fs::remove_file(file_path).unwrap();
    }

    #[test]
    fn test_verify_file_checksum_with_invalid_checksum() {
        let file_path = "test_file_inv.txt";
        let expected_checksum = "invalid_checksum";

        // Create a test file with the expected content
        fs::write(file_path, "This is a test file").unwrap();

        let result = verify_file_checksum(expected_checksum, file_path);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);

        // Clean up the test file
        fs::remove_file(file_path).unwrap();
    }
    #[test]
    fn test_verify_file_checksum_with_nonexistent_file() {
        let file_path = "nonexistent_file.txt";
        let expected_checksum = "6a266d99f1729281c1b7a079793898292837a659";

        let result = verify_file_checksum(expected_checksum, file_path);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false);
    }

    #[test]
    fn test_verify_file_checksum_with_empty_file() {
        let file_path = "empty_file.txt";
        let expected_checksum = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

        // Create an empty test file
        fs::File::create(file_path).unwrap();

        let result = verify_file_checksum(expected_checksum, file_path);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // Clean up the test file
        fs::remove_file(file_path).unwrap();
    }
    #[test]
    fn test_verify_file_checksum_with_large_file() {
        let file_path = "large_file.txt";
        let expected_checksum = "ef2e29e83198cfd2d1edd7b8c1508235d16a78d2d3a00e493c9c0bdebce8eecc";

        // Create a large test file with the expected content
        let mut file = fs::File::create(file_path).unwrap();
        for _ in 0..1000000 {
            file.write_all(b"This is a test file").unwrap();
        }

        let result = verify_file_checksum(expected_checksum, file_path);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true);

        // Clean up the test file
        fs::remove_file(file_path).unwrap();
    }
    #[test]
    fn test_ensure_path_with_special_characters() {
        let directory_path = "/tmp/path/to/directory with spaces and@special#characters";

        // Remove the directory if it already exists
        fs::remove_dir_all(directory_path).ok();

        let result = ensure_path(directory_path);

        assert!(result.is_ok());

        // Clean up the directory
        fs::remove_dir_all(directory_path).unwrap();
    }
    #[test]
    fn test_ensure_path_with_existing_directory() {
        let directory_path = "./python_scripts";

        // Create the existing directory
        fs::create_dir_all(directory_path).unwrap();

        let result = ensure_path(directory_path);

        assert!(result.is_ok());
    }
}
