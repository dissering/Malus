use super::logs_button::logs_button;
use super::play_button::play_button;
use super::playtime_label::playtime_label;
use super::status_label::status_label;
use crate::components::icon::app_icon;
use crate::state::{AppState, Client};
use adw::prelude::*;

pub fn client_page(state: &AppState, client: Client) -> gtk::Widget {
    let logo = app_icon(96);

    let title = gtk::Label::builder()
        .label(client.home_title())
        .css_classes(["title-1"])
        .build();

    let subtitle = status_label(state, client);

    let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    actions.set_halign(gtk::Align::Center);
    actions.set_margin_top(8);
    actions.append(&play_button(state, client));
    actions.append(&logs_button(state));

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(12)
        .halign(gtk::Align::Center)
        .valign(gtk::Align::Center)
        .vexpand(true)
        .build();
    content.append(&logo);
    content.append(&title);
    content.append(&subtitle);
    if client == Client::Player {
        content.append(&playtime_label(state));
    }
    content.append(&actions);

    content.upcast()
}
