use std::path::{Path, PathBuf};

use log::error;

use idf_env::driver::{self, install_driver};

use crate::{decompress_archive, download_file, verify_file_checksum};

#[derive(Debug, Default, Clone)]

pub struct Driver {
    url: &'static str,
    name: &'static str,
    file_name: &'static str,
    sha256: &'static str,
    install_file_name: &'static str,
}

pub fn get_drivers_list() -> Vec<Driver> {
    // TODO: maintaing the hardcoded sha somewhere besides the downloads
    [
    Driver {
        url: "https://www.silabs.com/documents/public/software/CP210x_Universal_Windows_Driver.zip",
        name: "silabs",
        file_name: "cp210x.zip",
        sha256: "414345bda1b0149f5daa567abdfa71e6d1a4405b7e0302bbc0dc46319fa154ab",
        install_file_name: "silabser.inf",
    },
    Driver {
        url: "https://www.ftdichip.com/Driver/CDM/CDM%20v2.12.28%20WHQL%20Certified.zip",
        name: "ftdi",
        file_name: "ftdi.zip",
        sha256: "82db36f089d391f194c8ad6494b0bf44c508b176f9d3302777c041dad1ef7fe6",
        install_file_name:"ftdiport.inf",
    },
    Driver {
        url: "https://dl.espressif.com/dl/idf-driver/idf-driver-esp32-usb-jtag-2021-07-15.zip",
        name: "espressif",
        file_name: "idf-driver-esp32-usb-jtag-2021-07-15.zip",
        sha256: "84e741dbec5526e3152bded421b4f06f990cd2d1d7e83b907c40e81f9db0f30e",
        install_file_name:"usb_jtag_debug_unit.inf",
    },
    Driver {
        url: "https://www.wch.cn/downloads/file/314.html",
        name: "wch",
        file_name: "whc-ch343ser.zip",
        sha256: "f57328f58769899aecda4b4192a8c288ab3bfd2198f1e157f4ef14a1b6020b35",
        install_file_name:"CH343SER/Driver/CH343SER.INF",
    },
  ].to_vec()
}

pub async fn donwload_drivers(
    progress_function: &dyn Fn(u64, u64),
    drivers: Vec<Driver>,
    download_dir: &str,
) {
    for driver in drivers {
        println!("Downloading {}...", driver.name);
        let mut file = PathBuf::new();
        file.push(download_dir);
        file.push(driver.file_name);
        // let download_path = format!("{}/{}", download_dir, driver.name);
        match verify_file_checksum(driver.sha256, file.to_str().unwrap()) {
            Ok(true) => {
                println!("Checksum matched for {}, skipping download.", driver.name);
                continue;
            }
            Ok(false) => {
                println!(
                    "Checksum did not match for {}, downloading again.",
                    driver.name
                );
            }
            Err(e) => {
                error!("Error verifying checksum for {}: {}", driver.name, e);
                continue;
            }
        }
        match download_file(
            &driver.url,
            download_dir,
            &progress_function,
            Some(driver.file_name),
        )
        .await
        {
            Ok(_) => {
                println!("Download of {} completed successfully.", driver.name);
            }
            Err(e) => {
                error!("Error downloading {}: {}", driver.name, e);
                continue;
            }
        }
        let mut decompress_folder = PathBuf::new();
        decompress_folder.push(download_dir);
        decompress_folder.push(driver.name);
        decompress_archive(file.to_str().unwrap(), decompress_folder.to_str().unwrap()).unwrap();
        let mut install_file = PathBuf::new();
        install_file.push(decompress_folder);
        install_file.push(driver.install_file_name);
        install_driver(install_file.to_string_lossy().to_string());
    }
}
