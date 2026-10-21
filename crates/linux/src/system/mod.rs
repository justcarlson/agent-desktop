pub(crate) mod action_clock;
mod action_clock_store;
mod adapter;
pub(crate) mod alias_cache;
mod app_alias;
mod app_name;
mod app_names;
mod app_ops;
pub(crate) mod app_outcome;
mod bus;
mod bus_deadline;
mod bus_fault;
mod bus_process;
mod bus_slice;
mod bus_stage;
pub(crate) mod capture;
mod close;
mod close_plan;
mod cursor_overlay;
mod desktop_entry;
pub(crate) mod desktop_entry_traits;
mod desktop_exec;
mod desktop_lookup;
mod display;
pub(crate) mod display_cause;
pub(crate) mod display_env;
mod display_error;
mod display_mutter;
mod display_probe;
pub(crate) mod display_reach;
pub(crate) mod display_server;
pub(crate) mod display_unavailable;
mod focus;
pub(crate) mod focus_cause;
pub(crate) mod focus_client_view;
pub(crate) mod focus_frame_view;
pub(crate) mod focus_window;
mod focus_world;
pub(crate) mod frame_read;
mod ghost_window;
pub(crate) mod interaction_lease;
mod key_dispatch;
mod launch;
mod launch_attach;
mod launch_observe;
pub(crate) mod launch_sample;
pub(crate) mod launch_search;
pub(crate) mod launch_step;
mod launch_target;
mod listed_window;
pub(crate) mod liveness;
mod menu_anchor;
mod menu_focus;
pub(crate) mod menu_found;
mod menu_popup;
mod menu_reader;
pub(crate) mod menu_surface;
mod menu_wait;
mod nested_scan;
pub(crate) mod nested_surface;
pub(crate) mod overview;
mod permissions;
mod proc_scan;
pub(crate) mod proc_stat;
pub(crate) mod proc_state;
pub(crate) mod process_facts;
pub(crate) mod process_lookup;
mod process_state;
mod process_state_classify;
mod protected;
pub(crate) mod registry;
mod registry_app;
mod registry_frames;
mod restore_minimized;
pub(crate) mod route;
pub(crate) mod running_app;
mod runtime_store;
mod screenshot;
mod session;
pub(crate) mod session_bus;
pub(crate) mod shell;
mod signal;
mod signal_boundary;
mod signal_capture;
mod signal_inventory;
mod signal_population;
pub(crate) mod signal_sent;
mod signal_surfaces;
mod stopped_windows;
mod surfaces;
mod toolkit;
pub(crate) mod toolkit_id;
mod unresponsive;
pub(crate) mod web_family;
mod window_client;
pub(crate) mod window_filter;
pub(crate) mod window_id;
mod window_ids;
mod window_listing;
mod window_op;
mod window_op_shadow;
mod window_op_verdict;
mod window_ops;
pub(crate) mod window_position;
mod window_report;
mod window_resolve;
pub(crate) mod x11;

pub(crate) use app_name::app_name as listed_app_name;
pub(crate) use bus::ensure_budget;
pub(crate) use bus::{
    ACCESSIBLE_INTERFACE, PROPERTIES_INTERFACE, SharedBus, classify, is_connection_failure,
};
pub(crate) use bus::{REGISTRY_ROOT_PATH, REGISTRY_SERVICE};
pub(crate) use bus_deadline::within;
pub(crate) use bus_fault::{NAME_HAS_NO_OWNER, SERVICE_UNKNOWN};
pub(crate) use bus_process::{owner_process_id, process_id};

pub(crate) use app_ops::{inventory as app_inventory, scoped as apps_scoped};
pub(crate) use display::{list_displays, list_wayland_displays};
pub(crate) use surfaces::list_surfaces;
pub(crate) use window_ops::{focused_window, list_windows};
pub(crate) use window_resolve::resolve_window_strict;

#[cfg(test)]
mod close_live_tests;
#[cfg(test)]
mod display_live_tests;
#[cfg(test)]
mod focus_window_live_tests;
pub(crate) mod frame_identity;
#[cfg(test)]
mod key_dispatch_live_tests;
#[cfg(test)]
mod launch_live_tests;
#[cfg(test)]
mod menu_fixture;
#[cfg(test)]
mod menu_live_rig;
#[cfg(test)]
mod menu_surface_live_tests;
#[cfg(test)]
mod menu_wait_live_tests;
#[cfg(test)]
mod process_state_live_tests;
#[cfg(test)]
mod registry_live_tests;
#[cfg(test)]
mod sheet_live_tests;
#[cfg(test)]
mod signal_live_apps;
#[cfg(test)]
mod signal_live_surfaces_tests;
#[cfg(test)]
mod signal_live_tests;
#[cfg(test)]
mod stopped_live_tests;
#[cfg(test)]
mod window_op_live_tests;
