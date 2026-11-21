use super::join_availability::JoinAvailability;
use super::place_link::place_id_in;
use super::previews::{Previews, REFRESH_INTERVAL};
use super::time_since::time_since;
use crate::backend::{self, Favorites};
use crate::state::{AppState, JoinedServer, PlayedGame, ServerHistory};
use adw::prelude::*;
use gtk::glib;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;

const UNKNOWN_LOCATION: &str = "Unknown location";

struct Library {
    favorites: RefCell<Favorites>,
    previews: Rc<Previews>,
    favorite_group: glib::WeakRef<adw::PreferencesGroup>,
    history_group: glib::WeakRef<adw::PreferencesGroup>,
    forget_group: glib::WeakRef<adw::PreferencesGroup>,
    favorite_rows: RefCell<Vec<glib::WeakRef<gtk::Widget>>>,
    history_rows: RefCell<Vec<(u64, glib::WeakRef<gtk::Widget>)>>,
    join_availability: JoinAvailability,
}

pub fn library_groups(
    state: &AppState,
    join_availability: &JoinAvailability,
) -> [adw::PreferencesGroup; 3] {
    let favorites_group = adw::PreferencesGroup::builder().title("Favorites").build();
    let add_row = adw::EntryRow::builder()
        .title("Pin an experience by place ID or Roblox link")
        .show_apply_button(true)
        .build();
    favorites_group.add(&add_row);

    let history_group = adw::PreferencesGroup::builder()
        .title("Recent experiences")
        .build();
    let forget_row = adw::ButtonRow::builder()
        .title("Clear history")
        .css_classes(["destructive-action"])
        .build();
    forget_row.connect_activated(glib::clone!(
        #[weak]
        state,
        move |_| backend::forget_server_history(&state)
    ));
    let forget_group = adw::PreferencesGroup::new();
    forget_group.add(&forget_row);

    let model = Rc::new(Library {
        favorites: RefCell::new(Favorites::load().unwrap_or_else(|error| {
            eprintln!("Couldn't load favorites: {error:#}");
            Favorites::default()
        })),
        previews: Rc::default(),
        favorite_group: favorites_group.downgrade(),
        history_group: history_group.downgrade(),
        forget_group: forget_group.downgrade(),
        favorite_rows: RefCell::default(),
        history_rows: RefCell::default(),
        join_availability: join_availability.clone(),
    });
    model.render(state);

    add_row.connect_apply(glib::clone!(
        #[weak]
        state,
        #[weak]
        model,
        move |row| {
            if let Some(place_id) = place_id_in(&row.text()).filter(|&id| id != 0) {
                if model.set_pinned(&state, place_id, true) {
                    row.set_text("");
                }
            } else {
                row.add_css_class("error");
            }
        }
    ));
    add_row.connect_changed(|row| row.remove_css_class("error"));

    state.connect_server_history_notify(glib::clone!(
        #[weak]
        model,
        move |state| model.render(state)
    ));
    favorites_group.connect_map(glib::clone!(
        #[weak]
        state,
        #[strong]
        model,
        move |_| model.refresh(&state)
    ));
    favorites_group.connect_unmap(glib::clone!(
        #[weak]
        model,
        move |_| model.previews.hide()
    ));
    let live_model = Rc::downgrade(&model);
    let live_state = state.downgrade();
    glib::timeout_add_local(REFRESH_INTERVAL, move || {
        match (live_model.upgrade(), live_state.upgrade()) {
            (Some(model), Some(state)) => {
                model.refresh(&state);
                glib::ControlFlow::Continue
            }
            _ => glib::ControlFlow::Break,
        }
    });

    [favorites_group, history_group, forget_group]
}

impl Library {
    fn set_pinned(self: &Rc<Self>, state: &AppState, place_id: u64, pinned: bool) -> bool {
        match Favorites::set_pinned(place_id, pinned) {
            Ok(favorites) => {
                self.favorites.replace(favorites);
                self.render(state);
                true
            }
            Err(error) => {
                state.report_error(&format!("{error:#}"));
                false
            }
        }
    }

    fn refresh(self: &Rc<Self>, state: &AppState) {
        if !self
            .favorite_group
            .upgrade()
            .is_some_and(|group| group.is_mapped())
        {
            return;
        }
        let history = state.server_history();
        let favorites = self.favorites.borrow();
        self.previews.refresh(
            favorites
                .place_ids()
                .iter()
                .copied()
                .chain(history.0.iter().map(|game| game.place_id)),
        );
    }

