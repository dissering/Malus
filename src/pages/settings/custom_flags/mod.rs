mod flag_rows;
mod transfer;

use crate::state::AppState;
use adw::prelude::*;
use flag_rows::{FlagRows, filter_rows, show_custom_flags, show_editor};
use gtk::glib;

pub fn custom_flags_group(state: &AppState, search: &gtk::SearchEntry) -> adw::PreferencesGroup {
    let buttons = gtk::Box::new(gtk::Orientation::Horizontal, 6);
    let add = gtk::Button::with_label("Add");
    let import = gtk::Button::with_label("Import");
    let export = gtk::Button::with_label("Export");
    for button in [&add, &import, &export] {
        button.set_valign(gtk::Align::Center);
        button.add_css_class("flat");
        buttons.append(button);
    }

    let group = adw::PreferencesGroup::builder()
        .title("Custom overrides")
        .header_suffix(&buttons)
        .build();
    let rows = FlagRows::default();

    state.follow_properties(
        &["fast-flags"],
        glib::clone!(
            #[weak]
            group,
            #[weak]
            search,
            #[weak]
            rows,
            move |state| {
                show_custom_flags(&group, &rows, state);
                filter_rows(&group, &rows, &search.text());
            }
        ),
    );
    search.connect_search_changed(glib::clone!(
        #[weak]
        group,
        move |search| filter_rows(&group, &rows, &search.text())
    ));

    add.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |button| show_editor(button, &state, None)
    ));
    import.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |button| transfer::show_import(button, &state)
    ));
    export.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |button| transfer::show_export(button, &state)
    ));
    group
}
