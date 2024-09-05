use decompress::{self, DecompressError, Decompression, ExtractOptsBuilder};
use git2::{
    FetchOptions, ObjectType, Progress, RemoteCallbacks, Repository, SubmoduleUpdateOptions,
};
use log::{error, info, warn};
use reqwest::Client;
use sha2::{Digest, Sha256};
use tera::{Context, Tera};

pub mod idf_tools;
pub mod idf_versions;
pub mod python_utils;
pub mod system_dependencies;
use std::fs::{set_permissions, File};
use std::{
    env,
    fs::{self},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Command,
};

/// Creates an executable shell script with the given content and file path.
///
/// # Parameters
///
/// * `file_path`: A string representing the path where the shell script should be created.
/// * `content`: A string representing the content of the shell script.
///
/// # Return
///
/// * `Result<(), String>`: On success, returns `Ok(())`. On error, returns `Err(String)` containing the error message.
fn create_executable_shell_script(file_path: &str, content: &str) -> Result<(), String> {
    if std::env::consts::OS == "windows" {
        unimplemented!("create_executable_shell_script not implemented for Windows")
    } else {
        // Create and write to the file
        let mut file = File::create(file_path).map_err(|e| e.to_string())?;
        file.write_all(content.as_bytes())
            .map_err(|e| e.to_string())?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // Set the file as executable (mode 0o755)
            let permissions = PermissionsExt::from_mode(0o755);
            set_permissions(Path::new(file_path), permissions).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Creates an activation shell script for the ESP-IDF toolchain.
///
/// # Parameters
///
/// * `file_path`: A string representing the path where the activation script should be created.
/// * `idf_path`: A string representing the path to the ESP-IDF installation.
/// * `idf_tools_path`: A string representing the path to the ESP-IDF tools installation.
/// * `idf_version`: A string representing the version of the ESP-IDF toolchain.
/// * `export_paths`: A vector of strings representing additional paths to be added to the shell's PATH environment variable.
///
/// # Return
///
/// * `Result<(), String>`: On success, returns `Ok(())`. On error, returns `Err(String)` containing the error message.
pub fn create_activation_shell_script(
    file_path: &str,
    idf_path: &str,
    idf_tools_path: &str,
    idf_version: &str,
    export_paths: Vec<String>,
) -> Result<(), String> {
    ensure_path(file_path).map_err(|e| e.to_string())?;
    let mut filename = PathBuf::from(file_path);
    filename.push(format!("activate_idf_{}.sh", idf_version));
    let template = include_str!("./../bash_scripts/activate_idf_template.sh");
    let mut tera = Tera::default();
    if let Err(e) = tera.add_raw_template("activate_idf_template", template) {
        error!("Failed to add template: {}", e);
        return Err(e.to_string());
    }
    let mut context = Context::new();
    context.insert("idf_path", idf_path);
    context.insert("idf_tools_path", idf_tools_path);
    context.insert("idf_version", idf_version);
    context.insert("addition_to_path", &export_paths.join(":"));
    let rendered = match tera.render("activate_idf_template", &context) {
        Err(e) => {
            error!("Failed to render template: {}", e);
            return Err(e.to_string());
        }
        Ok(text) => text,
    };

    create_executable_shell_script(filename.to_str().unwrap(), &rendered)?;
    Ok(())
}

/// Runs a PowerShell script and captures its output.
///
/// # Parameters
///
/// * `script`: A string containing the PowerShell script to be executed.
///
/// # Returns
///
/// * `Ok(String)`: If the PowerShell script executes successfully, the function returns a `Result` containing the script's output as a string.
/// * `Err(Box<dyn std::error::Error>)`: If an error occurs during the execution of the PowerShell script, the function returns a `Result` containing the error.
pub fn run_powershell_script(script: &str) -> Result<String, Box<dyn std::error::Error>> {
    let mut child = Command::new("powershell")
        .args(["-Command", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(script.as_bytes())?;
    }

    let output = child.wait_with_output()?;

    Ok(String::from_utf8(output.stdout)?)
}

/// Creates a PowerShell profile script for the ESP-IDF tools.
///
/// # Parameters
///
/// * `profile_path` - A string representing the path where the PowerShell profile script should be created.
/// * `idf_path` - A string representing the path to the ESP-IDF repository.
/// * `idf_tools_path` - A string representing the path to the ESP-IDF tools directory.
///
/// # Returns
///
/// * `Result<String, std::io::Error>` - On success, returns the path to the created PowerShell profile script.
///   On error, returns an `std::io::Error` indicating the cause of the error.
fn create_powershell_profile(
    profile_path: &str,
    idf_path: &str,
    idf_tools_path: &str,
    export_paths: Vec<String>,
) -> Result<String, std::io::Error> {
    let profile_template = include_str!("./../powershell_scripts/idf_tools_profile_template.ps1");

    let mut tera = Tera::default();
    if let Err(e) = tera.add_raw_template("powershell_profile", profile_template) {
        error!("Failed to add template: {}", e);
        return Err(std::io::Error::new(
            std::io::ErrorKind::Other,
            "Failed to add template",
        ));
    }
    ensure_path(profile_path).expect("Unable to create directory");
    let mut context = Context::new();
    context.insert("idf_path", idf_path);
    context.insert("idf_tools_path", idf_tools_path);
    context.insert("add_paths_extras", &export_paths.join(";"));
    let rendered = match tera.render("powershell_profile", &context) {
        Err(e) => {
            error!("Failed to render template: {}", e);
            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                "Failed to render template",
            ));
        }
        Ok(text) => text,
    };
    let mut filename = PathBuf::from(profile_path);
    filename.push("Microsoft.PowerShell_profile.ps1");
    fs::write(&filename, rendered).expect("Unable to write file");
    Ok(filename.display().to_string())
}

/// Creates a desktop shortcut for the IDF tools using PowerShell on Windows.
///
/// # Parameters
///
/// * `idf_path` - A string representing the path to the ESP-IDF repository.
/// * `idf_tools_path` - A string representing the path to the IDF tools directory.
///
/// # Return Value
///
/// * `Result<String, std::io::Error>` - On success, returns a string indicating the output of the PowerShell script.
///   On error, returns an `std::io::Error` indicating the cause of the error.
pub fn create_desktop_shortcut(
    profile_path: &str,
    idf_path: &str,
    name: &str,
    idf_tools_path: &str,
    export_paths: Vec<String>,
) -> Result<String, std::io::Error> {
    match std::env::consts::OS {
        "windows" => {
            let filename = match create_powershell_profile(
                profile_path,
                idf_path,
                idf_tools_path,
                export_paths,
            ) {
                Ok(filename) => filename,
                Err(err) => {
                    error!("Failed to create PowerShell profile: {}", err);
                    return Err(err);
                }
            };
            let powershell_script_template =
                include_str!("./../powershell_scripts/create_desktop_shortcut_template.ps1");
            // Create a new Tera instance
            let mut tera = Tera::default();
            if let Err(e) = tera.add_raw_template("powershell_script", powershell_script_template) {
                error!("Failed to add template: {}", e);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    "Failed to add template",
                ));
            }
            let mut context = Context::new();
            context.insert("custom_profile_filename", &filename);
            context.insert("name", &name);
            let rendered = match tera.render("powershell_script", &context) {
                Err(e) => {
                    error!("Failed to render template: {}", e);
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Failed to render template",
                    ));
                }
                Ok(text) => text,
            };

            let output = match run_powershell_script(&rendered) {
                Ok(o) => o,
                Err(err) => {
                    error!("Failed to execute PowerShell script: {}", err);
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        "Failed to execute PowerShell script",
                    ));
                }
            };

            Ok(output)
        }
        _ => {
            warn!("Creating desktop shortcut is only supported on Windows.");
            Ok("Unimplemented on this platform.".to_string())
        }
    }
}

