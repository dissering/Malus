use super::bound_rows::choice_row;
use super::mangohud::mangohud_row;
use crate::state::{AppState, PresentationMode, Renderer, SwapInterval, USE_MONITOR_REFRESH_RATE};
use adw::prelude::*;

pub fn graphics_group(state: &AppState) -> adw::PreferencesGroup {
    let renderer_row = choice_row::<Renderer>(state, "renderer", "Renderer");

    state
        .bind_property("vr", &renderer_row, "sensitive")
        .invert_boolean()
        .sync_create()
        .build();

    state
        .bind_property("vr", &renderer_row, "subtitle")
        .transform_to(|_, vr: bool| Some(renderer_subtitle(vr)))
        .sync_create()
        .build();

    let presentation_mode_row =
        choice_row::<PresentationMode>(state, "presentation-mode", "Vulkan presentation mode");
    presentation_mode_row
        .set_subtitle("How frames are shown on screen. Immediate turns V-Sync off.");

    let swap_interval_row =
        choice_row::<SwapInterval>(state, "opengl-swap-interval", "OpenGL swap interval");
    swap_interval_row
        .set_subtitle("Used instead of the presentation mode with the OpenGL renderer.");

    state
        .bind_property("renderer", &presentation_mode_row, "sensitive")
        .transform_to(|_, renderer: Renderer| Some(renderer == Renderer::Metal))
        .sync_create()
        .build();

    state
        .bind_property("renderer", &swap_interval_row, "sensitive")
        .transform_to(|_, renderer: Renderer| Some(renderer == Renderer::OpenGl))
        .sync_create()
        .build();

    let group = adw::PreferencesGroup::builder()
        .title("Graphics")
        .description("Rosemary options for how Roblox draws to the screen.")
        .build();
    group.add(&renderer_row);
    group.add(&presentation_mode_row);
    group.add(&swap_interval_row);
    group.add(&refresh_rate_row(state));
    group.add(&mangohud_row(state));

    group
}

fn renderer_subtitle(vr: bool) -> &'static str {
    if vr {
        "VR needs OpenGL. Turn off Play in VR to change this."
    } else {
        "The graphics API used for Rosemary. Use OpenGL if you have older hardware."
    }
}

pub fn refresh_rate_row(state: &AppState) -> adw::ActionRow {
    let refresh_rate_input = gtk::Entry::builder()
        .placeholder_text("The monitor's")
        .valign(gtk::Align::Center)
        .build();

    let row = adw::ActionRow::builder()
        .title("Refresh rate (Hz)")
        .subtitle("The refresh rate Roblox is told the display has. Default's to the monitor's refresh rate.")
        .build();
    row.add_suffix(&refresh_rate_input);

    state
        .bind_property("refresh-rate", &refresh_rate_input, "text")
        .transform_to(|_, rate: u32| Some(refresh_rate_as_text(rate)))
        .transform_from(|_, text: String| refresh_rate_from_text(&text))
        .bidirectional()
        .sync_create()
        .build();

    row
}

fn refresh_rate_as_text(rate: u32) -> String {
    if rate == USE_MONITOR_REFRESH_RATE {
        String::new()
    } else {
        rate.to_string()
    }
}

fn refresh_rate_from_text(text: &str) -> Option<u32> {
    let text = text.trim();
    if text.is_empty() {
        Some(USE_MONITOR_REFRESH_RATE)
    } else {
        text.parse().ok()
    }
}
