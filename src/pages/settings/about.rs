use adw::prelude::*;

const VERSION: &str = env!("CARGO_PKG_VERSION");
const UPSTREAM: &str = "https://github.com/QuadraHQ/Quadra";

pub fn show_about(parent: &impl IsA<gtk::Widget>) {
    let dialog = adw::AboutDialog::builder()
        .application_name("Malus")
        .application_icon(crate::APP_ID)
        .version(VERSION)
        .license_type(gtk::License::Gpl30Only)
        .comments("A fork of Quadra for running the macOS Roblox Player and Studio on Linux with Rosemary.")
        .build();
    dialog.add_link("Upstream Quadra", UPSTREAM);
    dialog.present(Some(parent));
}
