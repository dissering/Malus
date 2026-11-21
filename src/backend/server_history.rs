use super::background::run_quietly;
use super::game_activity::{self, GameEvent};
use super::game_details;
use super::paths;
use super::regions::PickedServer;
use crate::state::{AppState, JoinedServer, ServerHistory};
use anyhow::Context;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

thread_local! {
    static LOCATIONS_OF_PICKED_SERVERS: RefCell<HashMap<String, String>> = RefCell::default();
}

pub fn initialize(state: &AppState) {
    state.set_server_history(read());

    state.connect_log_appended(|state, output| {
        for event in game_activity::events_in(output) {
            if let GameEvent::Joining {
                place_id,
                job_id,
                server_address,
            } = event
            {
                remember_join(state, place_id, job_id, server_address);
            }
        }
    });
}

pub fn expect_join_to(picked_server: PickedServer) {
    if let Some(location) = picked_server.location {
        LOCATIONS_OF_PICKED_SERVERS
            .with_borrow_mut(|locations| locations.insert(picked_server.job_id, location));
    }
}

pub fn remember_location(state: &AppState, job_id: &str, location: &str) {
    save(
        state,
        state.server_history().with_location(job_id, location),
    );
}

pub fn forget_all(state: &AppState) {
    save(state, ServerHistory::default());
}

pub fn forget_experience(state: &AppState, place_id: u64) {
    let mut history = state.server_history();
    if !history.remove_experience(place_id) {
        return;
    }
    let path = paths::server_history_file();
    match write(&path, &history) {
        Ok(()) => state.set_server_history(history),
        Err(error) => {
            state.report_error(&format!("Couldn't remove the recent experience. {error:#}"))
        }
    }
}

fn remember_join(state: &AppState, place_id: u64, job_id: String, server_address: Option<String>) {
    let location =
        LOCATIONS_OF_PICKED_SERVERS.with_borrow_mut(|locations| locations.remove(&job_id));
    let joined_server = JoinedServer {
        job_id,
        address: server_address,
        location,
        joined_at: seconds_since_epoch(),
    };
    save(
        state,
        state.server_history().with_join(place_id, joined_server),
    );

    if !state.server_history().has_name_for(place_id) {
        fetch_game_name(state, place_id);
    }
}

fn fetch_game_name(state: &AppState, place_id: u64) {
    let state = state.clone();
    run_quietly(
        move || game_details::name_for_place(place_id),
        move |name| match name {
            Ok(name) => save(
                &state,
                state.server_history().with_game_name(place_id, &name),
            ),
            Err(error) => eprintln!("Couldn't get the name of place {place_id}: {error:#}"),
        },
    );
}

fn save(state: &AppState, history: ServerHistory) {
    let path = paths::server_history_file();
    if let Err(error) = write(&path, &history) {
        eprintln!(
            "Couldn't save the server history to {}: {error:#}",
            path.display()
        );
    }

    state.set_server_history(history);
}

fn write(path: &Path, history: &ServerHistory) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("Couldn't create the server history directory")?;
    }
    let contents = serde_json::to_string_pretty(history)?;
    let unfinished_path = path.with_extension("json.partial");
    fs::write(&unfinished_path, contents).context("Couldn't write the server history")?;
    fs::rename(unfinished_path, path).context("Couldn't replace the server history file")?;
    Ok(())
}

fn read() -> ServerHistory {
    let path = paths::server_history_file();
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_else(|error| {
            eprintln!(
                "Ignoring the invalid server history in {}: {error}",
                path.display()
            );
            ServerHistory::default()
        }),
        Err(error) if error.kind() == ErrorKind::NotFound => ServerHistory::default(),
        Err(error) => {
            eprintln!(
                "Couldn't read the server history from {}: {error}",
                path.display()
            );
            ServerHistory::default()
        }
    }
}

fn seconds_since_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since_epoch| i64::try_from(since_epoch.as_secs()).ok())
        .unwrap_or_default()
}
