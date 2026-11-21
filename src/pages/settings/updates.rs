use super::bound_rows::switch_row;
use crate::backend;
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

const RELEASES_PAGE: &str = "https://github.com/dissering/Malus/releases/latest";

pub fn launcher_update_group(state: &AppState) -> adw::PreferencesGroup {
    let group = adw::PreferencesGroup::builder().title("Updates").build();
    let row = adw::ActionRow::builder()
        .title(format!("Malus {}", env!("CARGO_PKG_VERSION")))
        .build();
    state
        .bind_property("launcher-update-status", &row, "subtitle")
        .sync_create()
        .build();

    let check = gtk::Button::builder()
        .label("Check for updates")
        .valign(gtk::Align::Center)
        .build();
    state
        .bind_property("launcher-update-checking", &check, "sensitive")
        .invert_boolean()
        .sync_create()
        .build();
    check.connect_clicked(glib::clone!(
        #[weak]
        state,
        move |_| backend::check_launcher_update(&state)
    ));
    row.add_suffix(&check);
    group.add(&row);

    if backend::launcher_can_update_itself() {
        let automatic = switch_row(
            state,
            "auto-update-launcher",
            "Install updates automatically",
        );
        automatic.set_subtitle("Wait for Roblox and Studio to close before updating");
        group.add(&automatic);
    }

    let releases = gtk::LinkButton::with_label(RELEASES_PAGE, "GitHub releases");
    releases.set_halign(gtk::Align::Start);
    group.add(&releases);
    group
}
