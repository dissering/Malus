use crate::state::{
    AppState, CursorStyle, DeathSound, FastFlags, PresentationMode, Renderer, SwapInterval,
    WindowSize,
};
use anyhow::{Context, bail, ensure};
use gtk::glib;
use gtk::prelude::ObjectExt;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_NAME_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LaunchProfile {
    renderer: Renderer,
    presentation_mode: PresentationMode,
    opengl_swap_interval: SwapInterval,
    #[serde(default)]
    window_size: Option<WindowSize>,
    fast_flags: FastFlags,
    death_sound: DeathSound,
    custom_death_sound: String,
    old_character_sounds: bool,
    cursor_style: CursorStyle,
    custom_cursor: String,
    custom_font: String,
    custom_mods: bool,
    #[serde(default)]
    disabled_mod_packs: Option<Vec<String>>,
}

impl LaunchProfile {
    pub fn capture(state: &AppState) -> Self {
        Self {
            renderer: state.renderer(),
            presentation_mode: state.presentation_mode(),
            opengl_swap_interval: state.opengl_swap_interval(),
            window_size: Some(state.window_size()),
            fast_flags: state.fast_flags(),
            death_sound: state.death_sound(),
            custom_death_sound: state.custom_death_sound(),
            old_character_sounds: state.old_character_sounds(),
            cursor_style: state.cursor_style(),
            custom_cursor: state.custom_cursor(),
            custom_font: state.custom_font(),
            custom_mods: state.custom_mods(),
            disabled_mod_packs: Some(state.disabled_mod_packs()),
        }
    }

    pub fn renderer(&self) -> Renderer {
        self.renderer
    }

    pub fn override_count(&self) -> usize {
        self.fast_flags.0.len()
    }

    pub fn has_mods(&self) -> bool {
        self.death_sound != DeathSound::Stock
            || self.old_character_sounds
            || self.cursor_style != CursorStyle::Stock
            || !self.custom_font.is_empty()
            || self.custom_mods
    }

    fn apply(self, state: &AppState) -> anyhow::Result<()> {
        ensure!(
            !state.busy() && !state.running(),
            "Stop Roblox and wait for any active task before switching profiles."
        );
        ensure!(
            !state.vr() || self.renderer == Renderer::OpenGl,
            "Turn VR off before applying a profile that uses Metal."
        );
        let _notifications = state.freeze_notify();
        state.set_renderer(self.renderer);
        state.set_presentation_mode(self.presentation_mode);
        state.set_opengl_swap_interval(self.opengl_swap_interval);
        if let Some(size) = self.window_size {
            state.set_window_size(size);
        }
        state.set_fast_flags(self.fast_flags);
        state.set_death_sound(self.death_sound);
        state.set_custom_death_sound(self.custom_death_sound);
        state.set_old_character_sounds(self.old_character_sounds);
        state.set_cursor_style(self.cursor_style);
        state.set_custom_cursor(self.custom_cursor);
        state.set_custom_font(self.custom_font);
        state.set_custom_mods(self.custom_mods);
        if let Some(disabled) = self.disabled_mod_packs {
            state.set_disabled_mod_packs(disabled);
        }
        Ok(())
    }
}

type Profiles = BTreeMap<String, LaunchProfile>;

pub fn list() -> anyhow::Result<Profiles> {
    read_profiles(&profiles_path())
}

pub fn apply(name: &str, state: &AppState) -> anyhow::Result<()> {
    let name = clean_name(name)?;
    let mut profiles = list()?;
    let profile = profiles.remove(name).with_context(|| {
        format!(
            "Launch profile “{name}” was not found. Open Settings → Profiles to see saved profiles."
        )
    })?;
    profile.apply(state)
}

#[derive(Clone, Copy)]
pub enum SaveMode {
    Create,
    Replace,
}

pub fn save(name: &str, state: &AppState, mode: SaveMode) -> anyhow::Result<()> {
    save_profile(&profiles_path(), name, LaunchProfile::capture(state), mode)
}

pub fn remove(name: &str) -> anyhow::Result<()> {
    let path = profiles_path();
    let mut profiles = read_profiles(&path)?;
    let name = clean_name(name)?;
    ensure!(
        profiles.remove(name).is_some(),
        "Launch profile “{name}” was not found."
    );
    write_profiles(&path, &profiles)
}

