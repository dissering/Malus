use crate::backend::{self, GamePreview, PreviewRequest};
use adw::prelude::*;
use gtk::{gdk, gdk_pixbuf, gio, glib};
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::time::{Duration, Instant};

pub const REFRESH_INTERVAL: Duration = Duration::from_secs(60);
const MAX_PREVIEWS_PER_BATCH: usize = 12;
const THUMBNAIL_WIDTH: i32 = 80;
const THUMBNAIL_HEIGHT: i32 = 45;

#[derive(Default)]
pub struct Previews {
    cached: RefCell<HashMap<u64, CachedPreview>>,
    listeners: RefCell<Vec<Listener>>,
    wanted: RefCell<Vec<u64>>,
    working: Cell<bool>,
    requested_at: Cell<Option<Instant>>,
}

#[derive(Default)]
struct CachedPreview {
    universe_id: Option<u64>,
    name: Option<String>,
    playing: Option<u64>,
    picture: Option<gdk::Texture>,
    last_request: Option<Instant>,
}

struct Listener {
    place_id: u64,
    row: glib::WeakRef<adw::PreferencesRow>,
    stack: glib::WeakRef<gtk::Stack>,
    picture: glib::WeakRef<gtk::Picture>,
    original_title: String,
    original_subtitle: String,
}

impl Previews {
    pub fn attach(
        &self,
        place_id: u64,
        row: &impl IsA<adw::PreferencesRow>,
        subtitle: &str,
    ) -> gtk::Stack {
        let picture = gtk::Picture::builder()
            .content_fit(gtk::ContentFit::Cover)
            .build();
        let stack = gtk::Stack::builder()
            .width_request(THUMBNAIL_WIDTH)
            .height_request(THUMBNAIL_HEIGHT)
            .valign(gtk::Align::Center)
            .overflow(gtk::Overflow::Hidden)
            .css_classes(["card"])
            .build();
        stack.add_named(
            &gtk::Image::from_icon_name("image-x-generic-symbolic"),
            Some("placeholder"),
        );
        stack.add_named(&picture, Some("thumbnail"));
        let row = row.upcast_ref::<adw::PreferencesRow>();
        let listener = Listener {
            place_id,
            row: row.downgrade(),
            stack: stack.downgrade(),
            picture: picture.downgrade(),
            original_title: row.title().to_string(),
            original_subtitle: subtitle.to_owned(),
        };
        if let Some(preview) = self.cached.borrow().get(&place_id) {
            listener.update(preview);
        }
        let mut listeners = self.listeners.borrow_mut();
        listeners.retain(|listener| listener.row.upgrade().is_some());
        listeners.push(listener);
        stack
    }

    pub fn refresh(self: &Rc<Self>, place_ids: impl IntoIterator<Item = u64>) {
        let mut seen = HashSet::new();
        let wanted = place_ids
            .into_iter()
            .filter(|id| seen.insert(*id))
            .collect();
        self.wanted.replace(wanted);
        self.requested_at.set(Some(Instant::now()));
        self.start_fetch();
    }

    pub fn hide(&self) {
        self.wanted.borrow_mut().clear();
    }

    fn start_fetch(self: &Rc<Self>) {
        if self.working.get() {
            return;
        }

        let Some(now) = self.requested_at.get() else {
            return;
        };
        let mut cached = self.cached.borrow_mut();
        let wanted = self.wanted.borrow();
        let wanted_ids = wanted.iter().copied().collect::<HashSet<_>>();
        if !wanted_ids.is_empty() {
            cached.retain(|id, _| wanted_ids.contains(id));
        }
        let requests = wanted
            .iter()
            .filter_map(|&place_id| {
                let item = cached.entry(place_id).or_default();
                if item
                    .last_request
                    .is_some_and(|last| now.saturating_duration_since(last) < REFRESH_INTERVAL)
                {
                    return None;
                }
                item.last_request = Some(now);
                Some(PreviewRequest {
                    place_id,
                    universe_id: item.universe_id,
                    needs_thumbnail: item.picture.is_none(),
                })
            })
            .take(MAX_PREVIEWS_PER_BATCH)
            .collect::<Vec<_>>();
        if requests.is_empty() {
            return;
        }
        self.working.set(true);
        let model = Rc::downgrade(self);
        backend::load_game_previews(requests, move |previews| {
            if let Some(model) = model.upgrade() {
                model.working.set(false);
                model.update(previews);
                model.start_fetch();
            }
        });
    }

    fn update(&self, previews: Vec<GamePreview>) {
        let mut cached = self.cached.borrow_mut();
        let changed = previews
            .into_iter()
            .filter_map(|preview| {
                let current = cached.get_mut(&preview.place_id)?;
                current.universe_id = Some(preview.universe_id);
                if let Some(name) = preview.name {
                    current.name = Some(name);
                }
                if let Some(playing) = preview.playing {
                    current.playing = Some(playing);
                }
                if let Some(bytes) = preview.thumbnail {
                    let bytes = glib::Bytes::from_owned(bytes);
                    let stream = gio::MemoryInputStream::from_bytes(&bytes);
                    if let Ok(picture) = gdk_pixbuf::Pixbuf::from_stream_at_scale(
                        &stream,
                        THUMBNAIL_WIDTH,
                        THUMBNAIL_HEIGHT,
                        true,
                        gio::Cancellable::NONE,
                    ) {
                        current.picture = Some(gdk::Texture::for_pixbuf(&picture));
                    }
                }
                Some(preview.place_id)
            })
            .collect::<HashSet<_>>();
        self.listeners.borrow_mut().retain(|listener| {
            if listener.row.upgrade().is_none() {
                return false;
            }
            if changed.contains(&listener.place_id) {
                if let Some(preview) = cached.get(&listener.place_id) {
                    listener.update(preview);
                }
            }
            true
        });
    }
}

impl Listener {
    fn update(&self, preview: &CachedPreview) {
        let Some(row) = self.row.upgrade() else {
            return;
        };
        row.set_title(preview.name.as_deref().unwrap_or(&self.original_title));
        let subtitle = match preview.playing {
            Some(playing) if self.original_subtitle.is_empty() => {
                format!("{} playing", player_count(playing))
            }
            Some(playing) => format!(
                "{} playing · {}",
                player_count(playing),
                self.original_subtitle
            ),
            None => self.original_subtitle.clone(),
        };
        if let Some(row) = row.downcast_ref::<adw::ExpanderRow>() {
            row.set_subtitle(&subtitle);
        } else if let Some(row) = row.downcast_ref::<adw::ActionRow>() {
            row.set_subtitle(&subtitle);
        }
        if let (Some(paintable), Some(picture), Some(stack)) = (
            &preview.picture,
            self.picture.upgrade(),
            self.stack.upgrade(),
        ) {
            picture.set_paintable(Some(paintable));
            stack.set_visible_child_name("thumbnail");
        }
    }
}

fn player_count(playing: u64) -> String {
    let digits = playing.to_string();
    let mut formatted = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            formatted.push(',');
        }
        formatted.push(digit);
    }
    formatted
}
