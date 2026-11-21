use crate::components::logs_dialog::logs_dialog;
use crate::state::AppState;
use adw::prelude::*;
use gtk::glib;

pub fn logs_button(state: &AppState) -> gtk::Button {
    let content = adw::ButtonContent::builder()
        .label("Logs")
        .icon_name("malus-terminal-symbolic")
        .build();

    let button = gtk::Button::builder()
        .child(&content)
        .css_classes(["flat"])
        .halign(gtk::Align::Center)
        .build();

    button.connect_clicked(glib::clone!(
        #[strong]
        state,
        move |button| logs_dialog(&state).present(Some(button))
    ));

    button
}
