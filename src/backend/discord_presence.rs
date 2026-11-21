use super::background::run_quietly;
use super::bloxstrap_rpc::{Overrides, PresenceUpdate};
use super::discord_ipc::{DiscordClient, Image, Link, RichPresence};
use super::game_activity::{self, GameEvent};
use super::game_details::{self, GameDetails};
use crate::state::AppState;
use gtk::glib;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const DISCORD_CLIENT_ID: &str = "363445589247131668";
const QUADRA_ICON_URL: &str = "https://raw.githubusercontent.com/dissering/Malus/main/data/icons/hicolor/512x512/apps/io.github.dissering.Tiltframe.png";
const QUADRA_ICON_TEXT: &str = "Malus";
const REFRESH_GAME_DETAILS_EVERY: Duration = Duration::from_secs(60);
const SHORTEST_TIME_BETWEEN_RPC_UPDATES: Duration = Duration::from_secs(1);
const WAIT_FOR_JOIN_REPORT: Duration = Duration::from_secs(10);
const PRESENCE_CUSTOMIZATIONS: [&str; 4] = [
    "discord-details-text",
    "discord-state-text",
    "discord-show-elapsed",
    "discord-show-game-page",
];
const SHORTEST_DISCORD_TEXT: usize = 2;
const LONGEST_DISCORD_TEXT: usize = 128;
const INVISIBLE_PADDING: char = '\u{2800}';
const VERIFIED_BADGE: &str = " ☑️";

struct CurrentGame {
    place_id: u64,
    universe_id: Option<u64>,
    joined_at: i64,
    details: Option<GameDetails>,
    overrides: Overrides,
    refreshing_details: glib::SourceId,
}

#[derive(Default)]
struct Presence {
    discord: Option<DiscordClient>,
    roblox_started_at: Option<i64>,
    game: Option<CurrentGame>,
    shown_presence: Option<RichPresence>,
    last_rpc_update_at: Option<Instant>,
    pending_overrides: Option<Overrides>,
    flush_rpc: Option<glib::SourceId>,
    asset_urls: HashMap<u64, String>,
    requested_assets: HashSet<u64>,
}

type SharedPresence = Rc<RefCell<Presence>>;

pub fn initialize(state: &AppState) {
    let presence = SharedPresence::default();

    state.connect_log_appended(glib::clone!(
        #[strong]
        presence,
        move |state, output| {
            for event in game_activity::events_in(output) {
                match event {
                    GameEvent::Joining { place_id, .. } => join_game(state, &presence, place_id),
                    GameEvent::JoinReported { universe_id } => {
                        learn_universe(state, &presence, universe_id);
                    }
                    GameEvent::Connected { .. } => {}
                    GameEvent::Left => leave_game(&presence),
                    GameEvent::PresenceUpdated(update) => {
                        update_presence(state, &presence, update);
                    }
                }
            }
            show(state, &presence);
        }
    ));

    state.connect_properties_notify(&PRESENCE_CUSTOMIZATIONS, {
        let presence = presence.clone();
        move |state| show(state, &presence)
    });
    state.connect_properties_notify(
        &["running", "discord-rich-presence", "allow-game-presence"],
        move |state| {
            if !state.allow_game_presence() {
                clear_overrides(&mut presence.borrow_mut());
            }
            follow_roblox(state, &presence);
            fetch_missing_assets(state, &presence);
            show(state, &presence);
        },
    );
}

fn follow_roblox(state: &AppState, presence: &SharedPresence) {
    if !state.running() {
        leave_game(presence);
        presence.borrow_mut().roblox_started_at = None;
        return;
    }

    presence
        .borrow_mut()
        .roblox_started_at
        .get_or_insert_with(now_in_seconds);

    let place_missing_details = presence
        .borrow()
        .game
        .as_ref()
        .filter(|game| game.details.is_none())
        .map(|game| game.place_id);
    if let Some(place_id) = place_missing_details {
        fetch_details(state, presence, place_id);
    }
}

