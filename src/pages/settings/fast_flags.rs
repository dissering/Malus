use super::custom_flags::custom_flags_group;
use super::fast_flag_allowlist::{
    ALLOWLIST, ALLOWLIST_SOURCE, AllowlistedFlag, FlagCategory, FlagKind,
};
use crate::state::{AppState, Choice, FastFlagValue, FastFlags};
use adw::prelude::*;
use gtk::glib;

const INVALID_NUMBER_STYLE: &str = "error";

#[derive(Clone, Copy, PartialEq, Eq)]
enum ToggleSetting {
    Default,
    On,
    Off,
}

impl Choice for ToggleSetting {
    const ALL: &'static [Self] = &[Self::Default, Self::On, Self::Off];

    fn label(self) -> &'static str {
        match self {
            Self::Default => "Default",
            Self::On => "On",
            Self::Off => "Off",
        }
    }
}

impl ToggleSetting {
    fn of(value: Option<FastFlagValue>) -> Self {
        match value {
            Some(FastFlagValue::Toggle(true)) => Self::On,
            Some(FastFlagValue::Toggle(false)) => Self::Off,
            _ => Self::Default,
        }
    }

    fn value(self) -> Option<FastFlagValue> {
        match self {
            Self::Default => None,
            Self::On => Some(FastFlagValue::Toggle(true)),
            Self::Off => Some(FastFlagValue::Toggle(false)),
        }
    }
}

const TEXTURE_ENABLED: &str = "DFFlagTextureQualityOverrideEnabled";
const TEXTURE_QUALITY: &str = "DFIntTextureQualityOverride";

pub fn fast_flags_section(state: &AppState) -> gtk::Box {
    let search = gtk::SearchEntry::builder()
        .placeholder_text("Search by name or value")
        .hexpand(true)
        .build();
    let active_only = gtk::ToggleButton::with_label("Overrides only");
    let filters = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    filters.append(&search);
    filters.append(&active_only);

    let reset = gtk::Button::with_label("Reset all…");
    reset.set_valign(gtk::Align::Center);
    reset.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |button| confirm_reset(button, &state)
    ));

    let summary = adw::PreferencesGroup::builder()
        .title("FastFlags")
        .header_suffix(&reset)
        .build();
    state.follow_properties(
        &["fast-flags"],
        glib::clone!(
            #[weak]
            summary,
            #[weak]
            reset,
            move |state| {
                let count = state.fast_flags().0.len();
                let noun = if count == 1 { "override" } else { "overrides" };
                summary.set_description(Some(&format!(
                    "{count} {noun} · Applied when Roblox Player starts"
                )));
                reset.set_sensitive(count != 0);
            }
        ),
    );

    let section = gtk::Box::new(gtk::Orientation::Vertical, 16);
    section.set_margin_start(12);
    section.set_margin_end(12);
    section.set_margin_bottom(24);
    section.append(&summary);
    section.append(&filters);
    let quick_settings = texture_preset_group(state);
    section.append(&quick_settings);
    section.append(&custom_flags_group(state, &search));

    let groups: Vec<_> = ALLOWLIST
        .iter()
        .map(|category| {
            let (group, rows) = category_group(state, category);
            section.append(&group);
            (
                group.downgrade(),
                rows.into_iter()
                    .map(|(row, flag)| (row.downgrade(), flag))
                    .collect::<Vec<_>>(),
            )
        })
        .collect();
    let link = gtk::LinkButton::with_label(ALLOWLIST_SOURCE, "Roblox FastFlags allowlist");
    section.append(&link);

    let filter = std::rc::Rc::new(glib::clone!(
        #[weak]
        search,
        #[weak]
        active_only,
        #[weak]
        quick_settings,
        move |state: &AppState| {
            let query = search.text().trim().to_lowercase();
            let overrides_only = active_only.is_active();
            quick_settings.set_visible(query.is_empty() && !overrides_only);
            let flags = state.fast_flags();
            for (group, rows) in &groups {
                let Some(group) = group.upgrade() else {
                    continue;
                };
                let mut has_match = false;
                for (row, flag) in rows {
                    let Some(row) = row.upgrade() else { continue };
                    let value = flags.0.get(flag.name);
                    let value_matches =
                        value.is_some_and(|value| value.as_text().to_lowercase().contains(&query));
                    let visible = (!overrides_only || value.is_some())
                        && (flag.name.to_lowercase().contains(&query)
                            || flag.title.to_lowercase().contains(&query)
                            || value_matches);
                    row.set_visible(visible);
                    has_match |= visible;
                }
                group.set_visible(has_match);
            }
        }
    ));
    state.follow_properties(&["fast-flags"], {
        let filter = filter.clone();
        move |state| filter(state)
    });
    search.connect_search_changed(glib::clone!(
        #[weak]
        state,
        #[strong]
        filter,
        move |_| filter(&state)
    ));
    active_only.connect_toggled(glib::clone!(
        #[weak]
        state,
        move |_| filter(&state)
    ));
    section
}

fn confirm_reset(parent: &gtk::Button, state: &AppState) {
    let dialog = adw::AlertDialog::builder()
        .heading("Reset all FastFlags?")
        .body("This removes every override so Roblox can use its defaults.")
        .build();
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("reset", "Reset all");
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("reset", adw::ResponseAppearance::Destructive);
    dialog.connect_response(
        Some("reset"),
        glib::clone!(
            #[strong]
            state,
            move |_, _| state.set_fast_flags(FastFlags::default())
        ),
    );
    dialog.present(Some(parent));
}

