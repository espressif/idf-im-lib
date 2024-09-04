use std::env;

use log::{debug, trace};

/// Determines the package manager installed on the system.
///
/// This function attempts to identify the package manager by executing each
/// listed package manager's version command and checking if the command
/// execution is successful.
///
/// This should be only executed on Linux systems, as package managers on other operating systems
/// are not supported.
///
/// # Returns
///
/// * `Some(&'static str)` - If a package manager is found, returns the name of the package manager.
/// * `None` - If no package manager is found, returns None.
fn determine_package_manager() -> Option<&'static str> {
    let package_managers = vec!["apt", "dpkg", "dnf", "pacman", "zypper"];

    for manager in package_managers {
        let output = std::process::Command::new(manager)
            .arg("--version")
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    return Some(manager);
                }
            }
            Err(_) => continue,
        }
    }

    None
}

/// Returns a hardcoded vector of required tools based on the operating system.
///
/// # Returns
///
/// * `Vec<&'static str>` - A vector of required tools for the current operating system.
pub fn get_prequisites() -> Vec<&'static str> {
    match std::env::consts::OS {
        "linux" => vec![
            "git",
            "cmake",
            "ninja",
            "wget",
            "flex",
            "bison",
            "gperf",
            "ccache",
            "libffi-dev",
            "libssl-dev",
            "dfu-util",
            "libusb-1.0-0",
        ],
        "windows" => vec!["git", "cmake", "ninja"], // temporary added cmake back before solving why it does not install from tools.json
        "macos" => vec!["dfu-util", "cmake", "ninja"],
        _ => vec![],
    }
}

