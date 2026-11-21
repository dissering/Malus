use super::prefix;
use crate::state::AppState;
use anyhow::{Context, bail, ensure};
use gtk::glib;
use std::ffi::{OsStr, OsString};
use std::fs::{self, OpenOptions};
use std::io::{self, ErrorKind};
use std::os::unix::fs::OpenOptionsExt;
use std::path::{Component, Path, PathBuf};

const PLUGINS_PATH: &str = "Documents/Roblox/Plugins";

pub fn directory(state: &AppState) -> anyhow::Result<PathBuf> {
    let prefix = prefix::directory_for(&state.prefix())
        .context("Select the Malus prefix to manage local Studio plugins.")?;
    let variables = state.environment_variables();
    let override_for = |name: &str| {
        variables
            .enabled()
            .filter(|variable| variable.name == name)
            .last()
            .map(|variable| variable.value.as_str())
    };
    if let Some(override_path) = override_for("ROSEMARY_PREFIX") {
        ensure!(
            Path::new(override_path) == prefix.as_path(),
            "Remove the custom ROSEMARY_PREFIX override to manage local Studio plugins."
        );
    }
    let setting = |name| {
        override_for(name)
            .map(OsString::from)
            .or_else(|| std::env::var_os(name))
    };
    if setting("ROSEMARY_PREFIX_HOME").as_deref() == Some(OsStr::new("host"))
        && let Some(home) = setting("HOME").filter(|home| !home.is_empty())
    {
        let home = PathBuf::from(home);
        ensure!(
            home.is_absolute(),
            "Studio's HOME must be an absolute path."
        );
        return Ok(home.join(PLUGINS_PATH));
    }

    let name = glib::user_name();
    let mut components = Path::new(&name).components();
    let (Some(Component::Normal(user)), None) = (components.next(), components.next()) else {
        bail!("Couldn't find Studio's home folder for the current user.");
    };
    Ok(prefix.join("root/Users").join(user).join(PLUGINS_PATH))
}

pub fn list(directory: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error).context("Couldn't open the local Studio plugins folder"),
    };
    let mut plugins = Vec::new();
    for entry in entries {
        let entry = entry.context("Couldn't read the local Studio plugins folder")?;
        let path = entry.path();
        if is_plugin_path(&path) && entry.file_type()?.is_file() {
            plugins.push(path);
        }
    }
    plugins.sort_unstable();
    Ok(plugins)
}

pub fn import(directory: &Path, source: &Path) -> anyhow::Result<()> {
    ensure!(
        is_plugin_path(source),
        "Choose an .rbxm or .rbxmx plugin file."
    );
    let name = source.file_name().context("The plugin file has no name.")?;
    let mut source = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
        .open(source)
        .context("Couldn't open the selected plugin file")?;
    ensure!(
        source.metadata()?.is_file(),
        "Choose a regular plugin file."
    );
    fs::create_dir_all(directory).context("Couldn't create Studio's local plugins folder")?;

    let staged = directory.join(format!(
        ".malus-plugin-{}.partial",
        glib::uuid_string_random()
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&staged)
        .context("Couldn't stage the local plugin")?;
    let result = (|| {
        io::copy(&mut source, &mut file).context("Couldn't copy the local plugin")?;
        file.sync_all().context("Couldn't save the local plugin")?;
        match fs::hard_link(&staged, directory.join(name)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::AlreadyExists => {
                bail!(
                    "A local plugin named {} already exists.",
                    name.to_string_lossy()
                )
            }
            Err(error) => Err(error).context("Couldn't install the local plugin"),
        }
    })();
    let _ = fs::remove_file(&staged);
    result
}

fn is_plugin_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("rbxm") || extension.eq_ignore_ascii_case("rbxmx")
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn import_keeps_existing_plugins_and_ignores_links_and_other_files() -> anyhow::Result<()> {
        let root = std::env::temp_dir().join(glib::uuid_string_random().as_str());
        fs::create_dir(&root)?;
        let result = (|| -> anyhow::Result<()> {
            let directory = root.join("Plugins");
            let source = root.join("Plugin.RBXMX");
            let destination = directory.join("Plugin.RBXMX");
            assert!(list(&directory)?.is_empty());
            fs::write(&source, b"first")?;
            import(&directory, &source)?;
            fs::write(&source, b"replacement")?;
            assert!(import(&directory, &source).is_err());
            assert_eq!(fs::read(&destination)?, b"first");

            let linked = root.join("Link.rbxm");
            symlink(&source, &linked)?;
            assert!(import(&directory, &linked).is_err());
            symlink(&source, directory.join("Link.rbxm"))?;
            fs::write(directory.join("notes.txt"), b"text")?;
            fs::create_dir(directory.join("Folder.rbxm"))?;
            assert_eq!(list(&directory)?, [destination]);
            assert_eq!(fs::read_dir(&directory)?.count(), 4);
            Ok(())
        })();
        fs::remove_dir_all(root)?;
        result
    }
}
