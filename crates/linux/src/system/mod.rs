mod adapter;
mod app_name;
mod app_ops;
mod bus;
mod bus_deadline;
mod bus_fault;
mod bus_process;
mod bus_slice;
mod bus_stage;
mod display;
mod display_mutter;
mod display_probe;
mod display_server;
mod focus;
mod interaction_lease;
mod listed_window;
mod permissions;
mod proc_scan;
pub(crate) mod process_facts;
mod registry;
mod registry_app;
mod registry_frames;
mod session;
mod surfaces;
mod toolkit;
mod toolkit_id;
mod window_filter;
pub(crate) mod window_id;
mod window_listing;
mod window_ops;
mod window_resolve;
pub(crate) mod x11;

pub(crate) use bus::{ACCESSIBLE_INTERFACE, PROPERTIES_INTERFACE, SharedBus, classify};
pub(crate) use bus::{REGISTRY_ROOT_PATH, REGISTRY_SERVICE};
pub(crate) use bus_deadline::within;
pub(crate) use bus_fault::{NAME_HAS_NO_OWNER, SERVICE_UNKNOWN};
pub(crate) use bus_process::process_id;

pub(crate) use app_ops::{inventory as app_inventory, scoped as apps_scoped};
pub(crate) use display::list_displays;
pub(crate) use surfaces::list_surfaces;
pub(crate) use window_ops::{focused_window, list_windows};
pub(crate) use window_resolve::resolve_window_strict;

#[cfg(test)]
mod registry_live_tests;