/// Checks the system for the required tools and returns a list of unsatisfied tools.
///
/// This function determines the operating system and package manager, then checks if each required tool is installed.
/// If a tool is not found, it is added to the `unsatisfied` vector and returned.
/// The prerequsites are met when empty vector is returned.
///
/// # Returns
///
/// * `Ok(Vec<&'static str>)` - If the function completes successfully, returns a vector of unsatisfied tools.
/// * `Err(String)` - If an error occurs, returns an error message.
pub fn check_prerequisites() -> Result<Vec<&'static str>, String> {
    let list_of_required_tools = get_prequisites();
    debug!("Checking for prerequisites...");
    debug!("will be checking for : {:?}", list_of_required_tools);
    let mut unsatisfied = vec![];
    match std::env::consts::OS {
        "linux" => {
            let package_manager = determine_package_manager();
            debug!("Detected package manager: {:?}", package_manager);
            match package_manager {
                Some("apt") => {
                    for tool in list_of_required_tools {
                        let output = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(format!("apt list --installed | grep {}", tool))
                            .output();
                        match output {
                            Ok(o) => {
                                if o.status.success() {
                                    debug!("{} is already installed: {:?}", tool, o);
                                } else {
                                    debug!("check for {} failed: {:?}", tool, o);
                                    unsatisfied.push(tool);
                                }
                            }
                            Err(_e) => {
                                unsatisfied.push(tool);
                            }
                        }
                    }
                }
                Some("dpkg") => {
                    for tool in list_of_required_tools {
                        let output = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(format!("dpkg -l | grep {}", tool))
                            .output();
                        match output {
                            Ok(o) => {
                                if o.status.success() {
                                    debug!("{} is already installed: {:?}", tool, o);
                                } else {
                                    debug!("check for {} failed: {:?}", tool, o);
                                    unsatisfied.push(tool);
                                }
                            }
                            Err(_e) => {
                                unsatisfied.push(tool);
                            }
                        }
                    }
                }
                Some("dnf") => {
                    for tool in list_of_required_tools {
                        let output = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(format!("dnf list installed | grep {}", tool))
                            .output();
                        match output {
                            Ok(o) => {
                                if o.status.success() {
                                    debug!("{} is already installed: {:?}", tool, o);
                                } else {
                                    unsatisfied.push(tool);
                                }
                            }
                            Err(_e) => {
                                unsatisfied.push(tool);
                            }
                        }
                    }
                }
                Some("pacman") => {
                    for tool in list_of_required_tools {
                        let output = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(format!("pacman -Qs | grep {}", tool))
                            .output();
                        match output {
                            Ok(o) => {
                                if o.status.success() {
                                    debug!("{} is already installed: {:?}", tool, o);
                                } else {
                                    unsatisfied.push(tool);
                                }
                            }
                            Err(_e) => {
                                unsatisfied.push(tool);
                            }
                        }
                    }
                }
                Some("zypper") => {
                    for tool in list_of_required_tools {
                        let output = std::process::Command::new("sh")
                            .arg("-c")
                            .arg(format!("zypper se --installed-only {}", tool))
                            .output();
                        match output {
                            Ok(o) => {
                                if o.status.success() {
                                    debug!("{} is already installed: {:?}", tool, o);
                                } else {
                                    unsatisfied.push(tool);
                                }
                            }
                            Err(_e) => {
                                unsatisfied.push(tool);
                            }
                        }
                    }
                }
                None => {
                    return Err(format!(
                        "Unsupported package manager - {}",
                        package_manager.unwrap()
                    ));
                }
                _ => {
                    return Err(format!(
                        "Unsupported package manager - {}",
                        package_manager.unwrap()
                    ));
                }
            }
        }
        "macos" => {
            for tool in list_of_required_tools {
                let output = std::process::Command::new("zsh")
                    .arg("-c")
                    .arg(format!("brew list | grep {}", tool))
                    .output();
                match output {
                    Ok(o) => {
                        if o.status.success() {
                            debug!("{} is already installed: {:?}", tool, o);
                        } else {
                            debug!("check for {} failed: {:?}", tool, o);
                            unsatisfied.push(tool);
                        }
                    }
                    Err(_e) => {
                        unsatisfied.push(tool);
                    }
                }
            }
        }
        "windows" => {
            for tool in list_of_required_tools {
                let output = std::process::Command::new("powershell")
                    .arg("-Command")
                    .arg(format!("{} --version", tool))
                    .output();
                match output {
                    Ok(o) => {
                        if o.status.success() {
                            debug!("{} is already installed: {:?}", tool, o);
                        } else {
                            debug!("check for {} failed: {:?}", tool, o);
                            unsatisfied.push(tool);
                        }
                    }
                    Err(_e) => {
                        unsatisfied.push(tool);
                    }
                }
            }
        }
        _ => {
            return Err(format!("Unsupported OS - {}", std::env::consts::OS));
        }
    }
    Ok(unsatisfied)
}

/// Returns the path to the Scoop shims directory.
/// This function is only relevant for Windows systems.
///
/// # Returns
///
/// * `Some(String)` - If the function is executed on a Windows system and the Scoop shims directory is found,
///   the function returns the path to the Scoop shims directory.
/// * `None` - If the function is executed on a non-Windows system or if the Scoop shims directory cannot be found,
///   the function returns None.
fn get_scoop_path() -> Option<String> {
    if std::env::consts::OS == "windows" {
        let home_dir = match dirs::home_dir() {
            Some(d) => d,
            None => {
                debug!("Could not get home directory");
                return None;
            }
        };
        let scoop_shims_path = home_dir.join("scoop").join("shims");
        let path = match std::env::var("PATH") {
            Ok(s) => s,
            Err(_) => {
                debug!("Could not get PATH environment variable");
                return None;
            }
        };
        Some(format!("{};{}", path, scoop_shims_path.to_string_lossy()))
    } else {
        None
    }
}

