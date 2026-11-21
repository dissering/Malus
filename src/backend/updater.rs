mod install_kind;
pub mod version;

use install_kind::InstallKind;
use std::path::PathBuf;

pub fn running_appimage() -> Option<PathBuf> {
    match InstallKind::detect() {
        InstallKind::AppImage(path) => Some(path),
        InstallKind::Flatpak | InstallKind::Other => None,
    }
}