    fn render(self: &Rc<Self>, state: &AppState) {
        let (Some(favorite_group), Some(history_group), Some(forget_group)) = (
            self.favorite_group.upgrade(),
            self.history_group.upgrade(),
            self.forget_group.upgrade(),
        ) else {
            return;
        };
        let history = state.server_history();
        for row in self
            .favorite_rows
            .borrow_mut()
            .drain(..)
            .filter_map(|row| row.upgrade())
        {
            favorite_group.remove(&row);
        }
        let favorites = self.favorites.borrow();
        for &place_id in favorites.place_ids() {
            let row = self.favorite_row(state, place_id, &history);
            favorite_group.add(&row);
            self.favorite_rows
                .borrow_mut()
                .push(row.upcast::<gtk::Widget>().downgrade());
        }

        let mut expanded = HashSet::new();
        for (place_id, row) in self.history_rows.borrow_mut().drain(..) {
            if let Some(row) = row.upgrade() {
                if row
                    .downcast_ref::<adw::ExpanderRow>()
                    .is_some_and(|row| row.is_expanded())
                {
                    expanded.insert(place_id);
                }
                history_group.remove(&row);
            }
        }
        if history.0.is_empty() {
            let empty = adw::ActionRow::builder()
                .title("No recent experiences")
                .subtitle("Play an experience to see its servers here.")
                .build();
            history_group.add(&empty);
            self.history_rows
                .borrow_mut()
                .push((0, empty.upcast::<gtk::Widget>().downgrade()));
        } else {
            for game in &history.0 {
                let row = self.history_row(state, game, favorites.contains(game.place_id));
                row.set_expanded(expanded.contains(&game.place_id));
                history_group.add(&row);
                self.history_rows
                    .borrow_mut()
                    .push((game.place_id, row.upcast::<gtk::Widget>().downgrade()));
            }
        }
        forget_group.set_visible(!history.0.is_empty());
        self.refresh(state);
    }

    fn favorite_row(
        self: &Rc<Self>,
        state: &AppState,
        place_id: u64,
        history: &ServerHistory,
    ) -> adw::ActionRow {
        let game = history.0.iter().find(|game| game.place_id == place_id);
        let row = adw::ActionRow::builder()
            .title(game_title(place_id, game))
            .use_markup(false)
            .build();
        row.add_prefix(&self.previews.attach(place_id, &row, ""));
        row.add_suffix(&self.pin_button(state, place_id, true));
        if let Some(server) = game.and_then(|game| game.servers.first()) {
            row.add_suffix(&self.rejoin_button(state, place_id, server));
        }
        row.add_suffix(&self.play_button(state, place_id));
        row
    }

    fn history_row(
        self: &Rc<Self>,
        state: &AppState,
        game: &PlayedGame,
        pinned: bool,
    ) -> adw::ExpanderRow {
        let place_id = game.place_id;
        let subtitle = game.last_played_at().map(time_since).unwrap_or_default();
        let row = adw::ExpanderRow::builder()
            .title(game_title(place_id, Some(game)))
            .subtitle(&subtitle)
            .use_markup(false)
            .build();
        row.add_prefix(&self.previews.attach(place_id, &row, &subtitle));
        row.add_suffix(&self.pin_button(state, place_id, pinned));
        if let Some(server) = game.servers.first() {
            row.add_suffix(&self.rejoin_button(state, place_id, server));
        }
        row.add_suffix(&self.play_button(state, place_id));
        for server in &game.servers {
            let server_row = adw::ActionRow::builder()
                .title(
                    server
                        .location
                        .as_deref()
                        .or(server.address.as_deref())
                        .unwrap_or(UNKNOWN_LOCATION),
                )
                .subtitle(time_since(server.joined_at))
                .use_markup(false)
                .build();
            server_row.add_suffix(&self.rejoin_button(state, place_id, server));
            row.add_row(&server_row);
        }
        row
    }

    fn pin_button(self: &Rc<Self>, state: &AppState, place_id: u64, pinned: bool) -> gtk::Button {
        let label = if pinned {
            "Remove from favorites"
        } else {
            "Add to favorites"
        };
        let button = gtk::Button::builder()
            .icon_name(if pinned {
                "starred-symbolic"
            } else {
                "non-starred-symbolic"
            })
            .tooltip_text(label)
            .css_classes(["flat"])
            .valign(gtk::Align::Center)
            .build();
        button.update_property(&[gtk::accessible::Property::Label(label)]);
        button.connect_clicked(glib::clone!(
            #[weak]
            state,
            #[weak(rename_to = model)]
            self,
            move |_| {
                model.set_pinned(&state, place_id, !pinned);
            }
        ));
        button
    }

    fn play_button(&self, state: &AppState, place_id: u64) -> gtk::Button {
        let button = suffix_button("Play");
        button.add_css_class("suggested-action");
        button.connect_clicked(glib::clone!(
            #[weak]
            state,
            move |_| backend::join_game(&state, place_id)
        ));
        self.join_availability.applies_to(&button);
        button
    }

    fn rejoin_button(&self, state: &AppState, place_id: u64, server: &JoinedServer) -> gtk::Button {
        let job_id = server.job_id.clone();
        let button = suffix_button("Rejoin");
        button.set_tooltip_text(Some("Rejoin this Roblox server"));
        button.connect_clicked(glib::clone!(
            #[weak]
            state,
            move |_| backend::rejoin_server(&state, place_id, &job_id)
        ));
        self.join_availability.applies_to(&button);
        button
    }
}

fn game_title(place_id: u64, game: Option<&PlayedGame>) -> String {
    game.filter(|game| !game.name.is_empty())
        .map_or_else(|| format!("Place {place_id}"), |game| game.name.clone())
}

fn suffix_button(label: &str) -> gtk::Button {
    gtk::Button::builder()
        .label(label)
        .valign(gtk::Align::Center)
        .build()
}