fn join_game(state: &AppState, presence: &SharedPresence, place_id: u64) {
    let is_already_in_this_place = presence
        .borrow()
        .game
        .as_ref()
        .is_some_and(|game| game.place_id == place_id);
    if is_already_in_this_place {
        return;
    }
    leave_game(presence);

    let refreshing_details = glib::timeout_add_local(
        REFRESH_GAME_DETAILS_EVERY,
        glib::clone!(
            #[strong]
            state,
            #[strong]
            presence,
            move || {
                fetch_details(&state, &presence, place_id);
                glib::ControlFlow::Continue
            }
        ),
    );
    presence.borrow_mut().game = Some(CurrentGame {
        place_id,
        universe_id: None,
        joined_at: now_in_seconds(),
        details: None,
        overrides: Overrides::default(),
        refreshing_details,
    });

    glib::timeout_add_local_once(
        WAIT_FOR_JOIN_REPORT,
        glib::clone!(
            #[strong]
            state,
            #[strong]
            presence,
            move || {
                let join_report_never_came = presence.borrow().game.as_ref().is_some_and(|game| {
                    game.place_id == place_id
                        && game.universe_id.is_none()
                        && game.details.is_none()
                });
                if join_report_never_came {
                    fetch_details(&state, &presence, place_id);
                }
            }
        ),
    );
}

fn learn_universe(state: &AppState, presence: &SharedPresence, universe_id: u64) {
    let place_missing_details = {
        let mut presence = presence.borrow_mut();
        let Some(game) = presence.game.as_mut() else {
            return;
        };
        game.universe_id = Some(universe_id);
        game.details.is_none().then_some(game.place_id)
    };
    if let Some(place_id) = place_missing_details {
        fetch_details(state, presence, place_id);
    }
}

fn leave_game(presence: &SharedPresence) {
    let left_game = {
        let mut presence = presence.borrow_mut();
        presence.asset_urls.clear();
        presence.requested_assets.clear();
        clear_overrides(&mut presence);
        presence.game.take()
    };
    if let Some(game) = left_game {
        game.refreshing_details.remove();
    }
}

fn clear_overrides(presence: &mut Presence) {
    if let Some(timer) = presence.flush_rpc.take() {
        timer.remove();
    }
    presence.pending_overrides = None;
    presence.last_rpc_update_at = None;
    if let Some(game) = presence.game.as_mut() {
        game.overrides = Overrides::default();
    }
}

fn update_presence(state: &AppState, presence: &SharedPresence, update: PresenceUpdate) {
    if !state.allow_game_presence() {
        return;
    }
    let mut current = presence.borrow_mut();
    if let Some(pending) = current.pending_overrides.as_mut() {
        pending.apply(update);
        return;
    }
    let now = Instant::now();
    let delay = current.last_rpc_update_at.map_or(Duration::ZERO, |last| {
        SHORTEST_TIME_BETWEEN_RPC_UPDATES.saturating_sub(now.duration_since(last))
    });
    let Some(game) = current.game.as_mut() else {
        return;
    };
    if delay.is_zero() {
        game.overrides.apply(update);
        current.last_rpc_update_at = Some(now);
        drop(current);
        fetch_missing_assets(state, presence);
        return;
    }

    let mut pending = game.overrides.clone();
    pending.apply(update);
    current.pending_overrides = Some(pending);
    current.flush_rpc = Some(glib::timeout_add_local_once(
        delay,
        glib::clone!(
            #[weak]
            state,
            #[weak]
            presence,
            move || {
                let mut current = presence.borrow_mut();
                current.flush_rpc.take();
                let pending = current.pending_overrides.take();
                if let (Some(game), Some(pending)) = (current.game.as_mut(), pending) {
                    game.overrides = pending;
                    current.last_rpc_update_at = Some(Instant::now());
                    drop(current);
                    fetch_missing_assets(&state, &presence);
                    show(&state, &presence);
                }
            }
        ),
    ));
}

fn fetch_missing_assets(state: &AppState, presence: &SharedPresence) {
    if !state.discord_rich_presence() {
        return;
    }

    let missing: Vec<u64> = {
        let mut presence = presence.borrow_mut();
        let wanted: Vec<u64> = presence
            .game
            .as_ref()
            .map(|game| game.overrides.asset_ids().collect())
            .unwrap_or_default();
        wanted
            .into_iter()
            .filter(|asset_id| {
                !presence.asset_urls.contains_key(asset_id)
                    && presence.requested_assets.insert(*asset_id)
            })
            .collect()
    };

    for asset_id in missing {
        let state = state.clone();
        let presence = presence.clone();
        run_quietly(
            move || game_details::asset_image_url(asset_id),
            move |url| {
                match url {
                    Ok(Some(url)) => {
                        presence.borrow_mut().asset_urls.insert(asset_id, url);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("Couldn't get the image of asset {asset_id}: {error:#}");
                        presence.borrow_mut().requested_assets.remove(&asset_id);
                    }
                }
                show(&state, &presence);
            },
        );
    }
}

