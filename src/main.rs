// Jackson Coxson

use std::{
    fs::File,
    io::{stdin, Read, Seek, Write},
    path::{Path, PathBuf},
    str::FromStr,
    sync::Arc,
};

use dialoguer::{theme::ColorfulTheme, Select};
use plist_plus::PlistType;
use rusty_libimobiledevice::{idevice::Device, services::userpref};
use walkdir::WalkDir;
use zip::{result::ZipError, write::FileOptions};

fn main() {
    _main();
    println!("Press any key to exit...");
    stdin().read_line(&mut String::new()).unwrap();
}

fn _main() {
    println!(
        r#"Welcome to the SideStore downloader.
        You will be guided through the steps of downloading the SideStore .ipa and modifying it for your device.
        Make sure your device is plugged in or connected via network so we can pull the information required from it.
        "#
    );
    #[cfg(target_os = "macos")]
    println!("Make sure to open Finder and ensure that your device is shown on the side bar");
    #[cfg(target_os = "windows")]
    println!("Make sure that iTunes is downloaded and that you've clicked your device at the top. Make sure to allow a connection.");
    #[cfg(target_os = "linux")]
    println!("Make sure that usbmuxd is installed and running. Ubuntu can install it from apt.");

    println!("Step 1/7: Download the SideStore .ipa");
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("We will download SideStore from the latest stable release. Would you like to specify a different URL?")
        .default(0)
        .items(&[ "No (recommended)", "Yes", "Choose a local file"])
        .interact()
        .unwrap();

    let ipa_bytes = if selection == 0 || selection == 1 {
        let url = if selection == 0 {
            "https://github.com/SideStore/SideStore/releases/latest/download/SideStore.ipa"
                .to_string()
        } else {
            println!("Enter the URL (ending in .ipa) for SideStore");
            let mut s = String::new();
            stdin()
                .read_line(&mut s)
                .expect("Did not enter a correct string");

            s.trim().to_string()
        };

        let agent = ureq::AgentBuilder::new()
            .tls_connector(Arc::new(native_tls::TlsConnector::new().unwrap()))
            .build();

        let ipa_bytes = match agent.get(&url).call() {
            Ok(i) => i,
            Err(e) => {
                println!("Could not download from specified URL: {:?}", e);
                return;
            }
        };
        let mut x_vec = Vec::new();
        if ipa_bytes.into_reader().read_to_end(&mut x_vec).is_err() {
            println!("Error getting bytes from URL");
            return;
        }
        x_vec
    } else {
        println!("Enter the path to the SideStore .ipa");
        let mut s = String::new();
        stdin()
            .read_line(&mut s)
            .expect("Did not enter a correct string");

        let path = Path::new(s.trim());
        let mut file = match File::open(path) {
            Ok(f) => f,
            Err(e) => {
                println!("Could not open file: {:?}", e);
                return;
            }
        };
        let mut x_vec = Vec::new();
        if file.read_to_end(&mut x_vec).is_err() {
            println!("Error getting bytes from file");
            return;
        }
        x_vec
    };
    let cursor = std::io::Cursor::new(ipa_bytes);
    let mut archive = match zip::read::ZipArchive::new(cursor) {
        Ok(a) => a,
        Err(e) => {
            println!("Unable to convert download into archive: {:?}", e);
            return;
        }
    };

    println!("\n\nStep 2/7: Choose a directory to save the .ipa");
    println!("Enter the path now (use . for the current directory)");
    let mut s = String::new();
    stdin()
        .read_line(&mut s)
        .expect("Did not enter a correct string");
    if s.trim() == "." {
        // nightmare nightmare nightmare nightmare nightmare nightmare nightmare nightmare nightmare
        s = std::env::current_dir()
            .unwrap()
            .canonicalize()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
    }
    let save_path = match PathBuf::from_str(s.trim()) {
        Ok(t) => t,
        Err(e) => {
            println!("Bad path string: {:?}", e);
            return;
        }
    };
    if !save_path.exists() {
        match std::fs::create_dir_all(&save_path) {
            Ok(_) => (),
            Err(e) => {
                println!("Path not found, creation failed: {:?}", e);
                return;
            }
        }
    }

    println!("\n\nStep 3/7: Choose the device you're creating the .ipa for");
    let device;
    let device_name;
    loop {
        let devices = match rusty_libimobiledevice::idevice::get_devices() {
            Ok(d) => d,
            Err(e) => {
                println!("Could not get the device list from the muxer: {:?}", e);
                #[cfg(target_os = "windows")]
                println!("Make sure you have iTunes installed and that it is running");
                #[cfg(target_os = "linux")]
                println!("Make sure that usbmuxd is running");
                continue;
            }
        };
        if devices.is_empty() {
            println!("No device connected!!");
            std::thread::sleep(std::time::Duration::from_secs(1));
            continue;
        }
        if devices.len() == 1 {
            // That clone might be the end of me
            device = devices[0].clone();
            let lock_cli = match device.new_lockdownd_client("ss_downloader") {
                Ok(l) => l,
                Err(e) => {
                    println!("Unable to start lockdownd client on device: {:?}", e);
                    return;
                }
            };
            let name = match lock_cli.get_device_name() {
                Ok(n) => n,
                Err(e) => {
                    println!("Unable to get device name: {:?}", e);
                    return;
                }
            };
            println!("Using the only connected device: {}", name);
            device_name = name;
            break;
        }
        let mut mp = Vec::with_capacity(devices.len());
        for device in devices {
            let lock_cli = match device.new_lockdownd_client("ss_downloader") {
                Ok(l) => l,
                Err(_) => continue,
            };
            let name = match lock_cli.get_device_name() {
                Ok(n) => n,
                Err(_) => continue,
            };
            // I hate this, but I'm lazy
            mp.push((
                device.clone(),
                format!(
                    "{} ({})",
                    name,
                    if device.get_network() {
                        "Network"
                    } else {
                        "USB"
                    }
                ),
            ));
        }

        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose your device")
            .default(0)
            .items(
                &mp.iter()
                    // Send help
                    .map(|x| x.1.clone())
                    .collect::<Vec<String>>()
                    .to_vec(),
            )
            .interact()
            .unwrap();

        // Talk about emotional damage
        device = mp[selection].0.clone();
        device_name = mp[selection].1.clone();
        break;
    }
    let device_name = device_name
        .replace(' ', "_")
        .replace('(', "")
        .replace(')', "")
        .replace('’', "")
        .replace('\'', "");

    println!("\n\nStep 4/7: Check the pairing file");
    if device.get_network() {
        println!("Device is connected over the network, test skipped");
    } else {
        println!("It is HIGHLY RECOMMENDED to test your device's pairing file.");
        println!("This will ensure that SideStore will be able to use it.");
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Proceed with pairing file test?")
            .default(0)
            .items(&["Yes (HIGHLY RECOMMENDED)", "No"])
            .interact()
            .unwrap();
        match selection {
            0 => {
                println!("Enter the local IP address of your device");
                let mut s = String::new();
                stdin().read_line(&mut s).expect("bad uh oh abort");
                let ip = match std::net::Ipv4Addr::from_str(s.trim()) {
                    Ok(i) => i,
                    Err(e) => {
                        println!("Could not parse input: {:?}", e);
                        return;
                    }
                };

                loop {
                    if test_device(ip, device.get_udid()) {
                        break;
                    }

                    // Unwrapping because we already know this works
                    let lock_cli = device.new_lockdownd_client("ss_downloader_regen").unwrap();
                    if let Err(e) = lock_cli.set_value(
                        "EnableWifiDebugging".to_string(),
                        "com.apple.mobile.wireless_lockdown".to_string(),
                        true.into(),
                    ) {
                        println!("Failed to enable WiFi sync: {:?}", e);
                        println!("Make sure you have a password set.");
                        return;
                    }
                    if test_device(ip, device.get_udid()) {
                        break;
                    }
                    if let Err(e) = lock_cli.pair(None, None) {
                        println!("Failed to pair to your device. {:?}", e);
                        return;
                    }
                }
            }
            1 => {
                println!("Skipping test, there is no guarantee SideStore will work.")
            }
            _ => unreachable!(),
        }
    }

    println!("\n\nStep 5/7: Choose an anisette server");
    let default_anisettes = vec![
        (
            "Sideloadly (recommended for most users)",
            "https://sideloadly.io/anisette/irGb3Quww8zrhgqnzmrx",
        ),
        ("Macley US", "http://191.101.206.188:6969/"),
        ("Macley DE", "http://45.132.246.138:6969/"),
        ("DrPudding US", "https://sign.puddingg.xyz/"),
        ("DrPudding FR", "https://sign.rheaa.xyz/"),
        ("Custom", "custom_todo"),
    ];
    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Choose an anisette server")
        .default(0)
        .items(
            &default_anisettes
                .iter()
                .map(|x| x.0.to_owned())
                .collect::<Vec<String>>()
                .to_vec(),
        )
        .interact()
        .unwrap();
    let mut anisette_url = default_anisettes[selection].1.to_owned();
    if anisette_url.as_str() == "custom_todo" {
        println!("Enter the URL to the anisette server. Be careful, as a malicious server can access your Apple account!");
        let mut s = String::new();
        stdin().read_line(&mut s).unwrap();
        anisette_url = s.trim().to_string();
    }

    println!("\n\nStep 6/7: Extract and modify the .ipa");
    if archive.extract(&save_path.join("temp")).is_err() {
        println!("Unable to extract the archive");
        return;
    }

    let plist_path = save_path
        .join("temp")
        .join("Payload")
        .join("SideStore.app")
        .join("Info.plist");
    if !plist_path.exists() {
        println!("Archive did not contain Info.plist");
        return;
    }

    let mut buf = Vec::new();
    let mut plist_file = std::fs::File::open(&plist_path).unwrap();
    plist_file.read_to_end(&mut buf).unwrap();
    let mut info_plist = match plist_plus::Plist::from_bin(buf[..].to_vec()) {
        Ok(i) => i,
        Err(e) => {
            println!("Failed to read plist from file: {:?}", e);
            return;
        }
    };

    if info_plist.plist_type != PlistType::Dictionary {
        println!("Info.plist was in the incorrect format");
        return;
    }

    info_plist
        .dict_set_item("ALTDeviceID", device.get_udid().into())
        .unwrap();

    let pairing_file = match userpref::read_pair_record(device.get_udid()) {
        Ok(mut p) => {
            p.dict_set_item("UDID", device.get_udid().into()).unwrap();
            p.to_string()
        }
        Err(e) => {
            println!("Failed to read pairing file for device from muxer: {:?}", e);
            return;
        }
    };

    info_plist
        .dict_set_item("ALTPairingFile", pairing_file.into())
        .unwrap();
    info_plist
        .dict_set_item("customAnisetteURL", anisette_url.into())
        .unwrap();

    let info_plist = info_plist.to_string();
    std::fs::remove_file(&plist_path).unwrap();
    let mut f = std::fs::File::create(plist_path).unwrap();
    let _ = f.write(info_plist.as_bytes()).unwrap();

    println!("\n\nStep 7/7: Compress ipa");
    pls_zip(
        save_path.join("temp").to_str().unwrap(),
        save_path
            .join(format!("SideStore-{}.ipa", device_name))
            .to_str()
            .unwrap(),
        zip::CompressionMethod::Deflated,
    )
    .unwrap();
    std::fs::remove_dir_all(save_path.join("temp")).unwrap();

    println!("\n\nDone!! Do not share this .ipa with others, it contains private information for your device.");
}

