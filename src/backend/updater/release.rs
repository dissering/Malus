use super::install_kind::InstallKind;
use super::version::Version;
use crate::backend::req_agent::api_agent;
use anyhow::Context;
use serde::Deserialize;

const RELEASES_URL: &str = "https://api.github.com/repos/dissering/Malus/releases/latest";
const DOWNLOAD_ROOT: &str = "https://github.com/dissering/Malus/releases/download";

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    draft: bool,
    prerelease: bool,
    assets: Vec<Asset>,
}

#[derive(Deserialize)]
struct Asset {
    name: String,
    browser_download_url: String,
    state: String,
    digest: Option<String>,
}

pub struct Update {
    pub version: String,
    pub download: Option<VerifiedDownload>,
}

pub struct VerifiedDownload {
    pub url: String,
    pub sha256: String,
}

pub fn newest_after(current: &str, install_kind: &InstallKind) -> anyhow::Result<Option<Update>> {
    let release: Release = api_agent()
        .get(RELEASES_URL)
        .header("Accept", "application/vnd.github+json")
        .call()
        .and_then(|mut response| response.body_mut().read_json())
        .context("Couldn't check GitHub for the latest Malus release")?;
    Ok(update_from(release, current, install_kind))
}

fn update_from(release: Release, current: &str, install_kind: &InstallKind) -> Option<Update> {
    let tag = &release.tag_name;
    let newer = Version::parse(tag)?;
    if release.draft
        || release.prerelease
        || newer.is_prerelease()
        || !tag
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b".-".contains(&byte))
        || newer <= Version::parse(current)?
    {
        return None;
    }

    let version = tag.strip_prefix('v').unwrap_or(tag).to_owned();
    let download = match install_kind {
        InstallKind::AppImage(_) => verified_asset(&release, &version, "AppImage"),
        InstallKind::Portable(_) => verified_asset(&release, &version, "tar.gz"),
        InstallKind::Flatpak | InstallKind::Other => None,
    };
    Some(Update { version, download })
}

fn verified_asset(release: &Release, version: &str, extension: &str) -> Option<VerifiedDownload> {
    let tag = &release.tag_name;
    let arch = std::env::consts::ARCH;
    let named_after_tag = format!("Malus-{tag}-{arch}.{extension}");
    let named_after_version = format!("Malus-{version}-{arch}.{extension}");
    let asset = release.assets.iter().find(|asset| {
        (asset.name == named_after_tag || asset.name == named_after_version)
            && asset.state == "uploaded"
            && asset.browser_download_url == format!("{DOWNLOAD_ROOT}/{tag}/{}", asset.name)
    })?;
    let sha256 = asset.digest.as_deref()?.strip_prefix("sha256:")?;
    (sha256.len() == 64 && sha256.bytes().all(|byte| byte.is_ascii_hexdigit())).then(|| {
        VerifiedDownload {
            url: asset.browser_download_url.clone(),
            sha256: sha256.to_ascii_lowercase(),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn release(version: &str, suffix: &str) -> Release {
        let tag = format!("v{version}");
        let name = format!("Malus-{version}-{}.{suffix}", std::env::consts::ARCH);
        Release {
            tag_name: tag.clone(),
            draft: false,
            prerelease: false,
            assets: vec![Asset {
                browser_download_url: format!("{DOWNLOAD_ROOT}/{tag}/{name}"),
                name,
                state: "uploaded".to_owned(),
                digest: Some(format!("sha256:{}", "a".repeat(64))),
            }],
        }
    }

    #[test]
    fn matches_only_the_asset_for_this_installation() {
        let portable = InstallKind::Portable(PathBuf::new());
        assert!(
            update_from(release("0.2.0", "tar.gz"), "0.1.0", &portable)
                .unwrap()
                .download
                .is_some()
        );
        assert!(
            update_from(release("0.2.0", "AppImage"), "0.1.0", &portable)
                .unwrap()
                .download
                .is_none()
        );
        assert!(
            update_from(release("0.2.0", "tar.gz"), "0.1.0", &InstallKind::Flatpak)
                .unwrap()
                .download
                .is_none()
        );
    }

    #[test]
    fn requires_a_new_stable_version_and_matching_github_digest() {
        let portable = InstallKind::Portable(PathBuf::new());
        assert!(update_from(release("0.1.0", "tar.gz"), "0.1.0", &portable).is_none());
        let mut prerelease = release("0.2.0-alpha", "tar.gz");
        prerelease.prerelease = true;
        assert!(update_from(prerelease, "0.1.0", &portable).is_none());
        assert!(update_from(release("0.2.0-alpha", "tar.gz"), "0.1.0", &portable).is_none());
        let mut unsigned = release("0.2.0", "tar.gz");
        unsigned.assets[0].digest = None;
        assert!(
            update_from(unsigned, "0.1.0", &portable)
                .unwrap()
                .download
                .is_none()
        );
        let mut unexpected_origin = release("0.2.0", "tar.gz");
        unexpected_origin.assets[0].browser_download_url =
            "https://example.org/launcher".to_owned();
        assert!(
            update_from(unexpected_origin, "0.1.0", &portable)
                .unwrap()
                .download
                .is_none()
        );
    }
}
