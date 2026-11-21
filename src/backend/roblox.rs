use super::background::Progress;
use super::download::{
    download_file, remove_directory_if_present, replace_directory, stop_others_from_writing,
};
use super::req_agent::api_agent;
use super::{fast_flags, paths};
use crate::state::Client;
use anyhow::Context;
use gtk::glib;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::path::{Path, PathBuf};

const LATEST_RELEASE_BASE_URL: &str = "https://clientsettingscdn.roblox.com/v2/client-version";
const DOWNLOAD_BASE_URL: &str = "https://setup.rbxcdn.com/mac";
const INSTALLED_RELEASE_FILE: &str = "release.toml";
const PLAYER_APP: &str = "RobloxPlayer.app";
const LEGACY_PLAYER_APP: &str = "Roblox.app";
const PLAYER_EXECUTABLE: &str = "Contents/MacOS/RobloxPlayer";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RobloxRelease {
    pub version: String,
    pub client_version_upload: String,
}

struct ClientFiles {
    release_channel: &'static str,
    archive_suffix: &'static str,
    app_name: &'static str,
    executable_inside_app: &'static str,
    directory: PathBuf,
}

fn files_of(client: Client) -> ClientFiles {
    match client {
        Client::Player => ClientFiles {
            release_channel: "MacPlayer",
            archive_suffix: "RobloxPlayer.zip",
            app_name: PLAYER_APP,
            executable_inside_app: PLAYER_EXECUTABLE,
            directory: paths::roblox_directory(),
        },
        Client::Studio => ClientFiles {
            release_channel: "MacStudio",
            archive_suffix: "RobloxStudioApp.zip",
            app_name: "RobloxStudio.app",
            executable_inside_app: "Contents/MacOS/RobloxStudio",
            directory: paths::studio_directory(),
        },
    }
}

pub fn app_path(client: Client) -> PathBuf {
    let files = files_of(client);
    files.directory.join(files.app_name)
}

pub fn installed_release(client: Client) -> Option<RobloxRelease> {
    let files = files_of(client);
    if !app_path(client).join(files.executable_inside_app).is_file() {
        return None;
    }
    let release = fs::read_to_string(files.directory.join(INSTALLED_RELEASE_FILE)).ok()?;
    toml::from_str(&release).ok()
}

pub fn latest_release(client: Client) -> anyhow::Result<RobloxRelease> {
    let url = format!(
        "{LATEST_RELEASE_BASE_URL}/{}",
        files_of(client).release_channel
    );
    api_agent()
        .get(&url)
        .call()
        .and_then(|mut response| response.body_mut().read_json())
        .with_context(|| format!("Couldn't check for the latest {} version", client.name()))
}

pub fn install(client: Client, release: &RobloxRelease, progress: &Progress) -> anyhow::Result<()> {
    let files = files_of(client);
    let name = client.name();
    let archive_name = format!("{}-{}", release.client_version_upload, files.archive_suffix);
    let archive = paths::downloads_directory().join(&archive_name);
    let is_update = installed_release(client).is_some();
    if is_update {
        progress.step(format!("Downloading the {name} {} update", release.version));
    } else {
        progress.step(format!("Downloading {name} {}", release.version));
    }
    download_file(&download_url(&archive_name), &archive, progress)?;

    if is_update {
        progress.step(format!("Updating {name} to {}", release.version));
    } else {
        progress.step(format!("Installing {name} {}", release.version));
    }
    let unpacking = files.directory.join(".unpacking");
    remove_directory_if_present(&unpacking)?;
    zip::ZipArchive::new(File::open(&archive)?)
        .and_then(|mut zip| zip.extract(&unpacking))
        .with_context(|| format!("Couldn't unpack the {name} download"))?;
    stop_others_from_writing(&unpacking)?;

    let unpacked_app = downloaded_bundle(client, &files, &unpacking).with_context(|| {
        format!(
            "The {name} download has no usable {} bundle.",
            files.app_name
        )
    })?;
    replace_directory(&unpacked_app, &app_path(client))?;
    remove_directory_if_present(&unpacking)?;
    if client == Client::Player {
        fast_flags::reapply_remembered()?;
    }
    fs::write(
        files.directory.join(INSTALLED_RELEASE_FILE),
        toml::to_string(release)?,
    )?;
    let _ = fs::remove_file(&archive);
    if client == Client::Player {
        archive_old_player_bundle();
    }

    Ok(())
}

