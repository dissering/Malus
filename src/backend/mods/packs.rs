use super::content_files::ContentFiles;
use crate::backend::paths;
use anyhow::{Context, ensure};
use gtk::glib;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Seek};
use std::path::{Component, Path, PathBuf};

const MANIFEST: &str = "pack.json";
const ASSETS: &str = "assets";
const MAX_ENTRIES: usize = 4096;
const MAX_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Serialize, Deserialize)]
pub struct ModPack {
    pub id: String,
    pub name: String,
}

fn directory() -> PathBuf {
    paths::quadra_data_directory().join("mod-packs")
}

pub fn list() -> anyhow::Result<Vec<ModPack>> {
    let entries = match fs::read_dir(directory()) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };
    let mut packs = Vec::new();
    for entry in entries {
        let entry = entry?;
        let id = entry.file_name().to_string_lossy().to_string();
        if !entry.file_type()?.is_dir() || !glib::uuid_string_is_valid(&id) {
            continue;
        }
        match read_pack(&id) {
            Ok(pack) => packs.push(pack),
            Err(error) => eprintln!("Couldn't read mod pack {id}: {error}"),
        }
    }
    packs.sort_by_cached_key(|pack| (pack.name.to_lowercase(), pack.id.clone()));
    Ok(packs)
}

pub fn import(source: &Path) -> anyhow::Result<()> {
    let name = source
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::trim)
        .filter(|stem| !stem.is_empty())
        .unwrap_or("Mod pack")
        .chars()
        .take(80)
        .collect();
    let pack = ModPack {
        id: glib::uuid_string_random().to_string(),
        name,
    };
    let root = directory();
    fs::create_dir_all(&root)?;
    let staging = root.join(format!(".import-{}", pack.id));
    fs::create_dir(&staging)?;
    let result = (|| {
        unpack(File::open(source)?, &staging.join(ASSETS))?;
        write_pack(&staging, &pack)?;
        fs::rename(&staging, root.join(&pack.id))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&staging);
    }
    result
}

fn read_pack(id: &str) -> anyhow::Result<ModPack> {
    ensure!(glib::uuid_string_is_valid(id), "Invalid mod pack ID.");
    let mut pack: ModPack =
        serde_json::from_slice(&fs::read(directory().join(id).join(MANIFEST))?)?;
    pack.id = id.to_owned();
    Ok(pack)
}

fn write_pack(target: &Path, pack: &ModPack) -> anyhow::Result<()> {
    glib::file_set_contents(target.join(MANIFEST), &serde_json::to_vec_pretty(pack)?)?;
    Ok(())
}

fn unpack(reader: impl Read + Seek, target: &Path) -> anyhow::Result<()> {
    let mut zip = zip::ZipArchive::new(reader).context("Couldn't open the mod ZIP")?;
    ensure!(
        zip.len() <= MAX_ENTRIES,
        "The mod ZIP has too many entries."
    );
    let mut written = 0;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        if !entry.is_file() || entry.is_symlink() {
            continue;
        }
        let Some(path) = entry.enclosed_name().and_then(|path| asset_path(&path)) else {
            continue;
        };
        let size = entry.size();
        ensure!(
            size <= MAX_BYTES - written,
            "The mod ZIP exceeds 256 MiB of assets."
        );
        let file = target.join(path);
        crate::utils::create_parent_directory(&file)?;
        let mut output = OpenOptions::new().write(true).create_new(true).open(file)?;
        let copied = io::copy(&mut entry.by_ref().take(size + 1), &mut output)?;
        ensure!(copied == size, "The mod ZIP reported an invalid file size.");
        written += copied;
    }
    ensure!(
        target.exists(),
        "The ZIP contains no supported sounds, textures, or fonts."
    );
    Ok(())
}

fn asset_path(path: &Path) -> Option<PathBuf> {
    if path
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let mut parts = path.iter();
    let first = parts.next()?.to_str()?;
    let relative = if first.eq_ignore_ascii_case("content") {
        parts.as_path()
    } else if parts
        .clone()
        .next()?
        .to_str()?
        .eq_ignore_ascii_case("content")
    {
        parts.next();
        parts.as_path()
    } else {
        path
    };
    let category = relative.iter().next()?.to_str()?.to_ascii_lowercase();
    let extension = relative.extension()?.to_str()?.to_ascii_lowercase();
    let accepted = match category.as_str() {
        "sounds" => matches!(extension.as_str(), "ogg" | "mp3" | "wav" | "flac"),
        "textures" => matches!(
            extension.as_str(),
            "png" | "jpg" | "jpeg" | "dds" | "bmp" | "tga" | "webp"
        ),
        "fonts" => matches!(extension.as_str(), "ttf" | "otf" | "woff" | "woff2"),
        _ => false,
    };
    if !accepted || relative.components().count() < 2 {
        return None;
    }
    let mut normalized = PathBuf::from(category);
    normalized.extend(relative.iter().skip(1));
    Some(normalized)
}

pub fn apply(files: &mut ContentFiles, disabled: &[String]) -> anyhow::Result<()> {
    for pack in list()?
        .into_iter()
        .filter(|pack| !disabled.contains(&pack.id))
    {
        let root = directory().join(pack.id).join(ASSETS);
        if !fs::symlink_metadata(&root).is_ok_and(|info| info.is_dir()) {
            continue;
        }
        let mut folders = vec![root.clone()];
        while let Some(folder) = folders.pop() {
            for entry in fs::read_dir(folder)? {
                let entry = entry?;
                let kind = entry.file_type()?;
                let path = entry.path();
                if kind.is_dir() {
                    folders.push(path);
                } else if kind.is_file()
                    && let Some(relative) = path.strip_prefix(&root).ok().and_then(asset_path)
                {
                    files.place_pack_asset(&relative, &path).with_context(|| {
                        format!("Couldn't apply {} from {}", relative.display(), pack.name)
                    })?;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};

    #[test]
    fn imports_cosmetic_paths_only() -> anyhow::Result<()> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for name in [
            "content/sounds/ouch.ogg",
            "textures/example.png",
            "My Pack/content/fonts/test.ttf",
            "content/sounds/script.lua",
            "../escape.png",
            "Contents/MacOS/RobloxPlayer",
        ] {
            zip.start_file(name, zip::write::SimpleFileOptions::default())?;
            zip.write_all(b"example")?;
        }
        let root =
            std::env::temp_dir().join(format!("malus-mod-test-{}", glib::uuid_string_random()));
        fs::create_dir(&root)?;
        let result = (|| {
            let target = root.join(ASSETS);
            unpack(zip.finish()?, &target)?;
            assert!(target.join("sounds/ouch.ogg").is_file());
            assert!(target.join("textures/example.png").is_file());
            assert!(target.join("fonts/test.ttf").is_file());
            assert!(!target.join("sounds/script.lua").exists());
            assert!(!root.join("escape.png").exists());
            assert!(!target.join("Contents").exists());
            Ok(())
        })();
        fs::remove_dir_all(root)?;
        result
    }
}