fn fetch_details(state: &AppState, presence: &SharedPresence, place_id: u64) {
    if !state.discord_rich_presence() {
        return;
    }

    let universe_id = presence
        .borrow()
        .game
        .as_ref()
        .filter(|game| game.place_id == place_id)
        .and_then(|game| game.universe_id);

    let state = state.clone();
    let presence = presence.clone();
    run_quietly(
        move || match universe_id {
            Some(universe_id) => game_details::fetch_universe(universe_id),
            None => game_details::fetch(place_id),
        },
        move |details| {
            let fetched = match details {
                Ok(details) => details,
                Err(error) => {
                    eprintln!("Couldn't get the details of place {place_id}: {error:#}");
                    return;
                }
            };

            if let Some(game) = presence
                .borrow_mut()
                .game
                .as_mut()
                .filter(|game| game.place_id == place_id)
            {
                let previous_icon_url = game
                    .details
                    .take()
                    .and_then(|previous_details| previous_details.icon_url);
                game.details = Some(GameDetails {
                    icon_url: fetched.icon_url.or(previous_icon_url),
                    ..fetched
                });
            }
            show(&state, &presence);
        },
    );
}

fn show(state: &AppState, presence: &SharedPresence) {
    let mut presence = presence.borrow_mut();
    let wants_presence = state.discord_rich_presence() && state.running();
    let rich_presence = wants_presence.then(|| {
        let (activity, game_name) = match &presence.game {
            Some(game) => (
                playing_presence(game, &presence.asset_urls),
                game.details
                    .as_ref()
                    .map_or("Roblox", |details| details.name.as_str()),
            ),
            None => (idle_presence(presence.roblox_started_at), "Roblox"),
        };
        customize_presence(state, activity, game_name)
    });

    if presence.shown_presence == rich_presence {
        return;
    }
    presence.shown_presence = rich_presence.clone();
    presence
        .discord
        .get_or_insert_with(|| DiscordClient::start(DISCORD_CLIENT_ID))
        .show(rich_presence);
}

fn customize_presence(
    state: &AppState,
    mut activity: RichPresence,
    game_name: &str,
) -> RichPresence {
    let details = state.discord_details_text();
    let status = state.discord_state_text();
    if !details.trim().is_empty() {
        activity.details = presence_text(&details, game_name);
    }
    if !status.trim().is_empty() {
        activity.state = Some(presence_text(&status, game_name));
    }
    if !state.discord_show_elapsed() {
        activity.started_at = None;
        activity.ended_at = None;
    }
    if !state.discord_show_game_page() {
        activity.link = None;
    }
    activity
}

fn presence_text(setting: &str, game_name: &str) -> String {
    let bounded: String = setting.trim().chars().take(LONGEST_DISCORD_TEXT).collect();
    discord_text(&bounded.replace("{game}", game_name))
}

fn idle_presence(roblox_started_at: Option<i64>) -> RichPresence {
    RichPresence {
        details: "Idle".to_owned(),
        state: None,
        started_at: roblox_started_at,
        ended_at: None,
        large_image: Some(quadra_icon()),
        small_image: None,
        link: None,
    }
}

fn playing_presence(game: &CurrentGame, asset_urls: &HashMap<u64, String>) -> RichPresence {
    let presence = game.overrides.over(game_presence(game), asset_urls);
    RichPresence {
        details: discord_text(&presence.details),
        state: presence.state.as_deref().map(discord_text),
        large_image: presence.large_image.map(fitted_image),
        small_image: presence.small_image.map(fitted_image),
        ..presence
    }
}

fn fitted_image(image: Image) -> Image {
    Image {
        text: discord_text(&image.text),
        ..image
    }
}

