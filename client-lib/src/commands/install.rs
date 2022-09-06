use anyhow::{anyhow, Context, Result};
use dcspkg_server::Package;
use flate2::read::GzDecoder;
use flate2::CrcReader;
use reqwest::blocking::get;
use reqwest::Url;
use std::fs::{self, Permissions};
use std::os::unix::fs::{symlink, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;
use tar::Archive;

pub fn install(pkg_name: &str, server_url: impl reqwest::IntoUrl) -> Result<()> {
    let server_url = server_url
        .into_url()
        .context("Could not parse server URL")?;
    //get package data
    let pkg =
        get_pkg_data(pkg_name, &server_url).context("Could not get package data from server")?;

    //create directory to unpack into
    let install_path = PathBuf::from(crate::PKGDIR).join("packages").join(pkg.name);
    fs::create_dir_all(&install_path).context("Could not create install directory for package")?;

    //download, checksum, and decompress into PKGDIR/bin
    download_install_file(pkg_name, pkg.crc, &server_url, &install_path)
        .context("Could not install file")?;

    //run install.sh if exists
    if pkg.has_installer {
        run_install_script(&install_path).context("Could not run install script for file")?;
    }

    if pkg.add_to_path {
        let bin_path = PathBuf::from(crate::PKGDIR).join("bin");
        let exe_path = install_path.join(pkg.executable_path.context(
            "Package is configured to add executable to path, but does not contain an executable.",
        )?);
        create_symlink(&bin_path, &exe_path)
            .context("Could not create symbolic link to package executable")?;
    }

    Ok(())
}

fn get_pkg_data(pkg_name: &str, server_url: &Url) -> Result<Package> {
    let url = server_url
        .join(format!("{}/{}", crate::DATA_ENDPOINT, pkg_name).as_ref())
        .context("Could not parse URL")?;

    log::info!("Downloading data for package {pkg_name} from {url}...");

    //download the package date as an option
    let package: Option<Package> = get(url.as_ref())
        .context("Request failed")?
        .json()
        .context("Could not parse JSON response")?;

    log::info!("Got reponse from {url}");
    log::debug!("Package data: {package:?}");

    //if option empty then err here
    package.ok_or_else(|| anyhow!("Package {pkg_name} does not exist on server"))
}

fn download_install_file(
    pkg_name: &str,
    checksum: u32,
    server_url: &Url,
    install_path: &Path,
) -> Result<()> {
    let url = server_url
        .join(crate::FILE_ENDPOINT)
        .and_then(|url| url.join(&format!("{url}.pkg")))
        .context("Could not parse URL")?;

    log::info!("Downloading compressed package {pkg_name}.pkg from {url}...");

    let response = reqwest::blocking::get(url.clone()).context("Request failed")?;

    log::info!("Got reponse from {url}");
    log::info!("Decompressing and unpacking package...");

    //the content of the response
    let compressed = response
        .text()
        .context("Could not get content of response")?;

    //check the crc value is correct
    let downloaded_checksum = CrcReader::new(compressed.as_bytes()).crc().sum();
    if downloaded_checksum != checksum {
        return Err(anyhow!("Checksum for downloaded file did not match!"));
    }

    //decompress and unarchive the bytes
    let tar = GzDecoder::new(compressed.as_bytes());
    let mut archive = Archive::new(tar);

    //unpack archive
    archive
        .unpack(&install_path)
        .context("Could not unpack archive")?;

    log::info!("Unpacked archive");
    log::debug!("Unpacked into {:?}", &install_path);

    Ok(())
}

fn run_install_script(path: &Path) -> Result<()> {
    //check the script is real
    let script = path.join("install.sh");
    if !script.exists() {
        return Err(anyhow!(
            "We were lied to by the server, install.sh does not exist at {script:?}"
        ));
    }

    log::info!("Got install script at {script:?}");

    //set the scripts perms to allow us to execute it
    fs::set_permissions(&script, Permissions::from_mode(0o764))?;

    log::info!("Executing install script...");
    //spawn a child process executing script
    let mut cmd = Command::new(path)
        .spawn()
        .context("Could not execute install.sh")?;

    //wait for it to finish
    cmd.wait()?;

    log::info!("Install script finished, cleaning up...");
    fs::remove_file(&script).context("Could not remove script")?;
    Ok(())
}

fn create_symlink(bin_path: &Path, exe_path: &Path) -> Result<()> {
    symlink(bin_path, exe_path)?;
    Ok(())
}
