mod install;
mod install_kind;
mod release;
pub mod version;

use super::background::{run_quietly, run_task};
use crate::state::AppState;
use gtk::glib;
use install_kind::InstallKind;
use release::Update;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::time::Duration;

const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(5);
const CHECK_INTERVAL: Duration = Duration::from_secs(60 * 60);

thread_local! {
    static INSTALL_KIND: InstallKind = InstallKind::detect();
    static AVAILABLE: RefCell<Option<Update>> = const { RefCell::new(None) };
    static UPDATED: Cell<bool> = const { Cell::new(false) };
}

pub fn running_appimage() -> Option<PathBuf> {
    match INSTALL_KIND.with(Clone::clone) {
        InstallKind::AppImage(path) => Some(path),
        InstallKind::Flatpak | InstallKind::Portable(_) | InstallKind::Other => None,
    }
}

pub fn launcher_can_update_itself() -> bool {
    INSTALL_KIND.with(|kind| kind.update_target().is_some())
}

pub fn initialize(state: &AppState) {
    state.connect_properties_notify(&["busy", "running", "auto-update-launcher"], offer_update);
    let weak_state = glib::prelude::ObjectExt::downgrade(state);
    glib::timeout_add_local_once(FIRST_CHECK_DELAY, move || {
        let Some(state) = weak_state.upgrade() else {
            return;
        };
        check_launcher_update(&state);
    });
    let weak_state = glib::prelude::ObjectExt::downgrade(state);
    glib::timeout_add_local(CHECK_INTERVAL, move || {
        if let Some(state) = weak_state.upgrade().filter(|_| !UPDATED.get()) {
            check_launcher_update(&state);
            glib::ControlFlow::Continue
        } else {
            glib::ControlFlow::Break
        }
    });
}

pub fn check_launcher_update(state: &AppState) {
    if state.launcher_update_checking() || UPDATED.get() {
        return;
    }
    state.set_launcher_update_checking(true);
    state.set_launcher_update_status("Checking for updates…");
    let state = state.clone();
    let install_kind = INSTALL_KIND.with(Clone::clone);
    run_quietly(
        move || release::newest_after(CURRENT_VERSION, &install_kind),
        move |result| {
            state.set_launcher_update_checking(false);
            match result {
                Ok(update) => {
                    AVAILABLE.replace(update);
                    if AVAILABLE.with_borrow(Option::is_some) {
                        offer_update(&state);
                    } else {
                        state.set_launcher_update_status("Malus is up to date");
                    }
                }
                Err(error) => {
                    eprintln!("Couldn't check for Malus updates: {error:#}");
                    state.set_launcher_update_status("Couldn't check for updates");
                }
            }
        },
    );
}

fn offer_update(state: &AppState) {
    if state.launcher_update_checking() || UPDATED.get() {
        return;
    }
    let Some((version, can_download)) = AVAILABLE.with_borrow(|available| {
        available
            .as_ref()
            .map(|update| (update.version.clone(), update.download.is_some()))
    }) else {
        return;
    };
    let install_kind = INSTALL_KIND.with(Clone::clone);
    if !can_download || install_kind.update_target().is_none() || !state.auto_update_launcher() {
        let suffix = match install_kind {
            InstallKind::Flatpak => ". Update through Flatpak",
            InstallKind::Other => ". Install the matching package from GitHub",
            _ => ". See GitHub Releases",
        };
        state.set_launcher_update_status(format!("Malus {version} is available{suffix}"));
    } else if state.running() {
        state
            .set_launcher_update_status(format!("Malus {version} will install after this session"));
    } else if state.busy() {
        state.set_launcher_update_status(format!("Malus {version} is queued to install"));
    } else if let Some(update) = AVAILABLE.take() {
        install_when_idle(state, install_kind, update);
    }
}

fn install_when_idle(state: &AppState, install_kind: InstallKind, update: Update) {
    let Some(download) = update.download else {
        return;
    };
    let version = update.version;
    let kind_for_stage = install_kind.clone();
    state.set_launcher_update_checking(true);
    state.set_launcher_update_status(format!("Installing Malus {version}…"));
    run_task(
        state,
        move |progress| {
            let result = install::stage(&kind_for_stage, &download, progress);
            Ok((result, download))
        },
        move |state, (staged, download)| {
            if state.running() || !state.auto_update_launcher() {
                AVAILABLE.replace(Some(Update {
                    version,
                    download: Some(download),
                }));
                state.set_launcher_update_checking(false);
                offer_update(state);
                return;
            }
            match staged.and_then(|update| update.commit(&install_kind)) {
                Ok(()) => {
                    UPDATED.set(true);
                    state.set_launcher_update_status(format!("Reopen Malus to use {version}"));
                }
                Err(error) => {
                    eprintln!("Couldn't install Malus {version}: {error:#}");
                    state.set_launcher_update_status(format!("Couldn't install Malus {version}"));
                    state.set_launcher_update_checking(false);
                }
            }
        },
    );
}