/// Installs the Scoop package manager on Windows.
///
/// This function is only relevant for Windows systems. It sets the execution policy to RemoteSigned,
/// downloads the Scoop installer script from the official website, and executes it.
///
/// # Returns
///
/// * `Ok(())` - If the Scoop package manager is successfully installed.
/// * `Err(String)` - If an error occurs during the installation process.
fn install_scoop_package_manager() -> Result<(), String> {
    match std::env::consts::OS {
        "windows" => {
            let path_with_scoop = match get_scoop_path() {
                Some(s) => s,
                None => {
                    debug!("Could not get scoop path");
                    return Err(String::from("Could not get scoop path"));
                }
            };
            // add_to_windows_path(&path_with_scoop).unwrap();
            add_to_path(&path_with_scoop).unwrap();
            let _ = std::process::Command::new("powershell")
                .arg("-Command")
                .arg("Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force")
                .output();
            let output = std::process::Command::new("powershell")
                .arg("-Command")
                .arg("Invoke-Expression (New-Object System.Net.WebClient).DownloadString('https://get.scoop.sh')")
                .output();
            match output {
                Ok(o) => {
                    trace!("{}", String::from_utf8(o.stdout).unwrap());
                    debug!("Successfully installed Scoop package manager. Adding to PATH");
                    // #[cfg(windows)]
                    // crate::win_tools::add_to_win_path(&path_with_scoop).unwrap();
                    Ok(())
                }
                Err(e) => Err(e.to_string()),
            }
        }
        _ => {
            // this function should not be called on non-windows platforms
            debug!("Scoop package manager is only supported on Windows. Skipping installation.");
            Err(format!("Unsupported OS - {}", std::env::consts::OS))
        }
    }
}

/// Ensures that the Scoop package manager is installed on Windows.
///
/// This function checks if the Scoop package manager is installed on the system.
/// If it is not installed, the function installs it by setting the execution policy to RemoteSigned,
/// downloading the Scoop installer script from the official website, and executing it.
///
/// # Returns
///
/// * `Ok(())` - If the Scoop package manager is successfully installed.
/// * `Err(String)` - If an error occurs during the installation process.
pub fn ensure_scoop_package_manager() -> Result<(), String> {
    match std::env::consts::OS {
        "windows" => {
            let path_with_scoop = match get_scoop_path() {
                Some(s) => s,
                None => {
                    debug!("Could not get scoop path");
                    return Err(String::from("Could not get scoop path"));
                }
            };
            // #[cfg(windows)]
            // crate::win_tools::add_to_win_path(&path_with_scoop).unwrap();
            // add_to_windows_path(&path_with_scoop).unwrap();
            add_to_path(&path_with_scoop).unwrap();
            let output = std::process::Command::new("powershell")
                .args(["-Command", "scoop", "--version"])
                .output();
            match output {
                Ok(o) => {
                    if o.status.success() {
                        debug!("Scoop package manager is already installed");
                        Ok(())
                    } else {
                        debug!("Installing scoop package manager");
                        install_scoop_package_manager()
                    }
                }
                Err(_) => install_scoop_package_manager(),
            }
        }
        _ => {
            // this function should not be called on non-windows platforms
            debug!("Scoop package manager is only supported on Windows. Skipping installation.");
            Err(format!("Unsupported OS - {}", std::env::consts::OS))
        }
    }
}

