pub fn launcher_version_row() -> adw::ActionRow {
    adw::ActionRow::builder()
        .title("Launcher version")
        .subtitle(env!("CARGO_PKG_VERSION"))
        .build()
}
