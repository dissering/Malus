use super::background::Progress;
use super::download::{download_file, sha256_of_file};
use super::paths;
use anyhow::{Context, bail, ensure};
use flate2::read::GzDecoder;
use gtk::glib;
use std::fs::{self, File};
use std::path::{Path, PathBuf};

pub const VERSION: &str = "v0.5.0-quadra.3-malus.1";

const SUPPORTED_ARCHITECTURE: &str = "x86_64";
const ARCHIVE_NAME: &str = "Malus-Rosemary-0.5.0-quadra.3-malus.1-x86_64.tar.gz";
const ARCHIVE_URL: &str = concat!(
    "https://github.com/dissering/Malus/releases/download/v0.1.1/",
    "Malus-Rosemary-0.5.0-quadra.3-malus.1-x86_64.tar.gz",
);
const ARCHIVE_SHA256: &str = include_str!("../../packaging/rosemary.sha256");
const EXECUTABLE_INSIDE_ARCHIVE: &str = "Rosemary/bin/rosemary";

pub fn executable() -> PathBuf {
    paths::quadra_rosemary_directory(VERSION).join(EXECUTABLE_INSIDE_ARCHIVE)
}

pub fn installed_version() -> Option<String> {
    executable().is_file().then(|| VERSION.to_owned())
}

pub fn bundled_archive() -> Option<PathBuf> {
    archive_beside(&std::env::current_exe().ok()?).filter(|path| path.is_file())
}

fn archive_beside(executable: &Path) -> Option<PathBuf> {
    let bin = executable.parent().filter(|path| path.ends_with("bin"))?;
    Some(bin.parent()?.join("share/malus/rosemary.tar.gz"))
}

pub fn install(progress: &Progress) -> anyhow::Result<()> {
    ensure!(
        std::env::consts::ARCH == SUPPORTED_ARCHITECTURE,
        "Malus's Rosemary is built for {SUPPORTED_ARCHITECTURE} computers."
    );
    if executable().is_file() {
        return Ok(());
    }

    let archive = locate_archive(progress)?;
    progress.step(format!("Unpacking Rosemary {VERSION}"));
    unpack_archive(&archive, &paths::quadra_rosemary_directory(VERSION))
}

fn locate_archive(progress: &Progress) -> anyhow::Result<PathBuf> {
    let expected = ARCHIVE_SHA256.trim();
    if let Some(bundle) = bundled_archive() {
        progress.step("Verifying the bundled Rosemary");
        match sha256_of_file(&bundle) {
            Ok(checksum) if checksum == expected => return Ok(bundle),
            Ok(_) => eprintln!("The Rosemary archive beside Malus has a different checksum."),
            Err(error) => eprintln!("Couldn't read the Rosemary archive beside Malus: {error:#}"),
        }
    }

    let cached = paths::downloads_directory().join(ARCHIVE_NAME);
    if cached.is_file() && sha256_of_file(&cached).is_ok_and(|checksum| checksum == expected) {
        return Ok(cached);
    }

    progress.step(format!("Downloading Rosemary {VERSION}"));
    download_file(ARCHIVE_URL, &cached, progress)?;
    progress.step("Verifying the Rosemary download");
    let checksum = sha256_of_file(&cached)?;
    if checksum != expected {
        let _ = fs::remove_file(&cached);
        bail!("The Rosemary download failed its SHA-256 check.");
    }
    Ok(cached)
}

fn unpack_archive(archive: &Path, destination: &Path) -> anyhow::Result<()> {
    let unpacking = destination.with_file_name(format!(
        ".rosemary-{}.unpacking",
        glib::uuid_string_random()
    ));
    fs::create_dir_all(&unpacking)?;
    let result = extract_archive(archive, &unpacking).and_then(|()| {
        match fs::rename(&unpacking, destination) {
            Ok(()) => Ok(()),
            Err(_) if destination.join(EXECUTABLE_INSIDE_ARCHIVE).is_file() => Ok(()),
            Err(error) => Err(error).context("Couldn't install Rosemary"),
        }
    });
    let _ = fs::remove_dir_all(&unpacking);
    result
}

fn extract_archive(archive: &Path, unpacking: &Path) -> anyhow::Result<()> {
    let mut tar = tar::Archive::new(GzDecoder::new(File::open(archive)?));
    for entry in tar.entries()? {
        let mut entry = entry?;
        let path = entry.path()?.into_owned();
        ensure!(
            entry
                .unpack_in(unpacking)
                .with_context(|| format!("Couldn't unpack {}", path.display()))?,
            "Refused to unpack {} outside the Rosemary directory.",
            path.display(),
        );
    }
    ensure!(
        crate::utils::is_executable_file(&unpacking.join(EXECUTABLE_INSIDE_ARCHIVE)),
        "The archive has no executable {EXECUTABLE_INSIDE_ARCHIVE}."
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn looks_beside_the_installed_launcher_under_each_package_prefix() {
        for prefix in ["/usr", "/app", "/tmp/Malus-x86_64", "/tmp/.mount_Malus/usr"] {
            let prefix = Path::new(prefix);
            assert_eq!(
                archive_beside(&prefix.join("bin/malus")),
                Some(prefix.join("share/malus/rosemary.tar.gz"))
            );
        }
        assert_eq!(archive_beside(Path::new("/build/target/debug/malus")), None);
    }

    #[test]
    fn leaves_the_active_runtime_untouched_when_unpacking_fails() -> anyhow::Result<()> {
        let parent = std::env::temp_dir().join(glib::uuid_string_random().as_str());
        let version = parent.join(VERSION);
        let executable = version.join(EXECUTABLE_INSIDE_ARCHIVE);
        fs::create_dir_all(executable.parent().unwrap())?;
        let result = (|| -> anyhow::Result<()> {
            fs::write(&executable, b"original")?;
            let archive = parent.join("damaged.tar.gz");
            fs::write(&archive, b"invalid gzip")?;
            assert!(unpack_archive(&archive, &version).is_err());
            assert_eq!(fs::read(&executable)?, b"original");
            assert_eq!(fs::read_dir(&parent)?.count(), 2);
            Ok(())
        })();
        fs::remove_dir_all(parent)?;
        result
    }
}