pub fn install_or_update(
    client: Client,
    installed: Option<&RobloxRelease>,
    progress: &Progress,
) -> anyhow::Result<()> {
    progress.step(format!("Checking for {} updates", client.name()));
    match latest_release(client) {
        Ok(latest) if Some(&latest) != installed => install(client, &latest, progress),
        Ok(_) => Ok(()),
        Err(error) if installed.is_some() => {
            eprintln!(
                "Starting the installed {} without updating: {error:#}",
                client.name()
            );
            Ok(())
        }
        Err(error) => Err(error),
    }
}

fn downloaded_bundle(client: Client, files: &ClientFiles, unpacking: &Path) -> Option<PathBuf> {
    let canonical = unpacking.join(files.app_name);
    if canonical.join(files.executable_inside_app).is_file() {
        return Some(canonical);
    }
    let legacy = unpacking.join(LEGACY_PLAYER_APP);
    (client == Client::Player && legacy.join(files.executable_inside_app).is_file())
        .then_some(legacy)
}

pub fn archive_old_player_bundle() {
    if installed_release(Client::Player).is_none() {
        return;
    }
    let root = paths::roblox_directory();
    let backups = paths::quadra_data_directory().join("legacy-roblox");
    if let Err(error) = archive_duplicate(&root, &backups) {
        eprintln!("Couldn't archive the older Roblox bundle: {error:#}");
    }
}

fn archive_duplicate(root: &Path, backups: &Path) -> anyhow::Result<()> {
    let legacy = root.join(LEGACY_PLAYER_APP);
    if !fs::symlink_metadata(&legacy).is_ok_and(|info| info.is_dir())
        || !root.join(PLAYER_APP).join(PLAYER_EXECUTABLE).is_file()
    {
        return Ok(());
    }

    let backup = backups.join(glib::uuid_string_random().as_str());
    fs::create_dir_all(backups)?;
    fs::create_dir(&backup)?;
    let moved = fs::rename(&legacy, backup.join(LEGACY_PLAYER_APP));
    if moved.is_err() {
        let _ = fs::remove_dir(&backup);
    }
    moved.with_context(|| {
        format!(
            "Couldn't move {} into {}",
            legacy.display(),
            backup.display()
        )
    })
}

fn download_url(archive_name: &str) -> String {
    match std::env::consts::ARCH {
        "aarch64" => format!("{DOWNLOAD_BASE_URL}/arm64/{archive_name}"),
        _ => format!("{DOWNLOAD_BASE_URL}/{archive_name}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognizes_both_archive_names_and_preserves_a_recoverable_legacy_copy() -> anyhow::Result<()>
    {
        let root =
            std::env::temp_dir().join(format!("tiltframe-bundles-{}", glib::uuid_string_random()));
        fs::create_dir(&root)?;
        let run = || -> anyhow::Result<()> {
            let current = root.join("roblox");
            let backups = root.join("legacy-roblox");
            let files = files_of(Client::Player);
            let legacy = current.join(LEGACY_PLAYER_APP);
            make_bundle(&legacy, "old")?;
            assert_eq!(
                downloaded_bundle(Client::Player, &files, &current),
                Some(legacy.clone())
            );
            assert_eq!(
                downloaded_bundle(Client::Studio, &files_of(Client::Studio), &current),
                None
            );
            archive_duplicate(&current, &backups)?;
            assert!(legacy.is_dir());

            let canonical = current.join(PLAYER_APP);
            make_bundle(&canonical, "current")?;
            assert_eq!(
                downloaded_bundle(Client::Player, &files, &current),
                Some(canonical.clone())
            );
            archive_duplicate(&current, &backups)?;
            archive_duplicate(&current, &backups)?;
            assert!(!legacy.exists());
            assert_eq!(
                fs::read_to_string(canonical.join(PLAYER_EXECUTABLE))?,
                "current"
            );
            let parked = fs::read_dir(&backups)?.collect::<Result<Vec<_>, _>>()?;
            assert_eq!(parked.len(), 1);
            assert_eq!(
                fs::read_to_string(
                    parked[0]
                        .path()
                        .join(LEGACY_PLAYER_APP)
                        .join(PLAYER_EXECUTABLE)
                )?,
                "old"
            );
            Ok(())
        };
        let result = run();
        fs::remove_dir_all(&root)?;
        result
    }

    fn make_bundle(directory: &Path, contents: &str) -> anyhow::Result<()> {
        let executable = directory.join(PLAYER_EXECUTABLE);
        crate::utils::create_parent_directory(&executable)?;
        fs::write(executable, contents)?;
        Ok(())
    }
}
