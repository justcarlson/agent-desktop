pub(crate) mod absence;
pub(crate) mod action_names;
pub(crate) mod action_read;
mod admission;
mod admit;
pub(crate) mod ancestry;
mod attributes;
mod bounded;
mod candidate;
pub(crate) mod capability_rules;
mod child_list;
pub(crate) mod decorate;
mod descriptor;
pub(crate) mod display_bounds;
mod element;
mod expansion;
mod extents;
pub(crate) mod failure;
mod field;
pub(crate) mod frame_geometry;
mod gtk_switch;
mod hit_cover;
pub(crate) mod hit_descent;
pub(crate) mod hit_guard;
pub(crate) mod hit_occlusion;
#[cfg(test)]
mod hit_overlap_live_tests;
pub(crate) mod hit_test;
pub(crate) mod hit_verdict;
mod identity;
mod interface_reads;
mod label_outcome;
mod label_read;
mod landing;
mod level;
pub(crate) mod live_offscreen;
pub(crate) mod live_project;
pub(crate) mod live_read;
pub(crate) mod liveness;
pub(crate) mod menu_context;
mod menu_identity;
pub(crate) mod menu_resolve;
mod modal_gate;
mod name_evidence;
pub(crate) mod nested_dialog;
mod node_class;
mod node_evidence;
mod node_position;
mod node_properties;
mod node_read;
mod node_traits;
mod observation;
mod observe;
pub(crate) mod observe_menu;
mod observed_read;
pub(crate) mod offscreen;
mod path_descent;
mod placed_states;
mod placeholder;
pub(crate) mod placement;
mod popup_position;
pub(crate) mod projection;
mod raw_node;
pub(crate) mod read_failure;
mod read_purpose;
mod readiness;
mod readiness_step;
mod resolution;
mod resolve;
mod resolve_anchor;
mod resolve_candidates;
mod resolve_errors;
#[cfg(test)]
mod resolve_fake;
mod resolve_gate;
mod resolve_identity;
mod resolve_search;
mod resolve_source;
pub(crate) mod role_numbers;
mod role_table;
mod roles;
pub(crate) mod scroll_capability;
mod searched;
pub(crate) mod state_bits;
mod states;
pub(crate) mod surfaces;
pub(crate) mod text_selection;
pub(crate) mod text_units;
pub(crate) mod toolkit_trust;
mod value_read;
mod vocabulary;
mod walk;
mod walk_counts;
mod walk_limits;
mod walked_node;
mod walker;
mod walker_expand;
pub(crate) mod window_extents;
mod wrapper;

#[cfg(test)]
#[path = "observe_live_tests.rs"]
mod observe_live_tests;

#[cfg(test)]
#[path = "walker_depth_tests.rs"]
mod walker_depth_tests;

#[cfg(test)]
#[path = "walker_tests.rs"]
mod walker_tests;

#[cfg(test)]
pub(crate) mod test_support;

#[cfg(test)]
mod absence_live_tests;

#[cfg(test)]
mod chromium_live_tests;

#[cfg(test)]
mod hit_live_tests;

#[cfg(test)]
mod live_apps;

#[cfg(test)]
mod live_horizontal_live_tests;

#[cfg(test)]
mod live_offscreen_live_tests;

#[cfg(test)]
mod live_fixture;

#[cfg(test)]
mod live_read_live_tests;

#[cfg(test)]
mod live_rig;

#[cfg(test)]
mod navigate_live_tests;

#[cfg(test)]
mod physical_live_apps_tests;

#[cfg(test)]
mod physical_live_rig;

#[cfg(test)]
mod physical_live_scripts;

#[cfg(test)]
mod physical_live_tests;

#[cfg(test)]
mod sheet_live_tests;

#[cfg(test)]
mod physical_live_x11;

#[cfg(test)]
mod qt_live_tests;

#[cfg(test)]
mod registry_toolkit_live_tests;

#[cfg(test)]
pub(crate) mod live_launch;

#[cfg(test)]
pub(crate) mod live_process;

#[cfg(test)]
mod live_qt;

#[cfg(test)]
mod live_timeout;

#[cfg(test)]
mod live_tree;

#[cfg(test)]
#[path = "vocabulary_live_analysis.rs"]
mod vocabulary_live_analysis;

#[cfg(test)]
#[path = "vocabulary_live_tests.rs"]
mod vocabulary_live_tests;

#[cfg(test)]
#[path = "walker_live_tests.rs"]
mod walker_live_tests;

#[cfg(test)]
#[path = "resolve_live_tests.rs"]
mod resolve_live_tests;

pub(crate) use child_list::list_children;
pub(crate) use element::AtspiElement;
pub(crate) use element::atspi_element;
pub(crate) use expansion::Expansion;
pub(crate) use extents::Extents;
pub(crate) use field::Field;
pub(crate) use hit_test::hit_test;
pub(crate) use label_outcome::LabelOutcome;
#[cfg(test)]
pub(crate) use label_read::read_label;
pub(crate) use live_read::{
    get_element_bounds, get_live_actions, get_live_element, get_live_state, get_live_value,
};
pub(crate) use menu_resolve::is_menu_entry;
pub(crate) use node_class::NodeClass;
pub(crate) use node_properties::NodeProperties;
pub(crate) use node_read::{COMPONENT_INTERFACE, call, read_node, read_node_for};
pub(crate) use node_traits::NodeTraits;
pub(crate) use observe::observe_tree;
pub(crate) use observed_read::ObservedRead;
pub(crate) use raw_node::{ChildAddress, RawNode};
pub(crate) use read_purpose::ReadPurpose;
pub(crate) use resolve_source::{resolve_element_strict, resolve_locator_anchor};
#[cfg(test)]
pub(crate) use vocabulary::{ResolvedVocabulary, resolve_vocabulary};
pub(crate) use walk::Walk;
pub(crate) use walk_counts::WalkCounts;
pub(crate) use walk_limits::WalkLimits;
pub(crate) use walked_node::WalkedNode;

#[cfg(test)]
mod evidence_live_tests;
