use crate::backend::{self, LaunchRequest, StudioTarget};
use adw::prelude::*;
use gtk::{gio, glib};

const ROBLOX_OPTION: &str = "roblox";
const STUDIO_OPTION: &str = "studio";
const STUDIO_LINK_SCHEME_PREFIX: &str = "roblox-studio";
const HARDWARE_REPORT_OPTION: &str = "hardware-report";
const COLLECT_DIAGNOSTICS_OPTION: &str = "collect-diagnostics";
const SETTINGS_OPTION: &str = "settings";
const LOGS_OPTION: &str = "logs";
const MENU_OPTION: &str = "open-menu";

pub enum Command {
    Activate,
    Menu,
    Settings,
    Logs,
    Launch(LaunchRequest),
}

pub fn normalized_arguments(mut args: Vec<String>) -> Result<Vec<String>, String> {
    let Some(command) = args.get(1).map(String::as_str) else {
        return Ok(args);
    };
    let (option, max_targets) = match command {
        "launch" => ("--roblox", 1),
        "studio" => ("--studio", 1),
        "settings" => ("--settings", 0),
        "logs" => ("--logs", 0),
        "menu" => ("--open-menu", 0),
        "hardware" => ("--hardware-report", 0),
        "diagnostics" => ("--collect-diagnostics", 1),
        "version" => ("--version", 0),
        "help" => ("--help", 0),
        "join" => {
            if args
                .get(2)
                .is_some_and(|arg| arg == "--help" || arg == "-h")
                && args.len() == 3
            {
                args[1] = "--help".to_owned();
                args.truncate(2);
                return Ok(args);
            }
            let valid_place = args
                .get(2)
                .and_then(|id| id.parse::<u64>().ok())
                .filter(|id| *id > 0);
            let Some(place_id) = valid_place else {
                return Err("Usage: malus join PLACE_ID [SERVER_ID]".to_owned());
            };
            let server = args.get(3).map(String::as_str);
            if args.len() > 4 || server.is_some_and(|id| !glib::uuid_string_is_valid(id)) {
                return Err(
                    "Usage: malus join PLACE_ID [SERVER_ID], where SERVER_ID is a UUID".to_owned(),
                );
            }
            let link = backend::game_uri(place_id, server);
            args.truncate(3);
            args[1] = "--roblox".to_owned();
            args[2] = link;
            return Ok(args);
        }
        _ => return Ok(args),
    };
    if args
        .get(2)
        .is_some_and(|arg| arg == "--help" || arg == "-h")
        && args.len() == 3
    {
        args[1] = "--help".to_owned();
        args.truncate(2);
        return Ok(args);
    }
    if args.len() > 2 + max_targets {
        return Err(format!(
            "Too many arguments for malus {command}. See malus help."
        ));
    }
    args[1] = option.to_owned();
    Ok(args)
}

pub fn add_options(app: &adw::Application) {
    app.set_option_context_summary(Some(
        "Commands:\n  launch [ROBLOX_LINK]       Launch Roblox Player\n  join PLACE_ID [SERVER_ID]  Join an experience or a specific server\n  studio [PLACE_FILE|LINK]   Launch Roblox Studio\n  menu                       Open the launcher\n  settings                   Open settings\n  logs                       Open live logs\n  hardware                   Print hardware information\n  diagnostics [DIRECTORY]    Save a diagnostics archive\n  version                    Print the Malus version",
    ));
    add_flag(app, MENU_OPTION, "Open the launcher");
    add_flag(app, SETTINGS_OPTION, "Open settings");
    add_flag(app, LOGS_OPTION, "Open live logs");
    add_flag(app, "version", "Print the Malus version, then exit");
    add_flag(
        app,
        ROBLOX_OPTION,
        "Start Roblox without opening Malus, following a roblox:// link if given",
    );
    add_flag(
        app,
        STUDIO_OPTION,
        "Start Roblox Studio without opening Malus, following a link or opening a place file if given",
    );
    add_flag(
        app,
        HARDWARE_REPORT_OPTION,
        "Print the hardware and driver details useful for bug reports, then exit",
    );
    add_flag(
        app,
        COLLECT_DIAGNOSTICS_OPTION,
        "Save a diagnostics archive (in the given folder, or Documents), then exit",
    );
    app.add_main_option(
        glib::OPTION_REMAINING,
        glib::Char(0),
        glib::OptionFlags::NONE,
        glib::OptionArg::StringArray,
        "",
        Some("[LINK|PLACE FILE]"),
    );
}

