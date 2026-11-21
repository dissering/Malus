use crate::backend;
use crate::state::AppState;
use adw::prelude::*;
use gtk::{gio, glib};

pub fn packs_group(state: &AppState) -> adw::PreferencesGroup {
    let import = gtk::Button::builder()
        .label("Import ZIP")
        .valign(gtk::Align::Center)
        .build();
    let group = adw::PreferencesGroup::builder()
        .title("Mod packs")
        .description("Import sounds, textures, and fonts. Enabled packs load in name order.")
        .header_suffix(&import)
        .build();
    let rows = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    group.add(&rows);
    state.follow_properties(
        &["disabled-mod-packs"],
        glib::clone!(
            #[weak]
            rows,
            move |state| refresh(&rows, state)
        ),
    );

    state.follow_properties(
        &["busy", "running"],
        glib::clone!(
            #[weak]
            import,
            move |state| import.set_sensitive(!state.busy() && !state.running())
        ),
    );
    state
        .bind_property("busy", &rows, "sensitive")
        .invert_boolean()
        .sync_create()
        .build();
    import.connect_clicked(glib::clone!(
        #[weak]
        rows,
        #[strong]
        state,
        move |button| open_zip(button, &rows, &state)
    ));
    group
}

fn refresh(rows: &gtk::ListBox, state: &AppState) {
    while let Some(row) = rows.first_child() {
        rows.remove(&row);
    }
    let packs = match backend::mod_packs() {
        Ok(packs) => packs,
        Err(error) => {
            rows.append(
                &adw::ActionRow::builder()
                    .title("Couldn't read mod packs")
                    .build(),
            );
            eprintln!("Couldn't read installed mod packs: {error:#}");
            return;
        }
    };
    if packs.is_empty() {
        rows.append(
            &adw::ActionRow::builder()
                .title("No mod packs installed")
                .build(),
        );
        return;
    }
    let disabled = state.disabled_mod_packs();
    for pack in packs {
        let row = adw::SwitchRow::builder()
            .title(pack.name)
            .use_markup(false)
            .active(!disabled.contains(&pack.id))
            .build();
        let state = state.clone();
        let id = pack.id;
        row.connect_active_notify(move |row| {
            let mut disabled = state.disabled_mod_packs();
            if row.is_active() {
                disabled.retain(|entry| entry != &id);
            } else if !disabled.contains(&id) {
                disabled.push(id.clone());
            }
            state.set_disabled_mod_packs(disabled);
        });
        rows.append(&row);
    }
}

fn open_zip(button: &gtk::Button, rows: &gtk::ListBox, state: &AppState) {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("ZIP archives"));
    filter.add_suffix("zip");
    let filters = gio::ListStore::new::<gtk::FileFilter>();
    filters.append(&filter);
    let dialog = gtk::FileDialog::builder()
        .title("Import mod pack")
        .filters(&filters)
        .build();
    let window = button.root().and_downcast::<gtk::Window>();
    dialog.open(
        window.as_ref(),
        gio::Cancellable::NONE,
        glib::clone!(
            #[weak]
            rows,
            #[strong]
            state,
            move |result| {
                if let Ok(file) = result
                    && let Some(path) = file.path()
                {
                    backend::import_mod_pack(
                        &state,
                        path,
                        glib::clone!(
                            #[weak]
                            rows,
                            #[weak]
                            state,
                            move || refresh(&rows, &state)
                        ),
                    );
                }
            }
        ),
    );
}
