use super::bound_rows::choice_row;
use super::mangohud::mangohud_row;
use crate::state::{AppState, PresentationMode, Renderer, SwapInterval};
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

    let group = adw::PreferencesGroup::builder().title("Graphics").build();
    group.add(&renderer_row);
    group.add(&mangohud_row(state));

    group
}

pub fn graphics_tuning_group(state: &AppState) -> adw::PreferencesGroup {
    let presentation_mode_row =
        choice_row::<PresentationMode>(state, "presentation-mode", "Vulkan presentation mode");
    presentation_mode_row.set_subtitle("Immediate turns V-Sync off.");

    let swap_interval_row =
        choice_row::<SwapInterval>(state, "opengl-swap-interval", "OpenGL swap interval");

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

    let group = adw::PreferencesGroup::builder().title("Graphics").build();
    group.add(&presentation_mode_row);
    group.add(&swap_interval_row);

    group
}

fn renderer_subtitle(vr: bool) -> &'static str {
    if vr {
        "VR needs OpenGL. Turn off Play in VR to change this."
    } else {
        "The graphics API used for Rosemary. Use OpenGL if you have older hardware."
    }
}
