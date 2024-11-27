use crate::command_executor::execute_command;
use std::{
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
pub fn find_directories_by_name(path: &Path, name: &str) -> Vec<PathBuf> {
    let mut result = Vec::new();

    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if path.file_name().and_then(|n| n.to_str()) == Some(name) {
                    result.push(path.clone());
                } else {
                    result.extend(find_directories_by_name(&path, name));
                }
            }
        }
    }

    result
}

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
