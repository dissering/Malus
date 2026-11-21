use super::install_kind::InstallKind;
use super::release::VerifiedDownload;
use crate::backend::background::Progress;
use crate::backend::download::{download_file, sha256_of_file};
use anyhow::{Context, ensure};
use flate2::read::GzDecoder;
use gtk::glib;
use std::fs::{self, DirBuilder, File, Permissions};
use std::io::{self, Read};
use std::os::unix::fs::{DirBuilderExt, PermissionsExt};
use std::path::{Path, PathBuf};

const MAX_PORTABLE_EXE_BYTES: u64 = 128 * 1024 * 1024;

pub fn stage(
    kind: &InstallKind,
    download: &VerifiedDownload,
    progress: &Progress,
) -> anyhow::Result<StagedUpdate> {
    let target = kind
        .update_target()
        .context("This installation cannot be updated in place")?;
    let parent = target
        .parent()
        .context("Couldn't locate the launcher folder")?;

    let staging_path = parent.join(format!(".malus-update-{}", glib::uuid_string_random()));
    DirBuilder::new()
        .mode(0o700)
        .create(&staging_path)
        .context("Couldn't create a staging directory beside the launcher")?;
    let staging = StagingDirectory(staging_path);
    let downloaded = staging.0.join("download");
    progress.step("Downloading the Malus update");
    download_file(&download.url, &downloaded, progress)?;

    progress.step("Verifying the Malus update");
    ensure!(
        sha256_of_file(&downloaded)? == download.sha256,
        "The Malus download failed its SHA-256 check"
    );

    let executable = if matches!(kind, InstallKind::Portable(_)) {
        let executable = staging.0.join("malus");
        unpack_executable(&downloaded, &executable)?;
        executable
    } else {
        downloaded
    };

    let mut file = File::open(&executable)?;
    let mut header = [0; 11];
    file.read_exact(&mut header)?;
    ensure!(
        &header[..4] == b"\x7fELF",
        "The Malus download is not a Linux executable"
    );
    if matches!(kind, InstallKind::AppImage(_)) {
        ensure!(
            &header[8..] == b"AI\x02",
            "The Malus download is not an AppImage"
        );
    }
    file.set_permissions(Permissions::from_mode(0o755))?;
    file.sync_all()?;
    Ok(StagedUpdate {
        _staging: staging,
        executable,
    })
}

fn unpack_executable(downloaded: &Path, output: &Path) -> anyhow::Result<()> {
    let expected_path = format!("Malus-{}/bin/malus", std::env::consts::ARCH);
    let mut archive = tar::Archive::new(GzDecoder::new(File::open(downloaded)?));
    let entries = archive.entries()?;
    for entry in entries {
        let mut entry = entry?;
        if entry.path()?.as_ref() != Path::new(&expected_path) {
            continue;
        }
        ensure!(
            entry.header().entry_type().is_file() && entry.size() <= MAX_PORTABLE_EXE_BYTES,
            "The portable update contains an invalid executable"
        );
        io::copy(&mut entry, &mut File::create(output)?)?;
        return Ok(());
    }

    anyhow::bail!("The portable update does not contain the Malus executable")
}

struct StagingDirectory(PathBuf);

impl Drop for StagingDirectory {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub struct StagedUpdate {
    _staging: StagingDirectory,
    executable: PathBuf,
}

impl StagedUpdate {
    pub fn commit(self, kind: &InstallKind) -> anyhow::Result<()> {
        let target = kind
            .update_target()
            .context("This installation cannot be updated in place")?;
        fs::rename(&self.executable, target).context("Couldn't replace the Malus executable")
    }
}