fn game_presence(game: &CurrentGame) -> RichPresence {
    let game_page = Link {
        label: "See game page".to_owned(),
        url: format!("https://www.roblox.com/games/{}", game.place_id),
    };

    let Some(details) = &game.details else {
        return RichPresence {
            details: "Playing a game".to_owned(),
            state: None,
            started_at: Some(game.joined_at),
            ended_at: None,
            large_image: Some(quadra_icon()),
            small_image: None,
            link: Some(game_page),
        };
    };

    let game_name = discord_text(&details.name);
    let (large_image, small_image) = match &details.icon_url {
        Some(icon_url) => (
            Image {
                url: icon_url.clone(),
                text: game_name.clone(),
            },
            Some(quadra_icon()),
        ),
        None => (quadra_icon(), None),
    };

    RichPresence {
        details: game_name,
        state: Some(discord_text(&made_by(details))),
        started_at: Some(game.joined_at),
        ended_at: None,
        large_image: Some(large_image),
        small_image,
        link: Some(game_page),
    }
}

fn made_by(details: &GameDetails) -> String {
    let badge = if details.creator_is_verified {
        VERIFIED_BADGE
    } else {
        ""
    };
    format!("by {}{badge}", details.creator)
}

fn quadra_icon() -> Image {
    Image {
        url: QUADRA_ICON_URL.to_owned(),
        text: QUADRA_ICON_TEXT.to_owned(),
    }
}

fn discord_text(text: &str) -> String {
    let mut text: String = text.chars().take(LONGEST_DISCORD_TEXT).collect();
    while text.chars().count() < SHORTEST_DISCORD_TEXT {
        text.push(INVISIBLE_PADDING);
    }
    text
}

fn now_in_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|since_epoch| i64::try_from(since_epoch.as_secs()).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    #[test]
    fn user_activity_text_and_privacy_take_precedence_over_game_values() {
        let state = glib::Object::builder::<AppState>()
            .property("discord-details-text", " Playing {game} ")
            .property("discord-state-text", " On Linux ")
            .build();
        let mut input = idle_presence(Some(10));
        input.ended_at = Some(20);
        input.link = Some(Link {
            label: String::from("Open"),
            url: String::from("https://www.roblox.com/games/123"),
        });
        let output = customize_presence(&state, input, "Example");
        assert_eq!(output.details, "Playing Example");
        assert_eq!(output.state.as_deref(), Some("On Linux"));
        assert_eq!(output.started_at, None);
        assert_eq!(output.ended_at, None);
        assert!(output.link.is_none());
    }

    use super::*;
    use crate::backend::bloxstrap_rpc::presence_update_in;
    #[test]
    fn coalesces_partial_updates_without_leaking_into_another_game() {
        let state = glib::Object::builder::<AppState>()
            .property("allow-game-presence", true)
            .build();
        let presence = SharedPresence::default();
        presence.borrow_mut().game = Some(CurrentGame {
            place_id: 123,
            universe_id: None,
            joined_at: 0,
            details: None,
            overrides: Overrides::default(),
            refreshing_details: glib::timeout_add_local_once(Duration::from_secs(60), || {}),
        });
        presence.borrow_mut().last_rpc_update_at =
            Some(Instant::now() - Duration::from_millis(975));

        let update = |fields| {
            presence_update_in(&format!(
                r#"[BloxstrapRPC] {{"command":"SetRichPresence","data":{fields}}}"#
            ))
            .unwrap()
        };
        update_presence(&state, &presence, update(r#"{"details":"Building"}"#));
        update_presence(&state, &presence, update(r#"{"state":"Round 2"}"#));

        while presence.borrow().flush_rpc.is_some() {
            glib::MainContext::default().iteration(true);
        }
        let result = presence
            .borrow()
            .game
            .as_ref()
            .unwrap()
            .overrides
            .over(idle_presence(None), &HashMap::new());
        assert_eq!(result.details, "Building");
        assert_eq!(result.state.as_deref(), Some("Round 2"));
        update_presence(&state, &presence, update(r#"{"state":"Discard on exit"}"#));
        leave_game(&presence);
        let current = presence.borrow();
        assert!(current.game.is_none());
        assert!(current.flush_rpc.is_none());
        assert!(current.pending_overrides.is_none());
        assert!(current.last_rpc_update_at.is_none());
    }
}
