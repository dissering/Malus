use super::rosemary_options::{graphics_group, input_group};
use super::rosemary_runtime::rosemary_runtime_group;
use crate::state::AppState;
use adw::prelude::*;

pub fn runtime_section(state: &AppState) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    section.append(&rosemary_runtime_group(state));
    section.append(&graphics_group(state));
    section.append(&input_group(state));

    section
}
