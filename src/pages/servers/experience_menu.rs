use crate::backend;
use crate::state::AppState;
use adw::prelude::*;
use gtk::{gio, glib};

pub fn experience_menu(
    state: &AppState,
    toasts: &glib::WeakRef<adw::ToastOverlay>,
    place_id: u64,
    recent: bool,
) -> gtk::MenuButton {
    let link = format!("https://www.roblox.com/games/{place_id}");
    let items = gio::Menu::new();
    let actions = gio::SimpleActionGroup::new();

    let copy = gio::SimpleAction::new("copy", None);
    copy.connect_activate(glib::clone!(
        #[strong]
        toasts,
        #[strong]
        link,
        move |_, _| {
            if let Some(toasts) = toasts.upgrade() {
                toasts.clipboard().set_text(&link);
                toasts.add_toast(adw::Toast::new("Roblox link copied"));
            }
        }
    ));
    actions.add_action(&copy);
    items.append(Some("Copy Roblox Link"), Some("experience.copy"));

    let open = gio::SimpleAction::new("open", None);
    open.connect_activate(glib::clone!(
        #[strong]
        toasts,
        #[weak]
        state,
        move |_, _| {
            let parent = toasts
                .upgrade()
                .and_then(|toasts| toasts.root())
                .and_downcast::<gtk::Window>();
            gtk::UriLauncher::new(&link).launch(
                parent.as_ref(),
                gio::Cancellable::NONE,
                glib::clone!(
                    #[weak]
                    state,
                    move |result| {
                        if let Err(error) = result {
                            state.report_error(&format!("Couldn't open the Roblox page: {error}"));
                        }
                    }
                ),
            );
        }
    ));
    actions.add_action(&open);
    items.append(Some("Open on Roblox"), Some("experience.open"));

    if recent {
        let remove = gio::SimpleAction::new("remove", None);
        remove.connect_activate(glib::clone!(
            #[weak]
            state,
            move |_, _| backend::forget_recent_experience(&state, place_id)
        ));
        actions.add_action(&remove);
        let history = gio::Menu::new();
        history.append(Some("Remove from Recents"), Some("experience.remove"));
        items.append_section(None, &history);
    }

    let button = gtk::MenuButton::builder()
        .icon_name("view-more-symbolic")
        .tooltip_text("Experience actions")
        .valign(gtk::Align::Center)
        .css_classes(["flat"])
        .menu_model(&items)
        .build();
    button.update_property(&[gtk::accessible::Property::Label("Experience actions")]);
    button.insert_action_group("experience", Some(&actions));
    button
}
