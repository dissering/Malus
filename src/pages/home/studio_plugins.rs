use crate::backend::{self, studio_plugins};
use crate::state::AppState;
use adw::prelude::*;
use gtk::{gio, glib};
use std::path::PathBuf;

pub fn plugins_button(state: &AppState) -> gtk::Button {
    let button = gtk::Button::builder()
        .label("Studio Plugins")
        .halign(gtk::Align::Center)
        .css_classes(["flat"])
        .build();
    button.connect_clicked(glib::clone!(
        #[weak]
        state,
        move |button| show_dialog(button, &state)
    ));
    button
}

fn show_dialog(parent: &gtk::Button, state: &AppState) {
    let dialog = adw::PreferencesDialog::builder()
        .title("Studio Plugins")
        .content_width(520)
        .content_height(460)
        .build();
    let import = gtk::Button::builder()
        .label("Import…")
        .valign(gtk::Align::Center)
        .sensitive(studio_plugins::directory(state).is_ok())
        .build();
    let group = adw::PreferencesGroup::builder()
        .title("Local plugins")
        .description("Studio loads imported .rbxm and .rbxmx files the next time it opens.")
        .header_suffix(&import)
        .build();
    let rows = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::None)
        .css_classes(["boxed-list"])
        .build();
    group.add(&rows);
    let page = adw::PreferencesPage::new();
    page.add(&group);
    dialog.add(&page);

    import.connect_clicked(glib::clone!(
        #[weak]
        state,
        #[weak]
        rows,
        #[weak]
        dialog,
        move |button| {
            if state.busy() || state.running() {
                dialog.add_toast(adw::Toast::new("Close Roblox and Studio before importing"));
                return;
            }
            let parent = button.root().and_downcast::<gtk::Window>();
            glib::spawn_future_local(import_plugin(parent, dialog, rows, state));
        }
    ));
    refresh(&dialog, &rows, state);
    dialog.present(Some(parent));
}

fn refresh(dialog: &adw::PreferencesDialog, rows: &gtk::ListBox, state: &AppState) {
    while let Some(row) = rows.first_child() {
        rows.remove(&row);
    }
    match studio_plugins::directory(state).and_then(|directory| studio_plugins::list(&directory)) {
        Ok(plugins) if plugins.is_empty() => {
            rows.append(&adw::ActionRow::builder().title("No local plugins").build());
        }
        Ok(plugins) => {
            for path in plugins {
                let name = path.file_name().unwrap_or_default().to_string_lossy();
                let row = adw::ActionRow::builder()
                    .title(name)
                    .use_markup(false)
                    .build();
                let export = gtk::Button::builder()
                    .label("Export…")
                    .valign(gtk::Align::Center)
                    .build();
                export.connect_clicked(glib::clone!(
                    #[weak]
                    dialog,
                    move |button| {
                        let parent = button.root().and_downcast::<gtk::Window>();
                        glib::spawn_future_local(export_plugin(parent, dialog, path.clone()));
                    }
                ));
                row.add_suffix(&export);
                rows.append(&row);
            }
        }
        Err(error) => {
            rows.append(
                &adw::ActionRow::builder()
                    .title("Couldn't open the local plugin folder")
                    .subtitle(format!("{error:#}"))
                    .use_markup(false)
                    .build(),
            );
        }
    }
}

async fn import_plugin(
    parent: Option<gtk::Window>,
    dialog: adw::PreferencesDialog,
    rows: gtk::ListBox,
    state: AppState,
) {
    let filter = gtk::FileFilter::new();
    filter.set_name(Some("Roblox plugin files"));
    filter.add_suffix("rbxm");
    filter.add_suffix("rbxmx");
    let chooser = gtk::FileDialog::builder()
        .title("Import Studio Plugin")
        .default_filter(&filter)
        .build();
    let Ok(file) = chooser.open_future(parent.as_ref()).await else {
        return;
    };
    let Some(path) = file.path() else {
        dialog.add_toast(adw::Toast::new("Choose a local plugin file"));
        return;
    };
    backend::import_studio_plugin(
        &state,
        path,
        glib::clone!(
            #[weak]
            dialog,
            #[weak]
            rows,
            #[weak]
            state,
            move || {
                refresh(&dialog, &rows, &state);
                dialog.add_toast(adw::Toast::new("Plugin imported"));
            }
        ),
    );
}

async fn export_plugin(parent: Option<gtk::Window>, dialog: adw::PreferencesDialog, path: PathBuf) {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let chooser = gtk::FileDialog::builder()
        .title("Export Studio Plugin")
        .initial_name(name)
        .build();
    let Ok(destination) = chooser.save_future(parent.as_ref()).await else {
        return;
    };
    if !path
        .symlink_metadata()
        .is_ok_and(|metadata| metadata.is_file())
    {
        dialog.add_toast(adw::Toast::new("The plugin file is no longer available"));
        return;
    }
    let (copy, _) = gio::File::for_path(&path).copy_future(
        &destination,
        gio::FileCopyFlags::OVERWRITE | gio::FileCopyFlags::NOFOLLOW_SYMLINKS,
        glib::Priority::DEFAULT,
    );
    let message = match copy.await {
        Ok(()) => "Plugin exported".to_owned(),
        Err(error) => format!("Couldn't export the plugin: {error}"),
    };
    dialog.add_toast(adw::Toast::new(&message));
}
