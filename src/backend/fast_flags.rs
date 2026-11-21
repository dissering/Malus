use super::{paths, roblox};
use crate::state::{AppState, Client, FastFlagValue, FastFlags};
use anyhow::Context;
use gtk::glib;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const CLIENT_SETTINGS_INSIDE_APP: &str = "Contents/MacOS/ClientSettings/ClientAppSettings.json";

pub fn initialize(state: &AppState) {
    state.follow_properties(&["fast-flags"], |state| {
        if let Err(error) = remember(&state.fast_flags()) {
            eprintln!("{error:#}");
        }
    });
}

pub fn write_client_settings(flags: &FastFlags) -> anyhow::Result<()> {
    write_if_changed(&client_settings_in_app(), &client_settings_json(flags)?)
}

pub fn reapply_remembered() -> anyhow::Result<()> {
    let contents = match fs::read(paths::fast_flags_file()) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).context("Couldn't read the saved FastFlags"),
    };
    write_if_changed(&client_settings_in_app(), &contents)
}

fn remember(flags: &FastFlags) -> anyhow::Result<()> {
    let contents = client_settings_json(flags)?;
    write_if_changed(&paths::fast_flags_file(), &contents)?;
    if roblox::installed_release(Client::Player).is_some() {
        write_if_changed(&client_settings_in_app(), &contents)?;
    }
    Ok(())
}

fn client_settings_in_app() -> PathBuf {
    roblox::app_path(Client::Player).join(CLIENT_SETTINGS_INSIDE_APP)
}

fn client_settings_json(flags: &FastFlags) -> anyhow::Result<Vec<u8>> {
    let settings: BTreeMap<&str, String> = flags
        .0
        .iter()
        .map(|(name, value)| (name.as_str(), client_setting_value(value)))
        .collect();
    Ok(serde_json::to_vec_pretty(&settings)?)
}

fn write_if_changed(path: &Path, contents: &[u8]) -> anyhow::Result<()> {
    if fs::read(path).is_ok_and(|current| current == contents) {
        return Ok(());
    }
    crate::utils::create_parent_directory(path)?;
    glib::file_set_contents(path, contents)
        .with_context(|| format!("Couldn't write the FastFlags to {}", path.display()))
}

fn client_setting_value(value: &FastFlagValue) -> String {
    match value {
        FastFlagValue::Toggle(true) => "True".to_owned(),
        FastFlagValue::Toggle(false) => "False".to_owned(),
        _ => value.as_text(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restores_missing_or_externally_modified_settings() -> anyhow::Result<()> {
        let temp = std::env::temp_dir().join(format!("tiltframe-flags-{}", std::process::id()));
        fs::create_dir(&temp)?;
        let file = temp.join("ClientAppSettings.json");
        let result = (|| -> anyhow::Result<()> {
            let flags = FastFlags::default().with("FFlagTest", Some(FastFlagValue::Toggle(true)));
            let contents = client_settings_json(&flags)?;
            write_if_changed(&file, &contents)?;
            fs::write(&file, b"{}")?;
            write_if_changed(&file, &contents)?;
            assert_eq!(fs::read(&file)?, contents);
            fs::remove_file(&file)?;
            write_if_changed(&file, &contents)?;
            assert_eq!(fs::read(&file)?, contents);
            write_if_changed(&file, &client_settings_json(&FastFlags::default())?)?;
            assert_eq!(fs::read(&file)?, b"{}");
            Ok(())
        })();
        fs::remove_dir_all(temp)?;
        result
    }
}
