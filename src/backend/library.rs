use super::background::run_quietly;
use super::game_details::universe_for_place;
use super::req_agent::{api_agent, get_json};
use anyhow::{Context, ensure};
use gtk::glib;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{ErrorKind, Read};
use std::path::{Path, PathBuf};
use std::thread;

const GAME_BATCH_SIZE: usize = 25;
const PARALLEL_REQUESTS: usize = 4;
const MAX_THUMBNAIL_BYTES: u64 = 512 * 1024;
const FAVORITES_FILE: &str = "favorites.json";

#[derive(Clone, Default)]
pub struct Favorites(Vec<u64>);

impl Favorites {
    pub fn load() -> anyhow::Result<Self> {
        Self::load_from(&favorites_file())
    }

    fn load_from(path: &Path) -> anyhow::Result<Self> {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Self::default()),
            Err(error) => return Err(error).context("Couldn't read favorites"),
        };
        let mut place_ids: Vec<u64> =
            serde_json::from_slice(&bytes).context("Couldn't read favorites")?;
        let mut seen = HashSet::new();
        place_ids.retain(|id| *id > 0 && seen.insert(*id));
        Ok(Self(place_ids))
    }

    pub fn place_ids(&self) -> &[u64] {
        &self.0
    }

    pub fn contains(&self, place_id: u64) -> bool {
        self.0.contains(&place_id)
    }

    pub fn set_pinned(place_id: u64, pinned: bool) -> anyhow::Result<Self> {
        Self::set_in(&favorites_file(), place_id, pinned)
    }

    fn set_in(path: &Path, place_id: u64, pinned: bool) -> anyhow::Result<Self> {
        ensure!(place_id > 0, "Enter a valid Roblox place ID.");
        let mut favorites = Self::load_from(path)?;
        let known = favorites.contains(place_id);
        if known == pinned {
            return Ok(favorites);
        }
        if pinned {
            favorites.0.push(place_id);
        } else {
            favorites.0.retain(|&id| id != place_id);
        }
        crate::utils::create_parent_directory(path)?;
        glib::file_set_contents(path, &serde_json::to_vec_pretty(&favorites.0)?)
            .context("Couldn't save favorites")?;
        Ok(favorites)
    }
}

fn favorites_file() -> PathBuf {
    super::paths::server_history_file().with_file_name(FAVORITES_FILE)
}

#[derive(Clone)]
pub struct PreviewRequest {
    pub place_id: u64,
    pub universe_id: Option<u64>,
    pub needs_thumbnail: bool,
}

pub struct GamePreview {
    pub place_id: u64,
    pub universe_id: u64,
    pub name: Option<String>,
    pub playing: Option<u64>,
    pub thumbnail: Option<Vec<u8>>,
}

#[derive(Deserialize)]
struct Listing<T> {
    data: Vec<T>,
}

