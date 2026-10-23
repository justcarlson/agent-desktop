#![cfg_attr(not(test), deny(clippy::unwrap_used, clippy::expect_used))]

mod accname;
pub mod action;
pub mod action_request;
pub mod action_result;
pub mod action_step;
pub mod action_step_outcome;
pub(crate) mod actionability;
pub mod adapter;
mod app_info;
mod app_inventory;
mod app_inventory_query;
mod app_lookup;
mod bounds_space;
pub mod capability;
pub mod cdp_endpoint;
pub mod commands;
mod containment_predicate;
pub mod context;
pub mod cursor_overlay;
mod delivery_semantics;
pub mod display_info;
mod element_identifier;
pub mod element_state;
mod headed_focus;
mod headed_requirement;
pub mod hints;
mod identifier_kind;
mod identity_predicate;
mod image_buffer;
mod image_format;
mod input;
pub mod interaction_policy;
pub mod launch_options;
pub mod launch_result;
mod live_locator;
pub(crate) mod locator;
mod name_evidence;
mod name_slot_status;
mod node;
mod node_descriptor;
mod node_identity;
mod node_match_context;
mod node_presentation;
mod notifications;
mod operation_report;
mod operation_report_build;
mod operation_report_dispatch;
mod operation_target;
pub mod permission_report;
pub mod permission_state;
mod point;
mod pointer_timeout;
mod post_action;
mod process_id;
mod process_identity;
pub mod process_state;
mod rect;
pub mod ref_action;
mod ref_action_actionability;
mod ref_action_context;
mod ref_action_poll;
mod ref_action_poll_state;
mod ref_action_single;
mod ref_action_wait;
mod ref_action_wait_context;
mod ref_action_wait_evidence;
mod ref_action_wait_support;
pub mod ref_alloc;
mod ref_alloc_config;
mod ref_alloc_options;
mod ref_alloc_scope;
mod ref_alloc_source;
mod ref_allocation;
mod ref_capabilities;
mod ref_entry;
mod ref_entry_identity;
mod ref_geometry;
pub mod ref_identity;
mod ref_identity_match;
mod ref_process;
mod ref_resolve_deadline;
mod ref_scope;
mod ref_source;
mod ref_token;
pub mod refs;
mod refs_lock;
pub mod refs_store;
#[cfg(test)]
mod refs_test_support;
mod refs_validate;
mod renderer_accessibility;
mod renderer_kind;
mod resolve_attempt_outcome;
pub mod resolve_errors;
pub mod response;
mod role;
pub mod role_text;
pub mod roles;
pub mod screenshot_target;
pub(crate) mod search_text;
pub mod session;
pub mod snapshot;
pub mod snapshot_ref;
pub mod snapshot_surface;
pub mod state;
mod state_predicate;
pub mod step_mechanism;
mod storage;
mod surface_info;
pub(crate) mod trace;
mod trace_artifact_budget;
pub(crate) mod trace_artifacts;
mod trace_file;
pub mod trace_read;
pub mod trace_sanitize;
mod trace_state;
pub mod tree_options;
mod value_matches;
mod wait;
pub mod window_filter;
mod window_focus;
mod window_info;
mod window_lookup;
mod window_op;
mod window_readback;
mod window_state;