/// Retrieves the path to the local data directory for storing logs.
///
/// This function uses the `dirs` crate to find the appropriate directory for storing logs.
/// If the local data directory is found, it creates a subdirectory named "logs" within it.
/// If the directory creation fails, it returns an error.
///
/// # Returns
///
/// * `Some(PathBuf)` if the local data directory and log directory are successfully created.
/// * `None` if the local data directory cannot be determined.
///
pub fn get_log_directory() -> Option<PathBuf> {
    // Use the dirs crate to find the local data directory
    dirs::data_local_dir().map(|data_dir| {
        // Create a subdirectory named "logs" within the local data directory
        let log_dir = data_dir.join("eim").join("logs");

        // Attempt to create the log directory
        std::fs::create_dir_all(&log_dir).expect("Failed to create log directory");

        // Return the path to the log directory
        log_dir
    })
}
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
/// use std::io;
/// use std::io::Write;
/// use idf_im_lib::download_file;
///
/// fn download_progress_callback(downloaded: u64, total: u64) {
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
/// use idf_im_lib::decompress_archive;
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
/// use idf_im_lib::add_path_to_path;
///
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

/// Messages that can be sent to update the progress bar.
pub enum ProgressMessage {
    /// Update the progress bar with the given value.
    Update(u64),
    /// Finish the progress bar.
    Finish,
}