fn test_device(ip: std::net::Ipv4Addr, udid: String) -> bool {
    let device = Device::new(udid, Some(std::net::IpAddr::V4(ip)), 696969);
    match device.new_heartbeat_client("ss_downloader_tester") {
        Ok(_) => true,
        Err(e) => {
            println!("Test failed!! {:?}", e);
            false
        }
    }
}

fn zip_dir<T>(
    it: &mut dyn Iterator<Item = walkdir::DirEntry>,
    prefix: &str,
    writer: T,
    method: zip::CompressionMethod,
) -> zip::result::ZipResult<()>
where
    T: Write + Seek,
{
    let mut zip = zip::ZipWriter::new(writer);
    let options = FileOptions::default()
        .compression_method(method)
        .unix_permissions(0o755);

    let mut buffer = Vec::new();
    for entry in it {
        let path = entry.path();
        let name = path.strip_prefix(Path::new(prefix)).unwrap();

        // Write file or directory explicitly
        // Some unzip tools unzip files with directory paths correctly, some do not!
        if path.is_file() {
            #[allow(deprecated)]
            zip.start_file_from_path(name, options)?;
            let mut f = File::open(path)?;

            f.read_to_end(&mut buffer)?;
            zip.write_all(&*buffer)?;
            buffer.clear();
        } else if !name.as_os_str().is_empty() {
            // Only if not root! Avoids path spec / warning
            // and mapname conversion failed error on unzip
            #[allow(deprecated)]
            zip.add_directory_from_path(name, options)?;
        }
    }
    zip.finish()?;
    Result::Ok(())
}

fn pls_zip(
    src_dir: &str,
    dst_file: &str,
    method: zip::CompressionMethod,
) -> zip::result::ZipResult<()> {
    if !Path::new(src_dir).is_dir() {
        return Err(ZipError::FileNotFound);
    }

    let path = Path::new(dst_file);
    let file = File::create(&path).unwrap();

    let walkdir = WalkDir::new(src_dir);
    let it = walkdir.into_iter();

    zip_dir(&mut it.filter_map(|e| e.ok()), src_dir, file, method)?;

    Ok(())
}
