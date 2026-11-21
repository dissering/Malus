use super::controller_access::controller_access_row;
use super::rosemary_options::graphics_group;
use super::rosemary_runtime::rosemary_runtime_group;
use crate::state::AppState;
use adw::prelude::*;

pub fn runtime_section(state: &AppState) -> gtk::Box {
    let section = gtk::Box::new(gtk::Orientation::Vertical, 12);
    let runtime = rosemary_runtime_group(state);
    if let Some(row) = controller_access_row() {
        runtime.add(&row);
    }
    section.append(&runtime);
    section.append(&graphics_group(state));

    section
}