fn texture_preset_group(state: &AppState) -> adw::PreferencesGroup {
    let row = adw::ActionRow::builder()
        .title("Texture quality")
        .subtitle("Change only the texture overrides")
        .build();
    for (label, value) in [("Default", None), ("Low", Some(0))] {
        let button = gtk::Button::with_label(label);
        button.set_valign(gtk::Align::Center);
        button.connect_clicked(glib::clone!(
            #[strong]
            state,
            move |_| {
                let updated = state
                    .fast_flags()
                    .with(TEXTURE_ENABLED, value.map(|_| FastFlagValue::Toggle(true)))
                    .with(TEXTURE_QUALITY, value.map(FastFlagValue::Number));
                if state.fast_flags() != updated {
                    state.set_fast_flags(updated);
                }
            }
        ));
        row.add_suffix(&button);
    }
    let group = adw::PreferencesGroup::builder()
        .title("Quick settings")
        .build();
    group.add(&row);
    group
}

fn category_group(
    state: &AppState,
    category: &'static FlagCategory,
) -> (
    adw::PreferencesGroup,
    Vec<(gtk::Widget, &'static AllowlistedFlag)>,
) {
    let group = adw::PreferencesGroup::builder()
        .title(category.title)
        .build();
    let rows = category
        .flags
        .iter()
        .map(|flag| {
            let row: gtk::Widget = match flag.kind {
                FlagKind::Toggle => toggle_row(state, flag).upcast(),
                FlagKind::Number => number_row(state, flag).upcast(),
            };
            group.add(&row);
            (row, flag)
        })
        .collect();
    (group, rows)
}

fn toggle_row(state: &AppState, flag: &'static AllowlistedFlag) -> adw::ComboRow {
    let current_setting = ToggleSetting::of(state.fast_flags().get(flag.name));

    let row = adw::ComboRow::builder()
        .title(flag.title)
        .subtitle(flag.name)
        .use_markup(false)
        .model(&gtk::StringList::new(&ToggleSetting::labels()))
        .selected(current_setting.position())
        .build();

    let syncing = std::rc::Rc::new(std::cell::Cell::new(false));
    row.connect_selected_notify(glib::clone!(
        #[strong]
        syncing,
        #[strong]
        state,
        move |row| {
            if syncing.get() {
                return;
            }
            let setting =
                ToggleSetting::at_position(row.selected()).unwrap_or(ToggleSetting::Default);
            set_flag(&state, flag.name, setting.value());
        }
    ));
    state.connect_notify_local(
        Some("fast-flags"),
        glib::clone!(
            #[weak]
            row,
            move |state, _| {
                let setting = ToggleSetting::of(state.fast_flags().get(flag.name));
                if row.selected() != setting.position() {
                    syncing.set(true);
                    row.set_selected(setting.position());
                    syncing.set(false);
                }
            }
        ),
    );

    row
}

fn number_row(state: &AppState, flag: &'static AllowlistedFlag) -> adw::EntryRow {
    let current_number = number_text(state, flag);

    let entry = adw::EntryRow::builder()
        .title(format!("{} · {}", flag.title, flag.name))
        .text(current_number)
        .input_purpose(gtk::InputPurpose::Number)
        .show_apply_button(true)
        .use_markup(false)
        .build();

    entry.connect_changed(|entry| {
        let text = entry.text();
        let text = text.trim();
        show_number_is_valid(entry, text.is_empty() || text.parse::<i64>().is_ok());
    });
    entry.connect_apply(glib::clone!(
        #[strong]
        state,
        move |entry| {
            let text = entry.text();
            let text = text.trim();
            if text.is_empty() {
                set_flag(&state, flag.name, None);
            } else if let Ok(number) = text.parse() {
                set_flag(&state, flag.name, Some(FastFlagValue::Number(number)));
            } else {
                entry.set_text(&number_text(&state, flag));
            }
        }
    ));
    let shown = std::cell::RefCell::new(number_text(state, flag));
    state.connect_notify_local(
        Some("fast-flags"),
        glib::clone!(
            #[weak]
            entry,
            move |state, _| {
                let saved = number_text(state, flag);
                if *shown.borrow() != saved {
                    entry.set_text(&saved);
                    shown.replace(saved);
                }
            }
        ),
    );

    entry
}

fn number_text(state: &AppState, flag: &AllowlistedFlag) -> String {
    state
        .fast_flags()
        .get(flag.name)
        .map(|value| value.as_text())
        .unwrap_or_default()
}

fn show_number_is_valid(entry: &adw::EntryRow, is_valid: bool) {
    if is_valid {
        entry.remove_css_class(INVALID_NUMBER_STYLE);
        entry.set_tooltip_text(None);
    } else {
        entry.add_css_class(INVALID_NUMBER_STYLE);
        entry.set_tooltip_text(Some(
            "Enter a whole number, or leave it empty for the default",
        ));
    }
}

fn set_flag(state: &AppState, name: &str, value: Option<FastFlagValue>) {
    let flags = state.fast_flags();
    if flags.get(name) != value {
        state.set_fast_flags(flags.with(name, value));
    }
}
