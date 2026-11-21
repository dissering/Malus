use crate::utils::is_running_in_flatpak;
use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

const PORTABLE_MARKER: &str = ".malus-portable";
const PORTABLE_MARKER_CONTENTS: &str = "Malus portable\n";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallKind {
    AppImage(PathBuf),
    Flatpak,
    Portable(PathBuf),
    Other,
}

impl InstallKind {
    pub fn detect() -> Self {
        Self::from_environment(
            std::env::var_os("APPIMAGE"),
            is_running_in_flatpak(),
            std::env::current_exe().ok(),
        )
    }

    fn from_environment(
        appimage: Option<OsString>,
        in_flatpak: bool,
        current_exe: Option<PathBuf>,
    ) -> Self {
        if in_flatpak {
            return Self::Flatpak;
        }
        match appimage.filter(|path| !path.is_empty()).map(PathBuf::from) {
            Some(path) if path.is_file() => Self::AppImage(path),
            _ => current_exe
                .filter(|path| is_portable_executable(path))
                .map_or(Self::Other, Self::Portable),
        }
    }

    pub fn update_target(&self) -> Option<&Path> {
        let executable = match self {
            Self::AppImage(path) if inside_appimage() && owned_file(path) => Some(path),
            Self::Portable(path) if is_portable_executable(path) => Some(path),
            Self::AppImage(_) | Self::Flatpak | Self::Portable(_) | Self::Other => None,
        }?;
        executable
            .parent()
            .filter(|parent| {
                fs::metadata(parent).is_ok_and(|metadata| {
                    owned_by_user(&metadata) && metadata.is_dir() && metadata.mode() & 0o200 != 0
                })
            })
            .map(|_| executable.as_path())
    }
}

fn inside_appimage() -> bool {
    let appdir = std::env::var_os("APPDIR")
        .filter(|value| !value.is_empty())
        .and_then(|path| fs::canonicalize(path).ok());
    match (appdir, std::env::current_exe()) {
        (Some(appdir), Ok(executable)) => executable.starts_with(appdir),
        _ => false,
    }
}

fn is_portable_executable(path: &Path) -> bool {
    let Some(bin) = path.parent().filter(|dir| dir.ends_with("bin")) else {
        return false;
    };
    let Some(root) = bin.parent() else {
        return false;
    };
    let marker = root.join(PORTABLE_MARKER);
    path.ends_with("malus")
        && owned_file(path)
        && owned_file(&marker)
        && fs::read_to_string(marker).is_ok_and(|contents| contents == PORTABLE_MARKER_CONTENTS)
}

fn owned_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| owned_by_user(&metadata) && metadata.is_file())
}

fn owned_by_user(metadata: &fs::Metadata) -> bool {
    let user = unsafe { libc::geteuid() };
    user != 0 && metadata.uid() == user && metadata.mode() & 0o022 == 0
}

#[cfg(test)]
mod tests {
    use super::InstallKind;

    #[test]
    fn tells_the_ways_malus_is_installed_apart() {
        let existing = std::env::current_exe().unwrap();
        assert_eq!(
            InstallKind::from_environment(Some(existing.clone().into_os_string()), false, None),
            InstallKind::AppImage(existing.clone())
        );
        assert_eq!(
            InstallKind::from_environment(Some(existing.into_os_string()), true, None),
            InstallKind::Flatpak
        );
        assert_eq!(
            InstallKind::from_environment(None, false, None),
            InstallKind::Other
        );
        assert_eq!(
            InstallKind::from_environment(Some("/nowhere/Malus.AppImage".into()), false, None),
            InstallKind::Other
        );
        assert_eq!(
            InstallKind::from_environment(Some("".into()), false, None),
            InstallKind::Other
        );
    }
}
