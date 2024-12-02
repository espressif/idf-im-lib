use crate::{command_executor::execute_command, idf_tools::read_and_parse_tools_file};
use rust_search::SearchBuilder;
#[cfg(not(windows))]
use std::os::unix::fs::MetadataExt;
use std::{
    collections::HashSet,
    fs, io,
    path::{Path, PathBuf},
};
/// This function retrieves the path to the git executable.
///
/// # Purpose
///
/// The function attempts to locate the git executable by checking the system's PATH environment variable.
/// It uses the appropriate command ("where" on Windows, "which" on Unix-like systems) to find the git executable.
///
/// # Parameters
///
/// There are no parameters for this function.
///
/// # Return Value
///
/// - `Ok(String)`: If the git executable is found, the function returns a `Result` containing the path to the git executable as a `String`.
/// - `Err(String)`: If the git executable is not found or an error occurs during the process of locating the git executable, the function returns a `Result` containing an error message as a `String`.
pub fn get_git_path() -> Result<String, String> {
    let cmd = match std::env::consts::OS {
        "windows" => "where",
        _ => "which",
    };

    let output = execute_command(cmd, &vec!["git"]).expect("failed to execute process");

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(stderr.trim().to_string())
    }
}
// Finds all directories in the specified path that match the given name.
// The function recursively searches subdirectories and collects matching paths in a vector.
// Returns a vector of PathBuf containing the paths of matching directories.
pub fn find_directories_by_name(path: &Path, name: &str) -> Vec<String> {
    let search: Vec<String> = SearchBuilder::default()
        .location(path)
        .search_input(name)
        // .limit(1000) // results to return
        .strict()
        // .depth(1)
        .ignore_case()
        .hidden()
        .build()
        .collect();
    filter_subpaths(search)
}

/// Checks if the given path is a valid ESP-IDF directory.
///
/// # Purpose
///
/// This function verifies if the specified directory contains a valid ESP-IDF setup by checking for the existence of the "tools.json" file in the "tools" subdirectory.
///
/// # Parameters
///
/// - `path`: A reference to a string representing the path to be checked.
///
/// # Return Value
///
/// - `bool`: Returns `true` if the specified path is a valid ESP-IDF directory, and `false` otherwise.
pub fn is_valid_idf_directory(path: &str) -> bool {
    let path = PathBuf::from(path);
    let tools_path = path.join("tools");
    let tools_json_path = tools_path.join("tools.json");
    if !tools_json_path.exists() {
        return false;
    }
    match read_and_parse_tools_file(tools_json_path.to_str().unwrap()) {
        Ok(_) => {
            return true;
        }
        Err(_) => {
            return false;
        }
    }
}

/// Filters out duplicate paths from a vector of strings.
///
/// This function checks for duplicate paths in the input vector and removes them.
/// It uses different strategies based on the operating system:
/// - On Windows, it compares the modification time and size of each file to identify duplicates.
/// - On Unix-like systems, it uses the device ID and inode number to identify duplicates.
///
/// # Parameters
///
/// - `paths`: A vector of strings representing file paths.
///
/// # Return Value
///
/// - A vector of strings containing the unique paths from the input vector.
pub fn filter_duplicate_paths(paths: Vec<String>) -> Vec<String> {
    let mut result = Vec::new();
    match std::env::consts::OS {
        "windows" => {
            let mut seen = HashSet::new();
            for path in paths {
                if let Ok(metadata) = fs::metadata(&path) {
                    let key = format!("{:?}-{:?}", metadata.modified().ok(), metadata.len());

                    if seen.insert(key) {
                        result.push(path);
                    }
                } else {
                    result.push(path);
                }
            }
        }
        _ => {
            #[cfg(not(windows))]
            let mut seen = HashSet::new();
            #[cfg(not(windows))]
            for path in paths {
                // Get the metadata for the path
                if let Ok(metadata) = fs::metadata(&path) {
                    // Create a tuple of device ID and inode number
                    let file_id = (metadata.dev(), metadata.ino());

                    // Only keep the path if we haven't seen this file_id before
                    if seen.insert(file_id) {
                        result.push(path);
                    }
                } else {
                    // If we can't get metadata, keep the original path
                    result.push(path);
                }
            }
        }
    }

    result
}

/// Filters out subpaths from a vector of strings.
///
/// This function checks for subpaths in the input vector and removes them.
/// It ensures that only the highest-level paths are retained.
///
/// # Parameters
///
/// - `paths`: A vector of strings representing file paths.
///
/// # Return Value
///
/// - A vector of strings containing the highest-level paths from the input vector.
///   Subpaths are removed, and only the highest-level paths are retained.
fn filter_subpaths(paths: Vec<String>) -> Vec<String> {
    let mut filtered = Vec::new();

    'outer: for path in paths {
        // Check if this path is a subpath of any already filtered path
        for other in &filtered {
            if path.starts_with(other) {
                continue 'outer;
            }
        }

        // Remove any previously added paths that are subpaths of this one
        filtered.retain(|other: &String| !other.starts_with(&path));

        // Add this path
        filtered.push(path);
    }

    filtered
}

/// Removes a directory and all its contents recursively.
///
/// This function attempts to remove a directory and all its contents, including subdirectories and files.
/// It handles cases where the directory or files are read-only on Windows.
///
/// # Parameters
///
/// - `path`: A reference to a type that implements the `AsRef<Path>` trait, representing the path to the directory to be removed.
///
/// # Return Value
///
/// - `io::Result<()>`: If the directory and its contents are successfully removed, the function returns `Ok(())`.
///   If an error occurs during the process, the function returns an `io::Error` containing the specific error details.
pub fn remove_directory_all<P: AsRef<Path>>(path: P) -> io::Result<()> {
    let path = path.as_ref();

    if !path.exists() {
        return Ok(());
    }

    // First ensure all contents are writable to handle readonly files
    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                fs::remove_dir_all(&path)?;
            } else {
                // On Windows, we need to ensure the file is writable before removal
                #[cfg(windows)]
                {
                    let metadata = fs::metadata(&path)?;
                    let mut permissions = metadata.permissions();
                    permissions.set_readonly(false);
                    fs::set_permissions(&path, permissions)?;
                }
                fs::remove_file(&path)?;
            }
        }
    }

    // Now remove the directory itself
    if path.is_dir() {
        fs::remove_dir_all(path)?;
    } else {
        fs::remove_file(path)?;
    }

    Ok(())
}
