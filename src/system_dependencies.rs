use log::{debug, trace};

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

pub fn get_prequisites() -> Vec<&'static str> {
    match std::env::consts::OS {
        "linux" => vec![
            "git",
            "wget",
            "flex",
            "bison",
            "gperf",
            "cmake",
            "ninja-build",
            "ccache",
            "libffi-dev",
            "libssl-dev",
            "dfu-util",
            "libusb-1.0-0",
        ],
        "windows" => vec!["cmake", "ninja", "git"],
        "macos" => vec!["cmake", "ninja", "dfu-util"],
        _ => vec![],
    }
}

pub fn check_prerequisites() -> Result<Vec<&'static str>, String> {
    let list_of_required_tools = get_prequisites();
    let mut unsatisfied = vec![];
    match std::env::consts::OS {
        "linux" => {
            let package_manager = determine_package_manager();
            match package_manager {
                Some("apt") => {
                    for tool in list_of_required_tools {
                        let output = std::process::Command::new("apt")
                            .arg(format!("list --installed | grep {}", tool))
                            .output();
                        match output {
                            Ok(_) => {}
                            Err(_e) => {
                                unsatisfied.push(tool);
                            }
                        }
                    }
                }
                Some("dpkg") => {
                    for tool in list_of_required_tools {
                        let output = std::process::Command::new("dpkg")
                            .arg(format!("-l | grep {}", tool))
                            .output();
                        match output {
                            Ok(_) => {}
                            Err(_e) => {
                                unsatisfied.push(tool);
                            }
                        }
                    }
                }
                Some("dnf") => {
                    for tool in list_of_required_tools {
                        let output = std::process::Command::new("dnf")
                            .arg(format!("list installed | grep {}", tool))
                            .output();
                        match output {
                            Ok(_) => {}
                            Err(_e) => {
                                unsatisfied.push(tool);
                            }
                        }
                    }
                }
                Some("pacman") => {
                    for tool in list_of_required_tools {
                        let output = std::process::Command::new("pacman")
                            .arg(format!("-Qs | grep {}", tool))
                            .output();
                        match output {
                            Ok(_) => {}
                            Err(_e) => {
                                unsatisfied.push(tool);
                            }
                        }
                    }
                }
                Some("zypper") => {
                    for tool in list_of_required_tools {
                        let output = std::process::Command::new("zypper")
                            .arg(format!("se --installed-only {}", tool))
                            .output();
                        match output {
                            Ok(_) => {}
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
                let output = std::process::Command::new("brew")
                    .arg(format!("list | grep {}", tool))
                    .output();
                match output {
                    Ok(_) => {}
                    Err(_e) => {
                        unsatisfied.push(tool);
                    }
                }
            }
        }
        "windows" => {
            for tool in list_of_required_tools {
                let output = std::process::Command::new(tool)
                    .arg(format!("-- version"))
                    .output();
                match output {
                    Ok(_) => {}
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

// Scoop in windows package manager, we do not need this funcion for other platforms
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

// TODO: do not even compile this for non-windows targets
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
            let _ = std::process::Command::new("powershell")
                .arg("-Command")
                .arg("Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser -Force")
                .output();
            let output = std::process::Command::new("powershell")
                .arg("-Command")
                .env(String::from("PATH"),path_with_scoop )
                .arg("Invoke-Expression (New-Object System.Net.WebClient).DownloadString('https://get.scoop.sh')")
                .output();
            match output {
                Ok(o) => {
                    trace!("{}", String::from_utf8(o.stdout).unwrap());
                    debug!("Successfully installed Scoop package manager. Adding to PATH");
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
            #[cfg(windows)]
            crate::win_tools::add_to_win_path(&path_with_scoop).unwrap();
            let output = std::process::Command::new("powershell")
                .env(String::from("PATH"), path_with_scoop)
                .args(&["-Command", "scoop", "--version"])
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
                            Err(e) => panic!("Failed to install {}: {}", package, e.to_string()),
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
                            Err(e) => panic!("Failed to install {}: {}", package, e.to_string()),
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
                            Err(e) => panic!("Failed to install {}: {}", package, e.to_string()),
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
                            Err(e) => panic!("Failed to install {}: {}", package, e.to_string()),
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
                    Err(e) => panic!("Failed to install {}: {}", package, e.to_string()),
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
                    .env(String::from("PATH"), path_with_scoop)
                    .args(&["-Command", "scoop", "install", &package])
                    .output();
                match output {
                    Ok(o) => {
                        trace!("{}", String::from_utf8(o.stdout).unwrap());
                        debug!("Successfully installed {:?}", package);
                    }
                    Err(e) => panic!("Failed to install {}: {}", package, e.to_string()),
                }
            }
        }
        _ => {
            return Err(format!("Unsupported OS - {}", std::env::consts::OS));
        }
    }
    Ok(())
}
