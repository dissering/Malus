use crate::backend::{self, MissingDiscordAccess};
use crate::components::command_dialog::show_command_dialog;
use crate::utils::is_running_in_flatpak;
use adw::prelude::*;

pub fn discord_access_row() -> Option<adw::ButtonRow> {
    if !is_running_in_flatpak() {
        return None;
    }

    let row = adw::ButtonRow::builder()
        .title("Set up Discord access")
        .end_icon_name("go-next-symbolic")
        .build();
    row.connect_activated(|row| {
        if let Some(missing) = backend::missing_discord_access() {
            show_discord_access_dialog(row, &missing);
        } else {
            let dialog = adw::AlertDialog::builder()
                .heading("Discord access is available")
                .body("Rich Presence appears when you play Roblox with Discord running.")
                .build();
            dialog.add_response("close", "Close");
            dialog.present(Some(row));
        }
    });
    Some(row)
}

fn show_discord_access_dialog(row: &adw::ButtonRow, missing: &MissingDiscordAccess) {
    let found = if missing.discords.is_empty() {
        "Malus couldn't find Discord running, so this allows every Discord app it supports. \
         To allow only yours, open Discord and try again."
            .to_owned()
    } else {
        format!(
            "Malus found {} running and needs permission to connect from Flatpak.",
            missing.discords.join(" and ")
        )
    };

    show_command_dialog(
        row,
        "Let Malus reach Discord",
        &format!("{found} Run this command in a terminal, then restart Malus."),
        &missing.command,
    );
}
