use crate::backend::{self, StudioTarget};
use crate::components::play_action::{PROPERTIES_THAT_DECIDE_PLAY_ACTION, PlayAction};
use crate::state::{AppState, Client};
use adw::prelude::*;
use gtk::{gio, glib};

pub fn open_place_button(state: &AppState) -> gtk::Button {
    let button = gtk::Button::builder()
        .label("Open Place…")
        .css_classes(["pill"])
        .build();
    button.connect_clicked(glib::clone!(
        #[weak]
        state,
        move |button| {
            let parent = button.root().and_downcast::<gtk::Window>();
            let filter = gtk::FileFilter::new();
            filter.set_name(Some("Roblox places"));
            filter.add_suffix("rbxl");
            filter.add_suffix("rbxlx");
            gtk::FileDialog::builder()
                .title("Open Roblox Place")
                .default_filter(&filter)
                .modal(true)
                .build()
                .open(
                    parent.as_ref(),
                    gio::Cancellable::NONE,
                    glib::clone!(
                        #[weak]
                        state,
                        move |result| match result {
                            Ok(file) => {
                                let path = file.path().filter(|path| {
                                    path.extension()
                                        .and_then(|extension| extension.to_str())
                                        .is_some_and(|extension| {
                                            extension.eq_ignore_ascii_case("rbxl")
                                                || extension.eq_ignore_ascii_case("rbxlx")
                                        })
                                });
                                if let Some(path) = path {
                                    backend::open_studio(&state, StudioTarget::PlaceFile(path));
                                } else {
                                    state
                                        .report_error("Choose a local .rbxl or .rbxlx place file.");
                                }
                            }
                            Err(error)
                                if !error.matches(gtk::DialogError::Dismissed)
                                    && !error.matches(gtk::DialogError::Cancelled) =>
                            {
                                state.report_error(&format!(
                                    "Couldn't open the file chooser: {error}"
                                ));
                            }
                            Err(_) => {}
                        }
                    ),
                );
        }
    ));
    state.follow_properties(
        &PROPERTIES_THAT_DECIDE_PLAY_ACTION,
        glib::clone!(
            #[weak]
            button,
            move |state| {
                let action = PlayAction::for_client(state, Client::Studio);
                button.set_sensitive(
                    !state.running()
                        && matches!(action, PlayAction::Install(_) | PlayAction::Play(_))
                        && action.is_available(state),
                );
            }
        ),
    );
    button
}