/// Installs the required packages based on the operating system.
/// This function actually panics if the required packages install fail.
/// This is to ensure that user actually sees the error and realize which package failed to install.
///
/// # Parameters
///
/// * `packages_list` - A vector of strings representing the names of the packages to be installed.
/// this can be obtained by calling the check_prerequisites() function.
///
/// # Returns
///
/// * `Ok(())` - If the packages are successfully installed.
/// * `Err(String)` - If an error occurs during the installation process.
pub fn install_prerequisites(packages_list: Vec<String>) -> Result<(), String> {
    match std::env::consts::OS {
        "linux" => {
            let package_manager = determine_package_manager();
            match package_manager {
                Some("apt") => {
                    for package in packages_list {
                        let output = std::process::Command::new("sudo")
                            .arg("apt")
                            .arg("install")
                            .arg("-y")
                            .arg(&package)
                            .output();
                        match output {
                            Ok(_) => {
                                debug!("Successfully installed {}", package);
                            }
                            Err(e) => panic!("Failed to install {}: {}", package, e),
                        }
                    }
                }
                Some("dnf") => {
                    for package in packages_list {
                        let output = std::process::Command::new("sudo")
                            .arg("dnf")
                            .arg("install")
                            .arg("-y")
                            .arg(&package)
                            .output();
                        match output {
                            Ok(_) => {
                                debug!("Successfully installed {}", package);
                            }
                            Err(e) => panic!("Failed to install {}: {}", package, e),
                        }
                    }
                }
                Some("pacman") => {
                    for package in packages_list {
                        let output = std::process::Command::new("sudo")
                            .arg("pacman")
                            .arg("-Syu")
                            .arg("--noconfirm")
                            .arg(&package)
                            .output();
                        match output {
                            Ok(_) => {
                                debug!("Successfully installed {}", package);
                            }
                            Err(e) => panic!("Failed to install {}: {}", package, e),
                        }
                    }
                }
                Some("zypper") => {
                    for package in packages_list {
                        let output = std::process::Command::new("sudo")
                            .arg("zypper")
                            .arg("--non-interactive")
                            .arg("install")
                            .arg(&package)
                            .output();
                        match output {
                            Ok(_) => {
                                debug!("Successfully installed {}", package);
                            }
                            Err(e) => panic!("Failed to install {}: {}", package, e),
                        }
                    }
                }
                _ => {
                    return Err(format!(
                        "Unsupported package manager - {}",
                        package_manager.unwrap()
                    ));
                }
            }
        }
        "macos" => {
            for package in packages_list {
                let output = std::process::Command::new("brew")
                    .arg("install")
                    .arg(&package)
                    .output();
                match output {
                    Ok(_) => {
                        debug!("Successfully installed {}", package);
                    }
                    Err(e) => panic!("Failed to install {}: {}", package, e),
                }
            }
        }
        "windows" => {
            ensure_scoop_package_manager()?;
            for package in packages_list {
                let path_with_scoop = match get_scoop_path() {
                    Some(s) => s,
                    None => {
                        debug!("Could not get scoop path");
                        return Err(String::from("Could not get scoop path"));
                    }
                };
                debug!("Installing {} with scoop: {}", package, path_with_scoop);
                let output = std::process::Command::new("powershell")
                    .args(["-Command", "scoop", "install", &package])
                    .output();
                match output {
                    Ok(o) => {
                        trace!("{}", String::from_utf8(o.stdout).unwrap());
                        debug!("Successfully installed {:?}", package);
                    }
                    Err(e) => panic!("Failed to install {}: {}", package, e),
                }
            }
        }
        _ => {
            return Err(format!("Unsupported OS - {}", std::env::consts::OS));
        }
    }
    Ok(())
}

/// Adds a new directory to the system's PATH environment variable.
///
/// This function checks if the new directory is already present in the PATH environment variable.
/// If it is not present, the function appends the new directory to the PATH.
///
/// # Parameters
///
/// * `new_path` - A string representing the path of the new directory to be added to the PATH.
///
/// # Returns
///
/// * `Ok(())` - If the new directory is successfully added to the PATH.
/// * `Err(std::io::Error)` - If an error occurs while trying to add the new directory to the PATH.
fn add_to_path(new_path: &str) -> Result<(), std::io::Error> {
    let binding = env::var_os("PATH").unwrap_or_default();
    let paths = binding.to_str().unwrap();

    if !paths.contains(new_path) {
        let new_path_string = match std::env::consts::OS {
            "windows" => format!("{};{}", new_path, paths),
            _ => format!("{}:{}", new_path, paths),
        };
        env::set_var("PATH", new_path_string);
    }

    Ok(())
}