#[derive(Deserialize)]
struct PreviewDetails {
    id: u64,
    name: String,
    playing: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UniverseThumbnails {
    universe_id: u64,
    thumbnails: Vec<Thumbnail>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Thumbnail {
    state: String,
    image_url: Option<String>,
}

pub fn load_game_previews(
    requests: Vec<PreviewRequest>,
    when_finished: impl FnOnce(Vec<GamePreview>) + 'static,
) {
    run_quietly(
        move || Ok(fetch_game_previews(&requests)),
        move |result| match result {
            Ok(previews) => when_finished(previews),
            Err(error) => {
                eprintln!("Couldn't load game previews: {error:#}");
                when_finished(Vec::new());
            }
        },
    );
}

fn fetch_game_previews(requests: &[PreviewRequest]) -> Vec<GamePreview> {
    let resolved = in_parallel(requests, |request| {
        let universe = request
            .universe_id
            .or_else(|| universe_for_place(request.place_id).ok())
            .filter(|id| *id > 0)?;
        Some((request.clone(), universe))
    });
    let mut game_info = HashMap::new();
    let ids = resolved
        .iter()
        .map(|(_, id)| *id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    for ids in ids.chunks(GAME_BATCH_SIZE) {
        let url = format!(
            "https://games.roblox.com/v1/games?universeIds={}",
            joined_ids(ids)
        );
        if let Ok(details) = get_json::<Listing<PreviewDetails>>(&url) {
            game_info.extend(details.data.into_iter().map(|game| (game.id, game)));
        }
    }

    let image_ids = resolved
        .iter()
        .filter(|(request, _)| request.needs_thumbnail)
        .map(|(_, id)| *id)
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut thumbnails = HashMap::new();
    for ids in image_ids.chunks(GAME_BATCH_SIZE) {
        let url = format!(
            "https://thumbnails.roblox.com/v1/games/multiget/thumbnails?universeIds={}&countPerUniverse=1&defaults=true&size=256x144&format=Png&isCircular=false",
            joined_ids(ids)
        );
        if let Ok(images) = get_json::<Listing<UniverseThumbnails>>(&url) {
            let ready = images
                .data
                .into_iter()
                .filter_map(|group| {
                    let image_url = group
                        .thumbnails
                        .into_iter()
                        .find(|image| image.state == "Completed")?
                        .image_url?;
                    Some((group.universe_id, image_url))
                })
                .collect::<Vec<_>>();
            thumbnails.extend(in_parallel(&ready, |(id, url)| {
                download_thumbnail(url).ok().map(|bytes| (*id, bytes))
            }));
        }
    }

    resolved
        .into_iter()
        .map(|(request, universe_id)| {
            let details = game_info.get(&universe_id);
            GamePreview {
                place_id: request.place_id,
                universe_id,
                name: details.map(|info| info.name.clone()),
                playing: details.and_then(|info| info.playing),
                thumbnail: thumbnails.get(&universe_id).cloned(),
            }
        })
        .collect()
}

fn joined_ids(ids: &[u64]) -> String {
    ids.iter().map(u64::to_string).collect::<Vec<_>>().join(",")
}

fn in_parallel<T: Sync, R: Send>(jobs: &[T], work: impl Fn(&T) -> Option<R> + Sync) -> Vec<R> {
    let chunk_size = jobs.len().div_ceil(PARALLEL_REQUESTS).max(1);
    thread::scope(|scope| {
        let threads = jobs
            .chunks(chunk_size)
            .map(|batch| {
                let work = &work;
                scope.spawn(move || batch.iter().filter_map(work).collect::<Vec<_>>())
            })
            .collect::<Vec<_>>();
        threads
            .into_iter()
            .flat_map(|worker| worker.join().unwrap_or_default())
            .collect()
    })
}

fn download_thumbnail(url: &str) -> anyhow::Result<Vec<u8>> {
    ensure!(
        url.strip_prefix("https://")
            .and_then(|rest| rest.split_once('/'))
            .is_some_and(|(host, _)| host == "rbxcdn.com" || host.ends_with(".rbxcdn.com")),
        "Unexpected Roblox thumbnail URL"
    );
    let mut response = api_agent().get(url).call()?;
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(MAX_THUMBNAIL_BYTES + 1)
        .read_to_end(&mut bytes)?;
    ensure!(
        bytes.len() as u64 <= MAX_THUMBNAIL_BYTES,
        "Roblox thumbnail exceeds its size limit"
    );
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pins_persist_without_duplicates_and_unpin_by_id() -> anyhow::Result<()> {
        let directory = std::env::temp_dir().join(glib::uuid_string_random().as_str());
        let path = directory.join(FAVORITES_FILE);
        let run = || -> anyhow::Result<()> {
            assert!(Favorites::load_from(&path)?.place_ids().is_empty());
            Favorites::set_in(&path, 920587237, true)?;
            Favorites::set_in(&path, 920587237, true)?;
            Favorites::set_in(&path, 606849621, true)?;
            assert_eq!(
                Favorites::load_from(&path)?.place_ids(),
                &[920587237, 606849621]
            );
            Favorites::set_in(&path, 920587237, false)?;
            assert_eq!(Favorites::load_from(&path)?.place_ids(), &[606849621]);
            fs::write(&path, b"invalid JSON")?;
            assert!(Favorites::set_in(&path, 920587237, true).is_err());
            assert_eq!(fs::read(&path)?, b"invalid JSON");
            Ok(())
        };
        let result = run();
        fs::remove_dir_all(directory)?;
        result
    }
}
