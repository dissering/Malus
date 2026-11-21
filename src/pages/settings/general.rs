use super::account::account_group;
use super::bound_rows::{choice_row, switch_row};
use super::discord_access::discord_access_row;
use super::runtime::runtime_section;
use super::session::session_group;
use super::support::support_group;
use super::updates::launcher_update_group;
use crate::state::{AppState, LaunchMode};
use adw::prelude::*;

pub fn general_section(state: &AppState) -> gtk::Box {
    let group = adw::PreferencesGroup::builder().title("Malus").build();

    let launch_mode_row = choice_row::<LaunchMode>(state, "launch-mode", "When Malus starts");

    let run_in_background_row = switch_row(state, "run-in-background", "Run in the background");
    run_in_background_row.set_subtitle("Keep Malus in the system tray after its window closes");

    let advanced_row = switch_row(state, "advanced-settings-visible", "Advanced settings");

    group.add(&launch_mode_row);
    group.add(&run_in_background_row);
    group.add(&advanced_row);

    let discord_rich_presence_row =
        switch_row(state, "discord-rich-presence", "Discord Rich Presence");
    discord_rich_presence_row
        .set_subtitle("Show the Roblox experience you’re playing on your Discord profile");
    let discord_group = adw::PreferencesGroup::builder().title("Discord").build();
    discord_group.add(&discord_rich_presence_row);
    if let Some(row) = discord_access_row() {
        state
            .bind_property("discord-rich-presence", &row, "visible")
            .sync_create()
            .build();
        discord_group.add(&row);
    }
    let game_presence_row = switch_row(
        state,
        "allow-game-presence",
        "Allow game activity overrides",
    );
    game_presence_row.set_subtitle(
        "Let experiences customize Discord text, timers, and images through BloxstrapRPC",
    );
    let activity = adw::ExpanderRow::builder()
        .title("Activity settings")
        .subtitle("Choose what appears on your profile")
        .build();
    activity.add_row(&game_presence_row);
    activity.add_row(&discord_text_row(
        state,
        "discord-details-text",
        "Activity title",
    ));
    activity.add_row(&discord_text_row(
        state,
        "discord-state-text",
        "Activity status",
    ));
    activity.add_row(&switch_row(state, "discord-show-elapsed", "Show the timer"));
    activity.add_row(&switch_row(
        state,
        "discord-show-game-page",
        "Show the Roblox game link",
    ));
    state
        .bind_property("discord-rich-presence", &activity, "sensitive")
        .sync_create()
        .build();
    discord_group.add(&activity);

    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&group);
    section.append(&launcher_update_group(state));
    section.append(&discord_group);
    section.append(&runtime_section(state));

    section.append(&session_group(state));
    section.append(&super::privacy::privacy_group(state));
    section.append(&account_group(state));
    section.append(&support_group(state));

    section
}

fn discord_text_row(state: &AppState, property: &str, title: &str) -> adw::ActionRow {
    let entry = gtk::Entry::builder()
        .max_length(128)
        .valign(gtk::Align::Center)
        .placeholder_text("Default")
        .build();
    let row = adw::ActionRow::builder()
        .title(title)
        .subtitle("Use {game} for the experience name")
        .activatable_widget(&entry)
        .build();
    row.add_suffix(&entry);
    state
        .bind_property(property, &entry, "text")
        .bidirectional()
        .sync_create()
        .build();
    row
}
