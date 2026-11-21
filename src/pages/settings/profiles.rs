use crate::backend::launch_profiles::{self, LaunchProfile, SaveMode};
use crate::state::{AppState, Choice};
use adw::prelude::*;
use gtk::glib;

pub fn profiles_section(state: &AppState) -> gtk::Box {
    let save = gtk::Button::with_label("Save current");
    save.set_valign(gtk::Align::Center);

    let group = adw::PreferencesGroup::builder()
        .title("Launch profiles")
        .description("Save a named set of FastFlags, graphics settings, and mod preferences. Apply it before your next Roblox session.")
        .header_suffix(&save)
        .build();

    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    let empty = gtk::Label::builder()
        .label("No launch profiles yet")
        .margin_top(12)
        .margin_bottom(12)
        .css_classes(["dim-label"])
        .build();
    list.set_placeholder(Some(&empty));
    group.add(&list);

    list.connect_map(glib::clone!(
        #[weak]
        state,
        move |list| refresh(list, &state)
    ));
    save.connect_clicked(glib::clone!(
        #[weak]
        state,
        #[weak]
        list,
        move |button| show_create_dialog(button, &list, &state)
    ));
    state.follow_properties(
        &["busy", "running"],
        glib::clone!(
            #[weak]
            list,
            #[weak]
            save,
            move |state| {
                let idle = !state.busy() && !state.running();
                save.set_sensitive(idle);
                list.set_sensitive(idle);
            }
        ),
    );

    let section = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_top(16)
        .margin_bottom(16)
        .build();
    section.append(&group);
    section
}

fn refresh(list: &gtk::ListBox, state: &AppState) {
    list.remove_all();
    let profiles = match launch_profiles::list() {
        Ok(profiles) => profiles,
        Err(error) => {
            let row = adw::ActionRow::builder()
                .title("Launch profiles unavailable")
                .subtitle(format!("{error:#}"))
                .use_markup(false)
                .build();
            list.append(&row);
            return;
        }
    };

    let current = LaunchProfile::capture(state);
    for (name, profile) in profiles {
        list.append(&profile_row(list, state, name, &profile, &current));
    }
}

fn profile_row(
    list: &gtk::ListBox,
    state: &AppState,
    name: String,
    profile: &LaunchProfile,
    current: &LaunchProfile,
) -> adw::ActionRow {
    let count = profile.override_count();
    let overrides = if count == 1 { "override" } else { "overrides" };
    let mods = if profile.has_mods() {
        "Mods enabled"
    } else {
        "Default assets"
    };
    let current_profile = profile == current;
    let row = adw::ActionRow::builder()
        .title(&name)
        .subtitle(format!(
            "{} · {count} {overrides} · {mods}",
            profile.renderer().label()
        ))
        .use_markup(false)
        .build();

    let apply = gtk::Button::with_label(if current_profile { "Applied" } else { "Apply" });
    apply.set_sensitive(!current_profile);
    apply.set_valign(gtk::Align::Center);
    apply.connect_clicked(glib::clone!(
        #[weak]
        state,
        #[weak]
        list,
        #[strong]
        name,
        move |_| report(&list, &state, launch_profiles::apply(&name, &state))
    ));
    row.add_suffix(&apply);

    let update = icon_button("view-refresh-symbolic", "Replace with current settings");
    update.set_sensitive(!current_profile);
    update.connect_clicked(glib::clone!(
        #[weak]
        state,
        #[weak]
        list,
        #[strong]
        name,
        move |button| {
            let action = glib::clone!(
                #[weak]
                list,
                #[weak]
                state,
                #[strong]
                name,
                move || report(
                    &list,
                    &state,
                    launch_profiles::save(&name, &state, SaveMode::Replace)
                )
            );
            confirm(
                button,
                "Replace profile?",
                &format!("Replace “{name}” with your current settings."),
                "Replace",
                adw::ResponseAppearance::Suggested,
                action,
            );
        }
    ));
    row.add_suffix(&update);

    let delete = icon_button("edit-delete-symbolic", "Delete profile");
    delete.connect_clicked(glib::clone!(
        #[weak]
        state,
        #[weak]
        list,
        move |button| {
            let action = glib::clone!(
                #[weak]
                list,
                #[weak]
                state,
                #[strong]
                name,
                move || report(&list, &state, launch_profiles::remove(&name))
            );
            confirm(
                button,
                "Delete profile?",
                &format!("Delete the saved “{name}” profile."),
                "Delete",
                adw::ResponseAppearance::Destructive,
                action,
            );
        }
    ));
    row.add_suffix(&delete);
    row
}

fn icon_button(icon: &str, tooltip: &str) -> gtk::Button {
    gtk::Button::builder()
        .icon_name(icon)
        .tooltip_text(tooltip)
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .build()
}

fn show_create_dialog(parent: &gtk::Button, list: &gtk::ListBox, state: &AppState) {
    let name = adw::EntryRow::builder().title("Profile name").build();
    let fields = adw::PreferencesGroup::new();
    fields.add(&name);

    let dialog = adw::AlertDialog::builder()
        .heading("Save launch profile")
        .extra_child(&fields)
        .build();
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("save", "Save");
    dialog.set_close_response("cancel");
    dialog.set_default_response(Some("save"));
    dialog.set_response_appearance("save", adw::ResponseAppearance::Suggested);
    dialog.set_response_enabled("save", false);
    name.connect_changed(glib::clone!(
        #[weak]
        dialog,
        move |entry| {
            let text = entry.text();
            let problem = launch_profiles::clean_name(&text).err();
            dialog.set_body(
                &problem
                    .as_ref()
                    .map(ToString::to_string)
                    .unwrap_or_default(),
            );
            dialog.set_response_enabled("save", problem.is_none());
        }
    ));
    dialog.connect_response(
        Some("save"),
        glib::clone!(
            #[weak]
            list,
            #[weak]
            state,
            #[strong]
            name,
            move |_, _| report(
                &list,
                &state,
                launch_profiles::save(&name.text(), &state, SaveMode::Create)
            )
        ),
    );
    dialog.present(Some(parent));
}

fn confirm(
    parent: &gtk::Button,
    heading: &str,
    body: &str,
    action_label: &str,
    appearance: adw::ResponseAppearance,
    action: impl Fn() + 'static,
) {
    let dialog = adw::AlertDialog::builder()
        .heading(heading)
        .body(body)
        .build();
    dialog.add_response("cancel", "Cancel");
    dialog.add_response("confirm", action_label);
    dialog.set_close_response("cancel");
    dialog.set_response_appearance("confirm", appearance);
    dialog.connect_response(Some("confirm"), move |_, _| action());
    dialog.present(Some(parent));
}

fn report(list: &gtk::ListBox, state: &AppState, result: anyhow::Result<()>) {
    match result {
        Ok(()) => refresh(list, state),
        Err(error) => state.report_error(&format!("{error:#}")),
    }
}