pub fn clean_name(name: &str) -> anyhow::Result<&str> {
    let name = name.trim();
    ensure!(!name.is_empty(), "Enter a profile name.");
    ensure!(
        name.chars().take(MAX_NAME_LENGTH + 1).count() <= MAX_NAME_LENGTH,
        "Profile names can have up to {MAX_NAME_LENGTH} characters."
    );
    ensure!(
        !name.chars().any(char::is_control),
        "Profile names cannot contain control characters."
    );
    Ok(name)
}

fn profiles_path() -> PathBuf {
    crate::state::settings_path().with_file_name("launch-profiles.json")
}

fn read_profiles(path: &Path) -> anyhow::Result<Profiles> {
    let contents = match fs::read(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Profiles::new()),
        Err(error) => return Err(error).context("Couldn't read launch profiles."),
    };
    serde_json::from_slice(&contents).context("Couldn't parse launch profiles.")
}

fn save_profile(
    path: &Path,
    name: &str,
    profile: LaunchProfile,
    mode: SaveMode,
) -> anyhow::Result<()> {
    let name = clean_name(name)?;
    let mut profiles = read_profiles(path)?;
    match (mode, profiles.contains_key(name)) {
        (SaveMode::Create, true) => bail!("A launch profile named “{name}” already exists."),
        (SaveMode::Replace, false) => bail!("Launch profile “{name}” was not found."),
        _ => {}
    }
    profiles.insert(name.to_owned(), profile);
    write_profiles(path, &profiles)
}

fn write_profiles(path: &Path, profiles: &Profiles) -> anyhow::Result<()> {
    let contents = serde_json::to_vec_pretty(profiles)?;
    crate::utils::create_parent_directory(path)?;
    glib::file_set_contents(path, &contents).context("Couldn't save launch profiles.")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::FastFlagValue;

    #[test]
    fn profile_round_trip_and_duplicate_protection() -> anyhow::Result<()> {
        let state: AppState = glib::Object::new();
        state.set_renderer(Renderer::OpenGl);
        state.set_fast_flags(
            FastFlags::default().with("FFlagProfile", Some(FastFlagValue::Toggle(true))),
        );
        state.set_custom_mods(true);
        state.set_disabled_mod_packs(vec![String::from("pack-id")]);
        let profile = LaunchProfile::capture(&state);
        let directory =
            std::env::temp_dir().join(format!("malus-profile-test-{}", glib::uuid_string_random()));
        fs::create_dir(&directory)?;
        let path = directory.join("launch-profiles.json");
        let result = (|| -> anyhow::Result<()> {
            save_profile(&path, "  Performance  ", profile.clone(), SaveMode::Create)?;
            let duplicate = save_profile(&path, "Performance", profile.clone(), SaveMode::Create);
            assert!(duplicate.is_err());

            let mut legacy: serde_json::Value = serde_json::from_slice(&fs::read(&path)?)?;
            legacy["Performance"]["refresh_rate"] = 144.into();
            fs::write(&path, serde_json::to_vec(&legacy)?)?;
            let restored = read_profiles(&path)?
                .remove("Performance")
                .context("Profile was missing after loading")?;
            assert_eq!(restored, profile);

            let changed: AppState = glib::Object::new();
            restored.apply(&changed)?;
            assert_eq!(LaunchProfile::capture(&changed), profile);

            changed.set_custom_mods(false);
            let replacement = LaunchProfile::capture(&changed);
            save_profile(&path, "Performance", replacement.clone(), SaveMode::Replace)?;
            assert_eq!(read_profiles(&path)?.get("Performance"), Some(&replacement));
            Ok(())
        })();
        fs::remove_dir_all(directory)?;
        result
    }

    #[test]
    fn leaves_settings_intact_when_launch_profile_conflicts_with_vr_or_busy_state() {
        let source: AppState = glib::Object::new();
        source.set_renderer(Renderer::Metal);
        source.set_custom_mods(true);
        let profile = LaunchProfile::capture(&source);
        let state: AppState = glib::Object::new();
        state.set_vr(true);
        state.set_renderer(Renderer::OpenGl);
        assert!(profile.clone().apply(&state).is_err());
        assert!(!state.custom_mods());
        state.set_vr(false);
        state.set_busy(true);
        assert!(profile.apply(&state).is_err());
        assert_eq!(state.renderer(), Renderer::OpenGl);
    }
}