pub use accname::{compute_description, compute_name, resolve_description, resolve_name};
pub use action::Action;
pub use action_request::ActionRequest;
pub use action_result::ActionResult;
pub use action_step::ActionStep;
pub use action_step_outcome::ActionStepOutcome;
pub use adapter::PlatformAdapter;
pub use adapter::actions::ActionOps;
pub use adapter::adapter_session::AdapterSession;
pub use adapter::hit_test::HitTestResult;
pub use adapter::input::InputOps;
pub use adapter::live_element::LiveElement;
pub use adapter::live_identity::LiveIdentity;
pub use adapter::native_handle::NativeHandle;
pub use adapter::observation::ObservationOps;
pub use adapter::system::SystemOps;
pub use app_info::{AppInfo, AppPresentation, app_name_matches};
pub use app_inventory::AppInventory;
pub use app_inventory_query::AppInventoryQuery;
pub use bounds_space::BoundsSpace;
pub use containment_predicate::ContainmentPredicate;
pub use context::{CommandContext, WaitSelector};
pub use cursor_overlay::{
    CURSOR_ARRIVAL_TIMEOUT_MS, CURSOR_HIGHLIGHT_HOLD_MS, CURSOR_IDLE_REST_MS,
    CURSOR_LABEL_REVEAL_MS, CURSOR_MOTION_BUDGET_MS, CURSOR_OVERLAY_GREETING, CURSOR_REST_FADE_MS,
    CursorImage, CursorImages, CursorMotion, CursorMotionProfile, CursorOverlayConfig,
    CursorOverlayControl, CursorOverlayInstruction, CursorOverlayStyle, CursorPhase, CursorPose,
    MAX_CURSOR_IMAGE_BYTES, MAX_CURSOR_IMAGE_PATH_BYTES, MAX_CURSOR_IMAGE_PIXELS,
    MAX_CURSOR_LABEL_BYTES, MAX_CURSOR_LABEL_WORDS, place_label,
};
pub use delivery_semantics::DeliverySemantics;
pub use display_info::DisplayInfo;
pub use element_identifier::ElementIdentifier;
pub use element_state::ElementState;
pub use headed_requirement::HeadedRequirement;
pub use identifier_kind::IdentifierKind;
pub use identity_predicate::IdentityPredicate;
pub use image_buffer::{ImageBuffer, MAX_PNG_INPUT_BYTES, parse_png_dimensions};
pub use image_format::ImageFormat;
pub use input::clipboard_content::ClipboardContent;
pub use input::clipboard_format::ClipboardFormat;
pub use input::direction::Direction;
pub use input::drag_params::{
    DragParams, MAX_DRAG_DROP_DELAY_MS, MAX_DRAG_DURATION_MS, MAX_DRAG_STEPS,
};
pub use input::key_combo::KeyCombo;
pub use input::modifier::Modifier;
pub use input::mouse_button::MouseButton;
pub use input::mouse_click_count::{MAX_MOUSE_CLICK_COUNT, validate_mouse_click_count};
pub use input::mouse_event::MouseEvent;
pub use input::mouse_event_kind::MouseEventKind;
pub use input::pointer_observation::PointerObservation;
pub use interaction_policy::InteractionPolicy;
pub use live_locator::{
    EvidenceRequirements, IdentifierEvidence, LocatorCardinality, LocatorEvidence, LocatorField,
    LocatorIdentifierStats, LocatorLimitStats, LocatorMaterialization, LocatorReadCounts,
    LocatorReadHealth, LocatorReadStats, LocatorRefEvidence, LocatorResolution,
    LocatorResolveRequest, LocatorSelection, LocatorSemanticReadStats, LocatorStats,
    LocatorTraversalStats, ObservationBudget, ObservationMode, ObservationRequest, ObservationRoot,
    ObservationSource, ObservedSubtree, ObservedTree, classify_query_result, evaluate_locator_tree,
    require_unique, resolve_query,
};
pub use locator::{
    LocatorQuery, accessibility_node_matches, node_context, node_matches, role_matches,
};
pub use name_evidence::NameEvidence;
pub use name_slot_status::{NameSlotStatus, SlotStatus};
pub use node::AccessibilityNode;
pub use node_descriptor::NodeDescriptor;
pub use node_identity::NodeIdentity;
pub use node_match_context::NodeMatchContext;
pub use node_presentation::NodePresentation;
pub use notifications::dismiss_all_notifications_request::DismissAllNotificationsRequest;
pub use notifications::dismiss_notification_request::DismissNotificationRequest;
pub use notifications::notification_action_request::NotificationActionRequest;
pub use notifications::notification_filter::NotificationFilter;
pub use notifications::notification_identity::NotificationIdentity;
pub use notifications::notification_identity_check::verify_notification_identity;
pub use notifications::notification_info::NotificationInfo;
pub use operation_report::OperationReport;
pub use operation_target::OperationTarget;
pub use permission_report::PermissionReport;
pub use permission_state::PermissionState;
pub use point::Point;
pub use post_action::execute_verified_action;
pub use process_id::ProcessId;
pub use process_identity::ProcessIdentity;
pub use rect::Rect;
pub use ref_capabilities::RefCapabilities;
pub use ref_entry::RefEntry;
pub use ref_entry_identity::RefEntryIdentity;
pub use ref_geometry::RefGeometry;
pub use ref_identity_match::IdentityMatch;
pub use ref_process::RefProcess;
pub use ref_scope::RefScope;
pub use ref_source::RefSource;
pub use refs::RefMap;
pub use refs_store::RefStore;
pub use renderer_kind::RendererKind;
pub use response::adapter_error::AdapterError;
pub use response::app_error::AppError;
pub use response::delivery_disposition::DeliveryDisposition;
pub use response::error_code::ErrorCode;
pub use response::output::{ErrorPayload, Response};
pub use response::recovery_hint::RecoveryHint;
pub use response::retry_disposition::RetryDisposition;
pub use role::Role;
pub use screenshot_target::ScreenshotTarget;
pub use session::interaction_lease::InteractionLease;
#[cfg(unix)]
pub use session::interaction_lease::{
    INTERACTION_LEASE_FD_ENV, acquire_unix_interaction_lease,
    adopt_inherited_unix_interaction_lease,
};
pub use session::process_lease_guard::ProcessLeaseGuard;
pub use session::session_affinity::SessionAffinity;
pub use snapshot_surface::SnapshotSurface;
pub use state_predicate::StatePredicate;
pub use step_mechanism::StepMechanism;
pub use storage::file_lock::FileLock;
pub use storage::private_file_ops::{
    PrivateFileOps, bounded_read, install_private_file_ops, temporary_file_name,
};
pub use storage::state_root::validate_state_root_env;
pub use surface_info::SurfaceInfo;
pub use trace_sanitize::sanitize_trace_value;
pub use tree_options::TreeOptions;
pub use value_matches::value_matches;
pub use wait::deadline::{DEFAULT_OPERATION_TIMEOUT_MS, Deadline};
pub use wait::event_kind::EventKind;
pub use wait::signal_baseline::SignalBaseline;
pub use wait::signal_completeness::SignalCompleteness;
pub use wait::signal_filter::SignalFilter;
pub use wait::signals::diff_signals;
pub use wait::surface_signal::SurfaceSignal;
pub use wait::ui_event::UiEvent;
pub use wait::wait_budget::{MAX_WAIT_TIMEOUT_MS, wait_timeout_duration};
pub use wait::wait_wakeup::WaitWakeup;
pub use window_filter::WindowFilter;
pub use window_info::WindowInfo;
pub use window_op::WindowOp;
pub use window_readback::WindowReadback;
pub use window_state::WindowState;
