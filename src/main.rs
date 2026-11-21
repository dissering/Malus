mod backend;
mod command_line;
mod components;
mod pages;
mod quadra;
mod state;
mod utils;

use adw::{Application, prelude::*};
use gtk::{gio, glib};
use quadra::Quadra;
use state::{AppState, Theme};
use std::cell::OnceCell;
use std::rc::Rc;

const APP_ID: &str = "io.github.dissering.Tiltframe";

fn main() -> glib::ExitCode {
    let args = match command_line::normalized_arguments(std::env::args().collect()) {
        Ok(args) => args,
        Err(error) => {
            eprintln!("{error}");
            return glib::ExitCode::FAILURE;
        }
    };
    if let Some(exit_code) = command_line::run_report_if_requested(&args) {
        return exit_code;
    }

    glib::set_application_name("Malus");
    let app = Application::builder()
        .application_id(APP_ID)
        .flags(gio::ApplicationFlags::HANDLES_COMMAND_LINE)
        .build();
    command_line::add_options(&app);
    let quadra: Rc<OnceCell<Rc<Quadra>>> = Rc::default();

    app.connect_startup(glib::clone!(
        #[strong]
        quadra,
        move |app| {
            let state = AppState::load();
            backend::initialize(&state);
            follow_theme(&state);
            app.connect_shutdown(glib::clone!(
                #[strong]
                state,
                move |_| state.save_pending_changes()
            ));
            quadra.get_or_init(|| Quadra::start(app, &state));
        }
    ));

    app.connect_command_line(glib::clone!(
        #[strong]
        quadra,
        move |app, command_line| {
            let Some(quadra) = quadra.get() else {
                return glib::ExitCode::FAILURE;
            };
            match command_line::requested_by(command_line)
                .and_then(|command| quadra.run_command(command))
            {
                Ok(()) => glib::ExitCode::SUCCESS,
                Err(error) => {
                    command_line::report_failure(command_line, &error);
                    if !command_line.is_remote() {
                        app.quit();
                    }
                    glib::ExitCode::FAILURE
                }
            }
        }
    ));

    app.connect_activate(move |_| {
        if let Some(quadra) = quadra.get() {
            quadra.activate();
        }
    });

    app.run_with_args(&args)
}

fn follow_theme(state: &AppState) {
    utils::register_bundled_fonts();

    let display = gtk::gdk::Display::default().unwrap();
    gio::resources_register_include!("malus-icons.gresource").expect("load Malus icons");
    let icons = gtk::IconTheme::for_display(&display);
    icons.add_resource_path("/io/github/dissering/Malus/icons");
    icons.add_search_path(glib::user_data_dir().join("icons"));
    gtk::Window::set_default_icon_name(APP_ID);
    let theme = gtk::CssProvider::new();
    gtk::style_context_add_provider_for_display(
        &display,
        &theme,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let header = gtk::CssProvider::new();
    header.load_from_string(
        ".app-header { padding-left: 0px; }
         .app-header .app-icon { margin-left: 12px; }
         .home-card { padding: 26px 28px 24px; border-radius: 22px; }
         .home-card .title-1 { margin-top: 2px; }",
    );
    gtk::style_context_add_provider_for_display(
        &display,
        &header,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    state.follow_properties(&["theme"], move |state| apply_theme(&theme, state.theme()));
}

fn apply_theme(provider: &gtk::CssProvider, theme: Theme) {
    let (color_scheme, stylesheet) = match theme {
        Theme::Grid => (adw::ColorScheme::ForceDark, include_str!("styles/grid.css")),
        Theme::Monochrome => (
            adw::ColorScheme::ForceDark,
            include_str!("styles/monochrome.css"),
        ),
        Theme::Adwaita => (adw::ColorScheme::Default, ""),
    };
    adw::StyleManager::default().set_color_scheme(color_scheme);
    provider.load_from_string(stylesheet);
}
