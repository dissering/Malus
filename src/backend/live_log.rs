use crate::state::{AppState, Client};
use gtk::{gio, glib, prelude::*};
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, SyncSender};
use std::time::Duration;

const CHECK_FOR_NEW_OUTPUT_EVERY: Duration = Duration::from_millis(200);
const READ_CHUNK_SIZE: usize = 8192;
const LOG_QUEUE_CAPACITY: usize = 8;

pub struct LogCompletion {
    writer_finished: Arc<AtomicBool>,
    changes: SyncSender<()>,
}

impl LogCompletion {
    pub fn finish(self) {
        self.writer_finished.store(true, Ordering::Release);
        let _ = self.changes.try_send(());
    }
}

fn started_marker(client: Client) -> &'static str {
    match client {
        Client::Player => "[FLog::Graphics]",
        Client::Studio => "[FLog::StudioMain]",
    }
}

pub fn follow(state: &AppState, path: &Path, log: File) -> (LogCompletion, glib::JoinHandle<()>) {
    state.clear_log();

    let writer_finished = Arc::new(AtomicBool::new(false));
    let (changes, notifications) = mpsc::sync_channel(1);
    let monitor = watch_log(path, changes.clone());
    let completion = LogCompletion {
        writer_finished: writer_finished.clone(),
        changes,
    };
    let (output_sender, output_receiver) = async_channel::bounded(LOG_QUEUE_CAPACITY);
    std::thread::spawn(move || {
        read_until_writer_finishes(log, &writer_finished, &notifications, &output_sender)
    });

    let state = state.clone();
    let drain = glib::spawn_future_local(async move {
        let _monitor = monitor;
        while let Ok(mut output) = output_receiver.recv().await {
            let mut batch_size = 1;
            while batch_size < LOG_QUEUE_CAPACITY {
                let Ok(more_output) = output_receiver.try_recv() else {
                    break;
                };
                output.push_str(&more_output);
                batch_size += 1;
            }
            state.append_log(&output);
            if !state.roblox_graphics_started()
                && output.contains(started_marker(state.running_client()))
            {
                state.set_roblox_graphics_started(true);
            }
            if batch_size == LOG_QUEUE_CAPACITY {
                glib::timeout_future(Duration::from_millis(1)).await;
            }
        }
    });
    (completion, drain)
}

fn watch_log(path: &Path, changes: SyncSender<()>) -> Option<gio::FileMonitor> {
    let monitor = gio::File::for_path(path)
        .monitor_file(gio::FileMonitorFlags::NONE, gio::Cancellable::NONE)
        .ok()?;
    monitor.set_rate_limit(0);
    monitor.connect_changed(move |_, _, _, _| {
        let _ = changes.try_send(());
    });
    Some(monitor)
}

fn read_until_writer_finishes(
    mut log: File,
    writer_finished: &AtomicBool,
    notifications: &Receiver<()>,
    output_sender: &async_channel::Sender<String>,
) {
    let mut unfinished_line = Vec::new();
    let mut chunk = [0; READ_CHUNK_SIZE];

    loop {
        let writer_had_finished = writer_finished.load(Ordering::Acquire);
        let read = match log.read(&mut chunk) {
            Ok(read) => read,
            Err(error) => {
                eprintln!("Stopped following the Roblox log: {error}");
                break;
            }
        };

        if read == 0 {
            if writer_had_finished {
                break;
            }
            if let Err(RecvTimeoutError::Disconnected) =
                notifications.recv_timeout(CHECK_FOR_NEW_OUTPUT_EVERY)
            {
                break;
            }
            continue;
        }

        unfinished_line.extend_from_slice(&chunk[..read]);
        if let Some(last_newline) = unfinished_line.iter().rposition(|&byte| byte == b'\n') {
            let consumed = last_newline + 1;
            let complete_lines = as_text(&unfinished_line[..consumed]);
            unfinished_line.drain(..consumed);
            if output_sender.send_blocking(complete_lines).is_err() {
                return;
            }
        }
    }

    if !unfinished_line.is_empty() {
        let _ = output_sender.send_blocking(as_text(&unfinished_line));
    }
}

fn as_text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes).into_owned()
}