pub fn run_report_if_requested(arguments: &[String]) -> Option<glib::ExitCode> {
    let arguments = &arguments[1..];
    let is_requested = |option: &str| {
        arguments
            .iter()
            .position(|argument| *argument == format!("--{option}"))
    };

    if is_requested("version").is_some() {
        println!("Malus {}", env!("CARGO_PKG_VERSION"));
        return Some(glib::ExitCode::SUCCESS);
    }

    if is_requested(HARDWARE_REPORT_OPTION).is_some() {
        print!("{}", backend::hardware_report());
        return Some(glib::ExitCode::SUCCESS);
    }

    let position = is_requested(COLLECT_DIAGNOSTICS_OPTION)?;
    let directory = arguments
        .get(position + 1)
        .filter(|argument| !argument.starts_with('-'))
        .map_or_else(backend::diagnostics_directory, std::path::PathBuf::from);
    match backend::collect_diagnostics(&directory) {
        Ok(archive) => {
            println!("{}", archive.display());
            Some(glib::ExitCode::SUCCESS)
        }
        Err(error) => {
            eprintln!("Couldn't collect the diagnostics: {error:#}");
            Some(glib::ExitCode::FAILURE)
        }
    }
}

pub fn requested_by(command_line: &gio::ApplicationCommandLine) -> Command {
    let options = command_line.options_dict();
    if options.contains(SETTINGS_OPTION) {
        return Command::Settings;
    }
    if options.contains(LOGS_OPTION) {
        return Command::Logs;
    }
    if options.contains(MENU_OPTION) {
        return Command::Menu;
    }
    launch_requested_by(command_line).map_or(Command::Activate, Command::Launch)
}

fn launch_requested_by(command_line: &gio::ApplicationCommandLine) -> Option<LaunchRequest> {
    let options = command_line.options_dict();
    let target = options
        .lookup::<Vec<String>>(glib::OPTION_REMAINING)
        .ok()
        .flatten()
        .and_then(|arguments| arguments.into_iter().next());

    if options.contains(STUDIO_OPTION) {
        return Some(LaunchRequest::Studio(studio_target(command_line, target)));
    }
    if options.contains(ROBLOX_OPTION) {
        return Some(LaunchRequest::Roblox { link: target });
    }

    let target = target?;
    let is_roblox_link =
        scheme_of(&target).is_some_and(|scheme| !scheme.starts_with(STUDIO_LINK_SCHEME_PREFIX));
    if is_roblox_link {
        Some(LaunchRequest::Roblox { link: Some(target) })
    } else {
        Some(LaunchRequest::Studio(studio_target(
            command_line,
            Some(target),
        )))
    }
}

fn add_flag(app: &adw::Application, name: &str, description: &str) {
    app.add_main_option(
        name,
        glib::Char(0),
        glib::OptionFlags::NONE,
        glib::OptionArg::None,
        description,
        None,
    );
}

fn studio_target(
    command_line: &gio::ApplicationCommandLine,
    target: Option<String>,
) -> StudioTarget {
    let Some(target) = target else {
        return StudioTarget::StartPage;
    };
    if scheme_of(&target).is_some() {
        return StudioTarget::Link(target);
    }
    match command_line.create_file_for_arg(&target).path() {
        Some(place_file) => StudioTarget::PlaceFile(place_file),
        None => StudioTarget::StartPage,
    }
}

fn scheme_of(argument: &str) -> Option<String> {
    glib::Uri::peek_scheme(argument)
        .map(|scheme| scheme.to_string())
        .filter(|scheme| scheme != "file")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_named_commands_and_keeps_legacy_handlers() {
        let normalize = |args: &[&str]| {
            normalized_arguments(args.iter().map(|arg| (*arg).to_owned()).collect()).unwrap()
        };
        assert_eq!(normalize(&["malus", "launch"]), ["malus", "--roblox"]);
        assert_eq!(
            normalize(&["malus", "studio", "my level.rbxl"]),
            ["malus", "--studio", "my level.rbxl"]
        );
        assert_eq!(
            normalize(&["malus", "--studio", "place.rbxl"]),
            ["malus", "--studio", "place.rbxl"]
        );
        assert_eq!(
            normalize(&["malus", "diagnostics", "/tmp"]),
            ["malus", "--collect-diagnostics", "/tmp"]
        );
        assert_eq!(
            normalize(&["malus", "join", "123"]),
            [
                "malus",
                "--roblox",
                "roblox://experiences/start?placeId=123"
            ]
        );
        assert_eq!(
            normalize(&[
                "malus",
                "join",
                "123",
                "74f41c26-97d9-415c-abfa-2b5b684d6573"
            ]),
            [
                "malus",
                "--roblox",
                "roblox://experiences/start?placeId=123&gameInstanceId=74f41c26-97d9-415c-abfa-2b5b684d6573"
            ]
        );
        assert!(normalized_arguments(vec!["malus".into(), "join".into(), "0".into()]).is_err());
    }
}
