pub(crate) mod absence;
pub(crate) mod action_names;
pub(crate) mod action_read;
mod admit;
pub(crate) mod ancestry;
mod attributes;
mod bounded;
pub(crate) mod capability_rules;
mod child_list;
pub(crate) mod decorate;
mod descriptor;
pub(crate) mod display_bounds;
mod element;
mod expansion;
mod extents;
mod field;
pub(crate) mod frame_geometry;
mod gtk_switch;
mod hit_cover;
pub(crate) mod hit_descent;
pub(crate) mod hit_guard;
pub(crate) mod hit_occlusion;
pub(crate) mod hit_test;
pub(crate) mod hit_verdict;
mod identity;
mod label_outcome;
mod label_read;
pub(crate) mod live_offscreen;
pub(crate) mod live_project;
mod live_read;
pub(crate) mod menu_context;
pub(crate) mod menu_resolve;
mod name_evidence;
pub(crate) mod nested_dialog;
mod node_class;
mod node_evidence;
mod node_position;
mod node_properties;
mod node_read;
mod node_traits;
mod observe;
pub(crate) mod observe_menu;
mod observed_read;
pub(crate) mod offscreen;
mod path_descent;
mod placeholder;
pub(crate) mod placement;
pub(crate) mod projection;
mod raw_node;
mod resolve;
mod resolve_anchor;
mod resolve_errors;
#[cfg(test)]
mod resolve_fake;
mod resolve_gate;
mod resolve_match;
mod resolve_search;
mod resolve_source;
mod resolve_verdict;
pub(crate) mod role_numbers;
mod role_table;
mod roles;
pub(crate) mod scroll_capability;
mod settle;
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
mod capability_live_tests;

#[cfg(test)]
mod chromium_live_tests;

#[cfg(test)]
mod hit_live_tests;

#[cfg(test)]
mod live_apps;

#[cfg(test)]
mod live_churn;

#[cfg(test)]
mod live_cover;

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
mod physical_live_mutter_tests;

#[cfg(test)]
mod physical_live_rig;

#[cfg(test)]
mod physical_live_shell;

#[cfg(test)]
mod physical_live_scripts;

#[cfg(test)]
mod physical_live_tests;

#[cfg(test)]
mod physical_live_x11;

#[cfg(test)]
mod qt_live_tests;

#[cfg(test)]
mod registry_toolkit_live_tests;

#[cfg(test)]
pub(crate) mod live_launch;

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
#[path = "find_live_tests.rs"]
mod find_live_tests;

#[cfg(test)]
#[path = "resolve_live_tests.rs"]
mod resolve_live_tests;

#[cfg(test)]
#[path = "resolve_twin_live_tests.rs"]
mod resolve_twin_live_tests;

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
pub(crate) use menu_resolve::{inside_popup, is_menu_entry, no_position};
pub(crate) use node_class::NodeClass;
pub(crate) use node_properties::NodeProperties;
pub(crate) use node_read::{COMPONENT_INTERFACE, read_node};
pub(crate) use node_traits::NodeTraits;
pub(crate) use observe::{observe_tree, observe_tree_with};
pub(crate) use observed_read::ObservedRead;
pub(crate) use raw_node::{ChildAddress, RawNode};
pub(crate) use resolve_source::{resolve_element_strict, resolve_locator_anchor};
#[cfg(test)]
pub(crate) use vocabulary::{ResolvedVocabulary, resolve_vocabulary};
pub(crate) use walk::Walk;
pub(crate) use walk_counts::WalkCounts;
pub(crate) use walk_limits::WalkLimits;
pub(crate) use walked_node::WalkedNode;
pub(crate) use walker::walk_tree;
