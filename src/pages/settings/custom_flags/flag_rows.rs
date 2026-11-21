use crate::components::ignored_flag_icon::{ignored_flag_icon, show_if_ignored};
use crate::pages::settings::fast_flag_allowlist::is_listed;
use crate::state::{AppState, FastFlagValue};
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::rc::Rc;

pub type FlagRows = Rc<RefCell<Vec<adw::ActionRow>>>;

pub fn show_custom_flags(group: &adw::PreferencesGroup, rows: &FlagRows, state: &AppState) {
    let mut rows = rows.borrow_mut();
    let flags = state.fast_flags();
    let custom: Vec<_> = flags
        .0
        .iter()
        .filter(|(name, _)| !is_listed(name))
        .collect();
    if rows.len() == custom.len()
        && rows.iter().zip(&custom).all(|(row, (name, value))| {
            row.title().as_str() == name.as_str()
                && row
                    .subtitle()
                    .is_some_and(|subtitle| subtitle.as_str() == value.as_text())
        })
    {
        return;
    }
    for row in rows.drain(..) {
        group.remove(&row);
    }
    for (name, value) in custom {
        let row = flag_row(state, name, value);
        group.add(&row);
        rows.push(row);
    }
}

pub fn filter_rows(group: &adw::PreferencesGroup, rows: &FlagRows, query: &str) {
    let query = query.trim().to_lowercase();
    let rows = rows.borrow();
    let mut matches = 0;
    for row in rows.iter() {
        let visible = row.title().to_lowercase().contains(&query)
            || row
                .subtitle()
                .is_some_and(|value| value.to_lowercase().contains(&query));
        row.set_visible(visible);
        matches += usize::from(visible);
    }
    group.set_description(match (rows.is_empty(), matches) {
        (true, _) => Some("No custom overrides"),
        (false, 0) => Some("No matching custom overrides"),
        _ => None,
    });
}

fn flag_row(state: &AppState, name: &str, value: &FastFlagValue) -> adw::ActionRow {
    let row = adw::ActionRow::builder()
        .title(name)
        .subtitle(value.as_text())
        .use_markup(false)
        .subtitle_lines(1)
        .build();

    let warning = ignored_flag_icon();
    show_if_ignored(&warning, name);
    row.add_prefix(&warning);

    let edit = row_button("malus-pencil-symbolic", "Edit override");
    row.add_suffix(&edit);
    row.set_activatable_widget(Some(&edit));
    let remove = row_button("malus-trash-symbolic", "Remove override");
    row.add_suffix(&remove);

    let name = name.to_owned();
    edit.connect_clicked(glib::clone!(
        #[weak]
        state,
        #[strong]
        name,
        move |button| show_editor(button, &state, Some(&name))
    ));
    remove.connect_clicked(glib::clone!(
        #[weak]
        state,
        move |_| state.set_fast_flags(state.fast_flags().with(&name, None))
    ));
    row
}

fn row_button(icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
}

pub fn show_editor(parent: &gtk::Button, state: &AppState, existing: Option<&str>) {
    let current = existing.and_then(|name| state.fast_flags().get(name));
    let name = adw::EntryRow::builder()
        .title("Flag name")
        .text(existing.unwrap_or_default())
        .build();
    let value = adw::EntryRow::builder()
        .title("Value")
        .text(
            current
                .as_ref()
                .map(FastFlagValue::as_text)
                .unwrap_or_default(),
        )
        .build();
    let fields = adw::PreferencesGroup::new();
    fields.add(&name);
    fields.add(&value);

    let dialog = adw::AlertDialog::builder()
        .heading(if existing.is_some() {
            "Edit override"
        } else {
            "Add override"
        })
        .extra_child(&fields)
        .build();
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("save", "Save");
    dialog.set_close_response("cancel");
    dialog.set_default_response(Some("save"));
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);

    let existing = existing.map(str::to_owned);
    let validate = glib::clone!(
        #[weak]
        state,
        #[weak]
        dialog,
        #[strong]
        existing,
        move |name: &adw::EntryRow| {
            let text = name.text();
            let name = text.trim();
            let problem = if name.is_empty() || name.contains([' ', '\t', '\n', '=']) {
                "Enter a flag name without spaces or equals signs."
            } else if is_listed(name) {
                "Edit this flag under its settings category."
            } else if existing.as_deref() != Some(name) && state.fast_flags().get(name).is_some() {
                "This flag already has an override."
            } else {
                ""
            };
            dialog.set_body(problem);
            dialog.set_response_enabled("save", problem.is_empty());
        }
    );
    validate(&name);
    name.connect_changed(validate);
    dialog.connect_response(
        Some("save"),
        glib::clone!(
            #[weak]
            state,
            move |_, _| {
                let entered_name = name.text();
                let entered_name = entered_name.trim();
                let text = value.text();
                let new_value = match current.as_ref() {
                    Some(current) if current.as_text() == text.as_str() => current.clone(),
                    _ => FastFlagValue::parse(&text),
                };
                let mut flags = state.fast_flags();
                if let Some(existing) = &existing {
                    flags.0.remove(existing);
                }
                flags.0.insert(entered_name.to_owned(), new_value);
                state.set_fast_flags(flags);
            }
        ),
    );
    dialog.present(Some(parent));
}
