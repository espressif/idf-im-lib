fn determine_package_manager() -> Option<&'static str> {
    let package_managers = vec!["dpkg", "rpm", "pacman", "zypper"];

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

// should not print on screen but return and let the output be made by cli or gui
pub fn check_prerequisites() -> Result<Vec<&'static str>, String> {
    let list_of_required_tools = get_prequisites();
    let mut unsatisfied = vec![];
    match std::env::consts::OS {
        "linux" => {
            let package_manager = determine_package_manager();
            match package_manager {
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
                Some("rpm") => {
                    for tool in list_of_required_tools {
                        let output = std::process::Command::new("rpm")
                            .arg(format!("-qa | grep {}", tool))
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