/// Performs a shallow clone of a Git repository.
///
/// # Arguments
///
/// * `url` - A string representing the URL of the Git repository to clone.
/// * `path` - A string representing the local path where the repository should be cloned.
/// * `branch` - An optional string representing the branch to checkout after cloning. If `None`, the default branch will be checked out.
/// * `tag` - An optional string representing the tag to checkout after cloning. If `None`, the repository will be cloned at the specified branch.
/// * `tx` - A channel sender for progress reporting.
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
    tx: std::sync::mpsc::Sender<ProgressMessage>,
    recurse_submodules: bool,
) -> Result<Repository, git2::Error> {
    // Initialize fetch options with depth 1 for shallow cloning
    let mut fo = FetchOptions::new();
    if tag.is_none() {
        fo.depth(1);
    }

    // Set up remote callbacks for progress reporting
    let mut callbacks = RemoteCallbacks::new();
    callbacks.transfer_progress(|stats| {
        let val =
            ((stats.received_objects() as f64) / (stats.total_objects() as f64) * 100.0) as u64;
        tx.send(ProgressMessage::Update(val)).unwrap();
        true
    });
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

    if (recurse_submodules) {
        let mut sfo = FetchOptions::new();
        let mut callbacks = RemoteCallbacks::new();
        info!("Fetching submodules");
        callbacks.transfer_progress(|stats| {
            let val =
                ((stats.received_objects() as f64) / (stats.total_objects() as f64) * 100.0) as u64;
            tx.send(ProgressMessage::Update(val)).unwrap();
            true
        });
        sfo.remote_callbacks(callbacks);
        tx.send(ProgressMessage::Finish).unwrap();
        update_submodules(&repo, sfo, tx.clone())?;
        info!("Finished fetching submodules");
    }
    // Return the opened repository
    Ok(repo)
}

/// Updates submodules in the given repository using the provided fetch options.//+
/////+
/// # Parameters//+
/////+
/// * `repo`: A reference to the `git2::Repository` object representing the repository.//+
/// * `fetch_options`: A `git2::FetchOptions` object containing the fetch options to be used.//+
/// * `tx`: A `std::sync::mpsc::Sender<ProgressMessage>` object for sending progress messages.//+
/////+
/// # Returns//+
/////+
/// * `Result<(), git2::Error>`: On success, returns `Ok(())`. On error, returns a `git2::Error` indicating the cause of the error.//+
fn update_submodules(
    repo: &Repository,
    fetch_options: FetchOptions,
    tx: std::sync::mpsc::Sender<ProgressMessage>,
) -> Result<(), git2::Error> {
    let mut submodule_update_options = git2::SubmoduleUpdateOptions::new();
    submodule_update_options.fetch(fetch_options);

    fn update_submodules_recursive(
        repo: &Repository,
        path: &Path,
        fetch_options: &mut SubmoduleUpdateOptions,
        tx: std::sync::mpsc::Sender<ProgressMessage>,
    ) -> Result<(), git2::Error> {
        let submodules = repo.submodules()?;
        for mut submodule in submodules {
            tx.send(ProgressMessage::Finish).unwrap();
            submodule.update(true, Some(fetch_options))?;
            let sub_repo = submodule.open()?;
            update_submodules_recursive(
                &sub_repo,
                &path.join(submodule.path()),
                fetch_options,
                tx.clone(),
            )?;
        }
        Ok(())
    }

    update_submodules_recursive(
        repo,
        repo.workdir().unwrap(),
        &mut submodule_update_options,
        tx.clone(),
    )
}

// This function is not used right now  because of limited scope of the POC
// It gets specific fork of rustpython with build in libraries needed for IDF
pub fn get_rustpython_fork(
    custom_path: &str,
    tx: std::sync::mpsc::Sender<ProgressMessage>,
) -> Result<String, git2::Error> {
    let output = shallow_clone(
        "https://github.com/Hahihula/RustPython.git",
        custom_path,
        Some("test-rust-build"),
        None,
        tx,
        false,
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

/// Clones the ESP-IDF repository from the specified URL, tag, or branch,
/// using the provided progress function for reporting cloning progress.
///
/// # Parameters
///
/// * `custom_path`: A string representing the local path where the repository should be cloned.
/// * `tag`: An optional string representing the tag to checkout after cloning. If `None`, the repository will be cloned at the specified branch.
/// * `progress_function`: A closure or function that will be called to report progress during the cloning process.
/// * `mirror`: An optional string representing the URL of a mirror to use for cloning the repository. If `None`, the default GitHub URL will be used.
/// * `group_name`: An optional string representing the group name for the repository. If `None`, the default group name "espressif" will be used.
///
/// # Returns
///
/// * `Result<String, git2::Error>`: On success, returns a `Result` containing the path of the cloned repository as a string.
///   On error, returns a `Result` containing a `git2::Error` indicating the cause of the error.
///
///

pub fn get_esp_idf_by_tag_name(
    custom_path: &str,
    tag: Option<&str>,
    tx: std::sync::mpsc::Sender<ProgressMessage>,
    mirror: Option<&str>,
    group_name: Option<&str>,
    with_submodules: bool,
) -> Result<String, git2::Error> {
    let group = group_name.unwrap_or("espressif");
    let url = match mirror {
        Some(url) => {
            format!("https://github.com/{}/esp-idf.git", group).replace("https://github.com", url)
        }
        None => "https://github.com/espressif/esp-idf.git".to_string(),
    };

    let _ = ensure_path(custom_path);
    let output = match tag {
        Some(tag) => shallow_clone(&url, custom_path, None, Some(tag), tx, with_submodules),
        None => shallow_clone(&url, custom_path, Some("master"), None, tx, with_submodules),
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
