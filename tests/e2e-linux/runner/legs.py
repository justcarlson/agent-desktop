from __future__ import annotations

from dataclasses import dataclass
from typing import Callable

TIER_NAMES = ("x11-openbox", "x11-mutter", "wayland")
MODE_NAMES = ("headless", "headed")
ORACLE_NAMES = ("atspi", "gsettings", "x11", "proc", "pixels", "clipboard", "dbus", "applog")
DISPOSITIONS = ("pass", "asserted-refusal", "owner-decision", "outstanding")

X11 = ("x11-openbox", "x11-mutter")
OPENBOX_ONLY = ("x11-openbox",)
MUTTER_ONLY = ("x11-mutter",)
WAYLAND_ONLY = ("wayland",)
ALL_TIERS = TIER_NAMES
HEADLESS = ("headless",)
HEADED = ("headed",)
BOTH_MODES = MODE_NAMES


@dataclass(frozen=True)
class Leg:
    id: str
    requirement: str
    target: str
    tiers: tuple[str, ...]
    modes: tuple[str, ...]
    oracles: tuple[str, ...]
    disposition: str
    reason: str = ""

    def __post_init__(self) -> None:
        if not self.requirement.strip():
            raise ValueError(f"leg {self.id} has no requirement")
        if not set(self.tiers) <= set(TIER_NAMES) or not self.tiers:
            raise ValueError(f"leg {self.id} names an unknown tier")
        if not set(self.modes) <= set(MODE_NAMES) or not self.modes:
            raise ValueError(f"leg {self.id} names an unknown mode")
        if not set(self.oracles) <= set(ORACLE_NAMES) or not self.oracles:
            raise ValueError(f"leg {self.id} names an unknown oracle")
        if self.disposition not in DISPOSITIONS:
            raise ValueError(f"leg {self.id} has an unknown disposition")
        if self.disposition != "pass" and not self.reason:
            raise ValueError(f"leg {self.id} needs a reason for its disposition")


ASSERTED_REFUSALS = frozenset({
    "capture.wayland_route_gated",
    "headed_actions.never_focused_window_refusal",
    "menus.hidden_bar_timeout_message",
    "modal.calculator_about_blocks_parent",
    "modal.files_properties_blocks_parent",
    "modal.terminal_preferences_blocks_parent",
    "occlusion.above_cover_refused_no_effect",
    "select.refusal_without_semantic_route",
    "state.undo_disabled_default_timeout",
    "state.undo_disabled_explicit_timeout",
    "state.undo_enable_delayed_short_timeout_fails",
    "state.undo_zero_timeout_single_check",
    "text_entry.unfocused_window_refusal",
    "toolkit_qt.scroll_refused_no_container",
    "tray.disabled_entry_refused",
    "tray.item_without_activate_refused",
    "wayland_actions.focus_window_wayland_session_refusal",
    "wayland_focus.headed_ref_action_refusal",
})

OUTSTANDING_REASONS = {
    "observation.unreachable_bus_permission_error":
        "the unavailable-accessibility-bus permission mapping is absent on this base",
}

HEADED_ACTION_PREFIXES = ("state.", "values.", "disclosure.", "select.", "scroll.")
HEADED_ACTION_LEGS = frozenset({"zero_bounds.history_listbox_outcome"})


def _row(leg_id, requirement, target, tiers, modes, oracles):
    disposition = "pass"
    reason = ""
    if leg_id in ASSERTED_REFUSALS:
        disposition = "asserted-refusal"
        reason = "the leg asserts the product's refusal as its expected outcome"
    elif leg_id in OUTSTANDING_REASONS:
        disposition = "outstanding"
        reason = OUTSTANDING_REASONS[leg_id]
    if leg_id.startswith(HEADED_ACTION_PREFIXES) or leg_id in HEADED_ACTION_LEGS:
        modes = BOTH_MODES
    return Leg(
        id=leg_id,
        requirement=requirement,
        target=target,
        tiers=tiers,
        modes=modes,
        oracles=tuple(oracles.split(",")),
        disposition=disposition,
        reason=reason,
    )


REGISTRY: tuple[Leg, ...] = (
    _row("observation.qualified_refs_survive_snapshots", "A qualified ref still resolves after unrelated snapshots", "Calculator", X11, HEADLESS, "atspi"),
    _row("observation.stale_ref_after_app_killed", "A ref held across an app kill answers STALE_REF or ELEMENT_NOT_FOUND", "Calculator", X11, HEADLESS, "atspi"),
    _row("observation.selector_wait_resolves", "A selector wait returns once the element appears", "Calculator", X11, HEADLESS, "atspi"),
    _row("observation.enabled_state_canonical_and_raw", "is enabled reads canonical enabled while the raw SENSITIVE bit holds", "Calculator", X11, HEADLESS, "atspi"),
    _row("observation.drilldown_root_replaces_subtree", "A drill-down from a ref replaces only that subtree's refs", "Files", X11, HEADLESS, "atspi"),
    _row("observation.session_namespace_isolation", "A snapshot taken in one session is not visible from another", "Calculator", X11, HEADLESS, "proc"),
    _row("observation.trace_segment_written", "A traced command writes one trace segment under the selected session", "Calculator", X11, HEADLESS, "proc"),
    _row("observation.unreachable_bus_permission_error", "An unreachable accessibility bus maps to the permission error", "none", X11, HEADLESS, "dbus"),
    _row("state.undo_enable_delayed_default_timeout", "A wait on a delayed Undo succeeds under the default timeout", "Calculator", X11, HEADLESS, "atspi"),
    _row("state.undo_enable_delayed_explicit_timeout", "A wait on a delayed Undo succeeds under an explicit timeout", "Calculator", X11, HEADLESS, "atspi"),
    _row("state.undo_enable_delayed_short_timeout_fails", "The same wait fails under a timeout shorter than the delay", "Calculator", X11, HEADLESS, "atspi"),
    _row("state.undo_dispatches_exactly_once", "One Undo click dispatches once, the display reads 7", "Calculator", X11, HEADLESS, "atspi"),
    _row("state.undo_disabled_explicit_timeout", "A permanently disabled Undo times out near an explicit 2 s timeout", "Calculator", X11, HEADLESS, "atspi"),
    _row("state.undo_disabled_default_timeout", "A permanently disabled Undo times out near the untouched default", "Calculator", X11, HEADLESS, "atspi"),
    _row("state.undo_zero_timeout_single_check", "A zero timeout makes one immediate check with no late effect", "Calculator", X11, HEADLESS, "atspi"),
    _row("state.terminal_copy_follows_selection", "Copy enables and disables with a text selection", "Terminal", X11, HEADLESS, "atspi"),
    _row("state.text_editor_semantic_type", "The semantic type path writes the typed text into the document", "Text Editor", X11, HEADLESS, "atspi"),
    _row("values.hot_corner_flip_restore", "Hot Corner flips, the key confirms it, and it restores", "Settings", X11, HEADLESS, "gsettings"),
    _row("values.pointer_speed_commit", "Pointer Speed takes a value the key confirms after panel reopen", "Settings", X11, HEADLESS, "gsettings"),
    _row("values.workspace_count_commit", "Number of Workspaces takes a value the key confirms after panel reopen", "Settings", X11, HEADLESS, "gsettings"),
    _row("values.terminal_read_only_flip", "The Read-Only check item flips and restores", "Terminal", X11, HEADLESS, "atspi"),
    _row("disclosure.files_tree_expand_collapse", "A list-view folder row expands and collapses with its children read both ways", "Files", X11, HEADLESS, "atspi"),
    _row("select.screen_blank_value", "Screen Blank takes a new value the idle-delay key confirms", "Settings", X11, HEADLESS, "gsettings"),
    _row("select.refusal_without_semantic_route", "A combo with no semantic route is refused with no effect", "Settings", X11, HEADLESS, "gsettings"),
    _row("placeholder.settings_apps_search", "Applications search reports its placeholder and no description", "Settings", X11, HEADLESS, "atspi"),
    _row("placeholder.files_search_none", "Files search reports no placeholder", "Files", X11, HEADLESS, "atspi"),
    _row("scroll.files_300_last_item", "A 300-file folder reaches its last item through the scroll route and the view restores", "Files", X11, HEADLESS, "atspi"),
    _row("zero_bounds.files_container_raw_only", "A zero-axis Files container gets no ref and is covered as raw geometry", "Files", X11, HEADLESS, "atspi"),
    _row("zero_bounds.settings_container_raw_only", "A zero-axis Settings container gets no ref and is covered as raw geometry", "Settings", X11, HEADLESS, "atspi"),
    _row("zero_bounds.history_listbox_outcome", "The history listbox reads as the resolver outcome defines", "Calculator", X11, HEADLESS, "atspi"),
    _row("zero_bounds.hidden_menu_items_not_visible", "Hidden menu bar items read as not visible", "Terminal", X11, HEADLESS, "atspi"),
    _row("duplicates.two_processes_distinct_window_ids", "Two Calculator processes have distinct window ids that each snapshot", "Calculator", X11, HEADLESS, "atspi"),
    _row("duplicates.main_menu_nodes_separate", "Main Menu resolves its menubutton and switch as separate nodes", "Calculator", X11, HEADLESS, "atspi"),
    _row("duplicates.new_window_distinct_id", "A Ctrl+N window has its own id that snapshots and focuses exactly", "Text Editor", X11, HEADLESS, "atspi"),
    _row("toolkit_qt.refs_and_completeness", "The first snapshot of the Qt window is complete with refs", "Qt window", X11, HEADLESS, "applog"),
    _row("toolkit_qt.headless_click_logged_once", "A headless click logs exactly one activation", "Qt window", X11, HEADLESS, "applog"),
    _row("toolkit_qt.scroll_refused_no_container", "Scroll is refused with no_scroll_container", "Qt window", X11, HEADLESS, "applog"),
    _row("toolkit_qt.wait_menu_opens", "wait --menu succeeds after the menu opens", "Qt window", X11, HEADLESS, "applog"),
    _row("toolkit_qt.wait_menu_closes", "wait --menu-closed succeeds after the menu closes", "Qt window", X11, HEADLESS, "applog"),
    _row("toolkit_qt.menu_already_open", "A menu predicate reads the already-open state at once", "Qt window", X11, HEADLESS, "applog"),
    _row("toolkit_qt.headed_click_logged_once", "A headed click logs exactly one activation", "Qt window", X11, HEADED, "applog"),
    _row("toolkit_electron.frame_stays_registered", "The frame stays registered for the whole leg", "VS Code", X11, HEADLESS, "atspi"),
    _row("toolkit_electron.refs_and_completeness", "The first snapshot is complete with refs", "VS Code", X11, HEADLESS, "atspi"),
    _row("toolkit_electron.menu_bar_by_keyboard", "The keyboard opens the application menu bar and it reads back", "VS Code", X11, HEADED, "atspi"),
    _row("toolkit_electron.wait_menu_both_directions", "wait --menu and --menu-closed each succeed on VS Code", "VS Code", X11, HEADED, "atspi"),
    _row("headed_actions.click_button", "A headed click lands and the display changes", "Calculator", X11, HEADED, "atspi,x11"),
    _row("headed_actions.toggle_switch", "A headed toggle flips the switch and restores", "Calculator", X11, HEADED, "atspi,x11"),
    _row("headed_actions.delayed_enable", "A headed wait on a delayed Undo succeeds with one dispatch", "Calculator", X11, HEADED, "atspi,x11"),
    _row("headed_actions.focus_reads_active_window", "The product's focus step makes the target the active window", "Calculator", X11, HEADED, "atspi,x11"),
    _row("headed_actions.set_value_commit", "A headed value write commits to the backing key", "Settings", X11, HEADED, "atspi,gsettings,x11"),
    _row("headed_actions.select_combo", "A headed select takes a new value the key confirms", "Settings", X11, HEADED, "atspi,gsettings,x11"),
    _row("headed_actions.select_combo_modifier_keys", "A combo select with super and win keys lands", "Settings", X11, HEADED, "atspi,gsettings,x11"),
    _row("headed_actions.expand_collapse_row", "A headed expand and collapse read back both ways", "Files", X11, HEADED, "atspi,x11"),
    _row("headed_actions.type_text", "Headed typing reaches the document", "Text Editor", X11, HEADED, "atspi,x11"),
    _row("headed_actions.never_focused_window_refusal", "A never-focused window gives the landed refusal and no input arrives", "Text Editor", MUTTER_ONLY, HEADED, "x11"),
    _row("pointer.click", "click lands on a GTK3, GTK4 and Qt target: pointer position, application effect and extents plus origin agree", "GTK3, GTK4, Qt", X11, HEADED, "atspi,x11"),
    _row("pointer.double_click", "double-click lands on a GTK3, GTK4 and Qt target: pointer position, application effect and extents plus origin agree", "GTK3, GTK4, Qt", X11, HEADED, "atspi,x11"),
    _row("pointer.triple_click", "triple-click lands on a GTK3, GTK4 and Qt target: pointer position, application effect and extents plus origin agree", "GTK3, GTK4, Qt", X11, HEADED, "atspi,x11"),
    _row("pointer.right_click", "right-click lands on a GTK3, GTK4 and Qt target: pointer position, application effect and extents plus origin agree", "GTK3, GTK4, Qt", X11, HEADED, "atspi,x11"),
    _row("pointer.hover", "hover lands on a GTK3, GTK4 and Qt target: pointer position, application effect and extents plus origin agree", "GTK3, GTK4, Qt", X11, HEADED, "atspi,x11"),
    _row("pointer.drag", "drag lands on a GTK3, GTK4 and Qt target: pointer position, application effect and extents plus origin agree", "GTK3, GTK4, Qt", X11, HEADED, "atspi,x11"),
    _row("pointer.mouse_move", "mouse-move lands on a GTK3, GTK4 and Qt target: pointer position, application effect and extents plus origin agree", "GTK3, GTK4, Qt", X11, HEADED, "atspi,x11"),
    _row("pointer.mouse_click", "mouse-click lands on a GTK3, GTK4 and Qt target: pointer position, application effect and extents plus origin agree", "GTK3, GTK4, Qt", X11, HEADED, "atspi,x11"),
    _row("pointer.mouse_wheel", "mouse-wheel lands on a GTK3, GTK4 and Qt target: pointer position, application effect and extents plus origin agree", "GTK3, GTK4, Qt", X11, HEADED, "atspi,x11"),
    _row("pointer.scroll", "scroll lands on a GTK3, GTK4 and Qt target: pointer position, application effect and extents plus origin agree", "GTK3, GTK4, Qt", X11, HEADED, "atspi,x11"),
    _row("pointer.gtk3_extents_origin", "GTK3 screen extents match the pointer position", "Terminal", X11, HEADED, "atspi,x11"),
    _row("pointer.gtk4_two_positions", "GTK4 window extents plus the client origin hold at two window positions", "Calculator, Files, Settings", X11, HEADED, "atspi,x11"),
    _row("pointer.qt_extents_origin", "Qt screen extents match the pointer position", "Qt window", X11, HEADED, "applog,x11"),
    _row("pointer.drag_delivery_report", "The drag delivery report agrees with the oracle delivered and undelivered", "Calculator", X11, HEADED, "x11,atspi"),
    _row("pointer.mouse_wheel_delivery_report", "The wheel delivery report agrees with the oracle delivered and undelivered", "Calculator", X11, HEADED, "x11,atspi"),
    _row("pointer.mouse_click_delivery_report", "The mouse-click delivery report agrees with the oracle delivered and undelivered", "Calculator", X11, HEADED, "x11,atspi"),
    _row("occlusion.ordinary_cover_removed_click_lands", "An ordinary cover is removed by focus preparation and the click lands", "Calculator over Text Editor", X11, HEADED, "x11,atspi"),
    _row("occlusion.above_cover_refused_no_effect", "An above-layer cover is refused with no effect in either window", "Calculator over Text Editor", X11, HEADED, "x11,atspi"),
    _row("occlusion.above_flag_dropped_click_lands", "Without the above flag the same click lands in the editor", "Calculator over Text Editor", X11, HEADED, "x11,atspi"),
    _row("modal.files_properties_blocks_parent", "Properties blocks a ref action on its parent", "Files, Terminal, Calculator", X11, HEADED, "atspi,x11"),
    _row("modal.terminal_preferences_blocks_parent", "Preferences blocks a ref action on its parent", "Files, Terminal, Calculator", X11, HEADED, "atspi,x11"),
    _row("modal.calculator_about_blocks_parent", "About blocks a ref action on its parent", "Files, Terminal, Calculator", X11, HEADED, "atspi,x11"),
    _row("modal.second_app_unaffected", "A ref action in a second app is unaffected by the modal", "Files, Terminal, Calculator", X11, HEADED, "atspi,x11"),
    _row("text_entry.typed_text_reaches_document", "Typed text reaches the document", "Text Editor", X11, HEADED, "atspi,x11"),
    _row("text_entry.find_bar_ctrl_f", "Ctrl+F opens the find bar", "Text Editor", X11, HEADED, "atspi,x11"),
    _row("text_entry.replace_bar_ctrl_h", "Ctrl+H opens the replace bar", "Text Editor", X11, HEADED, "atspi,x11"),
    _row("text_entry.replace_all_gating", "Replace All enables on typed search text and disables when it is removed", "Text Editor", X11, HEADED, "atspi,x11"),
    _row("text_entry.held_key_released", "A held key is not left pressed", "Text Editor", X11, HEADED, "atspi,x11"),
    _row("text_entry.unfocused_window_refusal", "A window that never became active gives the landed refusal", "Text Editor", X11, HEADED, "atspi,x11"),
    _row("non_interference.pointer_unchanged_by_semantic_click", "A semantic click leaves the pointer where it was", "Calculator", X11, HEADLESS, "x11,atspi"),
    _row("non_interference.active_window_unchanged_by_semantic_click", "A semantic click leaves the active window as it was", "Calculator", X11, HEADLESS, "x11,atspi"),
    _row("non_interference.pointer_unchanged_by_value_write", "A semantic value write leaves the pointer where it was", "Calculator", X11, HEADLESS, "x11,atspi"),
    _row("lifecycle.close_app_delivery_report", "close-app reports delivered when the process is gone", "Calculator", X11, HEADLESS, "x11,proc"),
    _row("lifecycle.close_app_undelivered_report", "close-app reports not delivered when the process stays", "Calculator", X11, HEADLESS, "x11,proc"),
    _row("lifecycle.focus_window_active_reads", "focus-window makes the target active by the X11 reads", "Calculator", X11, HEADLESS, "x11,proc"),
    _row("lifecycle.resize_window", "resize-window changes the geometry the oracle reads", "Calculator", X11, HEADLESS, "x11,proc"),
    _row("lifecycle.move_window", "move-window changes the position the oracle reads", "Calculator", X11, HEADLESS, "x11,proc"),
    _row("lifecycle.minimize_restore", "minimize and restore change the window state the oracle reads", "Calculator", X11, HEADLESS, "x11,proc"),
    _row("lifecycle.maximize_restore", "maximize and restore change the window state the oracle reads", "Calculator", X11, HEADLESS, "x11,proc"),
    _row("lifecycle.window_operation_delivery_reports", "move, resize and minimize reports agree with the oracle both ways", "Calculator", X11, HEADLESS, "x11,proc"),
    _row("lifecycle.mutter_lift_outcome_set", "The lift case ends in a measured valid outcome", "Calculator, Text Editor", MUTTER_ONLY, HEADLESS, "x11"),
    _row("lifecycle.mutter_desktop_switch_read_back", "A desktop switch is read back before focus-window is asserted", "Calculator, Text Editor", MUTTER_ONLY, HEADLESS, "x11"),
    _row("lifecycle.mutter_activation_refusal_codes", "An activation refusal is activation_refused or activation_not_granted", "Calculator, Text Editor", MUTTER_ONLY, HEADLESS, "x11"),
    _row("lifecycle.openbox_lift_restored_and_active", "The lift case ends restored_and_active", "Calculator", OPENBOX_ONLY, HEADLESS, "x11"),
    _row("capture.window_pixels_match_xwd", "A window capture matches the pixels xwd reads", "Text Editor", X11, HEADLESS, "pixels,x11"),
    _row("capture.stable_at_idle", "Two captures of an idle window compare equal", "Text Editor", X11, HEADLESS, "pixels,x11"),
    _row("capture.content_change_moves_region", "A text change moves the compared region", "Text Editor", X11, HEADLESS, "pixels,x11"),
    _row("capture.covered_window_capture", "A covered window captures its own content", "Text Editor", X11, HEADLESS, "pixels,x11"),
    _row("capture.minimised_window_capture", "A minimised window captures or refuses as the landed route defines", "Text Editor", X11, HEADLESS, "pixels,x11"),
    _row("capture.display_capture", "A display capture decodes with content", "Text Editor", X11, HEADLESS, "pixels,x11"),
    _row("capture.wayland_route_gated", "The gated Wayland capture route answers its landed refusal", "Text Editor", WAYLAND_ONLY, HEADLESS, "pixels"),
    _row("clipboard.text_roundtrip", "Text round trips against a foreign producer and reader", "foreign client", ALL_TIERS, HEADLESS, "clipboard"),
    _row("clipboard.html_roundtrip", "HTML round trips against a foreign producer and reader", "foreign client", ALL_TIERS, HEADLESS, "clipboard"),
    _row("clipboard.image_roundtrip", "An image round trips against a foreign producer and reader", "foreign client", ALL_TIERS, HEADLESS, "clipboard"),
    _row("clipboard.owner_outlives_setter", "The selection owner outlives the setter process", "foreign client", ALL_TIERS, HEADLESS, "clipboard"),
    _row("clipboard.clear_leaves_empty", "clipboard-clear leaves the clipboard empty", "foreign client", ALL_TIERS, HEADLESS, "clipboard"),
    _row("clipboard.foreign_read_after_cli_exit", "A foreign read of a product-set selection works after the CLI exited", "foreign client", ALL_TIERS, HEADLESS, "clipboard"),
    _row("events.window_opened_transition", "wait --event window-opened succeeds on a real transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.window_opened_quiet", "wait --event window-opened times out with no transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.window_closed_transition", "wait --event window-closed succeeds on a real transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.window_closed_quiet", "wait --event window-closed times out with no transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.app_launched_transition", "wait --event app-launched succeeds on a real transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.app_launched_quiet", "wait --event app-launched times out with no transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.app_terminated_transition", "wait --event app-terminated succeeds on a real transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.app_terminated_quiet", "wait --event app-terminated times out with no transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.surface_appeared_transition", "wait --event surface-appeared succeeds on a real transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.surface_appeared_quiet", "wait --event surface-appeared times out with no transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.surface_dismissed_transition", "wait --event surface-dismissed succeeds on a real transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.surface_dismissed_quiet", "wait --event surface-dismissed times out with no transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.focus_changed_transition", "wait --event focus-changed succeeds when focus moves between two windows", "Text Editor", X11, HEADLESS, "atspi,x11"),
    _row("events.focus_changed_quiet", "wait --event focus-changed times out when focus stays", "Text Editor", X11, HEADLESS, "atspi,x11"),
    _row("events.scoped_and_unscoped_forms", "Each event wait works scoped to a process and unscoped", "Text Editor", X11, HEADLESS, "atspi,x11"),
    _row("events.open_and_close_inside_one_wait", "A window that opens and closes inside one wait is reported", "Text Editor", X11, HEADLESS, "atspi,x11"),
    _row("events.target_killed_mid_wait", "A target killed mid-wait is reported as such", "Text Editor", X11, HEADLESS, "atspi,x11"),
    _row("menus.wait_menu_opens_calculator", "wait --menu succeeds on the Calculator main menu", "Calculator, Terminal", X11, HEADLESS, "atspi"),
    _row("menus.wait_menu_closes_calculator", "wait --menu-closed succeeds on the Calculator main menu", "Calculator, Terminal", X11, HEADLESS, "atspi"),
    _row("menus.wait_menu_opens_terminal", "wait --menu succeeds on a Terminal menu", "Calculator, Terminal", X11, HEADLESS, "atspi"),
    _row("menus.wait_menu_already_open", "A menu predicate reads the already-open state at once", "Calculator, Terminal", X11, HEADLESS, "atspi"),
    _row("menus.hidden_bar_timeout_message", "A hidden menu bar times out with the landed message", "Calculator, Terminal", X11, HEADLESS, "atspi"),
    _row("menus.terminal_preferences_modal", "Terminal Preferences opens as a modal", "Calculator, Terminal", X11, HEADLESS, "atspi"),
    _row("menus.settings_panel_navigation", "gnome-control-center lands on the named panel content", "Calculator, Terminal", X11, HEADLESS, "atspi"),
    _row("identity.restart_old_id_window_not_found", "After a restart the old window id answers WINDOW_NOT_FOUND", "Calculator", X11, HEADLESS, "atspi,proc"),
    _row("identity.restart_new_id_resolves", "After a restart the new window id resolves", "Calculator", X11, HEADLESS, "atspi,proc"),
    _row("identity.id_survives_unrelated_launch", "A window id survives the launch of an unrelated app", "Calculator", X11, HEADLESS, "atspi,proc"),
    _row("surfaces.batch_seed_opener_about", "One seeded batch opens About and surface-appeared is satisfied with no title", "Calculator", X11, HEADLESS, "atspi"),
    _row("surfaces.baseline_after_opener_fails", "A baseline seeded after the opener fails the same batch", "Calculator", X11, HEADLESS, "atspi"),
    _row("surfaces.window_and_sheet_dedupe_one_id", "A dialog listed as window and sheet dedupes to one id", "Calculator", X11, HEADLESS, "atspi"),
    _row("surfaces.files_properties_surface_scope", "Properties is discovered by identity and scoped to its app", "Files", X11, HEADLESS, "atspi"),
    _row("launch.launch_calculator_attributed", "launch starts Calculator and the registry diff attributes it", "Calculator, Text Editor, Terminal", X11, HEADLESS, "proc,atspi"),
    _row("launch.launch_text_editor_new_window", "A second launch gives Text Editor a new window", "Calculator, Text Editor, Terminal", X11, HEADLESS, "proc,atspi"),
    _row("launch.launch_terminal_owner", "Terminal's owner is the terminal server, not the spawned pid", "Calculator, Text Editor, Terminal", X11, HEADLESS, "proc,atspi"),
    _row("launch.list_apps_alias", "list-apps answers like its alias", "Calculator, Text Editor, Terminal", X11, HEADLESS, "proc,atspi"),
    _row("launch.launch_suggestion", "A failed launch carries the landed suggestion", "Calculator, Text Editor, Terminal", X11, HEADLESS, "proc,atspi"),
    _row("launch.recovery_strategy_on_error", "An error envelope carries recovery.strategy only when retry is safe", "Calculator, Text Editor, Terminal", X11, HEADLESS, "proc,atspi"),
    _row("shell_surfaces.snapshot_system_tray", "snapshot --surface system-tray lists the tray", "GNOME shell", MUTTER_ONLY, HEADLESS, "atspi,pixels"),
    _row("shell_surfaces.snapshot_action_center", "snapshot --surface action-center lists the message list", "GNOME shell", MUTTER_ONLY, HEADLESS, "atspi,pixels"),
    _row("shell_surfaces.snapshot_quick_settings", "snapshot --surface quick-settings lists the panel", "GNOME shell", MUTTER_ONLY, HEADLESS, "atspi,pixels"),
    _row("shell_surfaces.dock_supported_surfaces_agree", "supported surfaces agree with open-system-surface", "GNOME shell", MUTTER_ONLY, HEADLESS, "atspi,pixels"),
    _row("shell_surfaces.system_button_open_close_by_ref", "The System button opens and closes by ref, read back by pixels and the raw tree", "GNOME shell", MUTTER_ONLY, HEADED, "atspi,pixels"),
    _row("shell_surfaces.message_list_closed_after_leg", "The message list is closed after each leg", "GNOME shell", MUTTER_ONLY, HEADED, "atspi,pixels"),
    _row("shell_surfaces.menu_surface_counts_gtk4", "The menu surface item count equals the raw SHOWING count on GTK4", "Calculator, Terminal, Qt window", MUTTER_ONLY, HEADLESS, "atspi"),
    _row("shell_surfaces.menu_surface_counts_gtk3", "The menu surface item count equals the raw SHOWING count on GTK3", "Calculator, Terminal, Qt window", MUTTER_ONLY, HEADLESS, "atspi"),
    _row("shell_surfaces.menu_surface_counts_qt", "The menu surface item count equals the raw SHOWING count on Qt", "Calculator, Terminal, Qt window", MUTTER_ONLY, HEADLESS, "atspi"),
    _row("notifications.list_equals_sender_record", "list-notifications equals the independent sender's record in order", "GNOME shell", MUTTER_ONLY, HEADLESS, "dbus"),
    _row("notifications.app_filter_case_insensitive", "The app filter is a case-insensitive contains", "GNOME shell", MUTTER_ONLY, HEADLESS, "dbus"),
    _row("notifications.wait_notification_passes", "wait --notification passes near the send", "GNOME shell", MUTTER_ONLY, HEADLESS, "dbus"),
    _row("notifications.wait_notification_times_out", "wait --notification times out with none sent", "GNOME shell", MUTTER_ONLY, HEADLESS, "dbus"),
    _row("notifications.headless_mutations_policy_denied", "Headless mutations answer POLICY_DENIED and the list is unchanged", "GNOME shell", MUTTER_ONLY, HEADLESS, "dbus"),
    _row("notifications.wrong_app_not_found", "A wrong expected app answers NOTIFICATION_NOT_FOUND with no signal", "GNOME shell", MUTTER_ONLY, HEADLESS, "dbus"),
    _row("notifications.empty_list_not_found", "An empty list answers NOTIFICATION_NOT_FOUND with no signal", "GNOME shell", MUTTER_ONLY, HEADLESS, "dbus"),
    _row("notifications.dismiss_one_reason_closed", "A headed dismiss closes only the target with reason 2", "GNOME shell", MUTTER_ONLY, HEADED, "dbus"),
    _row("notifications.dismiss_keeps_bystander", "A headed dismiss leaves a bystander notification", "GNOME shell", MUTTER_ONLY, HEADED, "dbus"),
    _row("notifications.dismiss_all_closes_every_notification", "dismiss-all closes every notification", "GNOME shell", MUTTER_ONLY, HEADED, "dbus"),
    _row("notifications.action_invoked_signal", "notification-action shows ActionInvoked on the monitor", "GNOME shell", MUTTER_ONLY, HEADED, "dbus"),
    _row("notifications.delivered_mutation_internal_fails", "A delivered mutation answering INTERNAL fails the leg", "GNOME shell", MUTTER_ONLY, HEADED, "dbus"),
    _row("tray.sender_registered_before_first_leg", "The harness item is registered with the watcher before the first leg", "harness tray item", MUTTER_ONLY, HEADLESS, "applog,dbus"),
    _row("tray.headless_activate_logged", "Headless click delivers Activate as the sender's log shows", "harness tray item", MUTTER_ONLY, HEADLESS, "applog,dbus"),
    _row("tray.headless_menu_event_clicked", "Headless menu click delivers Event(id, clicked) as the log shows", "harness tray item", MUTTER_ONLY, HEADLESS, "applog,dbus"),
    _row("tray.disabled_entry_refused", "A disabled entry is refused with no call logged", "harness tray item", MUTTER_ONLY, HEADLESS, "applog,dbus"),
    _row("tray.item_without_activate_refused", "An item without Activate is refused with no call logged", "harness tray item", MUTTER_ONLY, HEADLESS, "applog,dbus"),
    _row("tray.killed_owner_stale_ref", "A killed owner turns the old ref into STALE_REF", "harness tray item", MUTTER_ONLY, HEADLESS, "applog,dbus"),
    _row("tray.headed_click_opens_shell_popup", "A headed click on an item without Activate opens the shell popup", "harness tray item", MUTTER_ONLY, HEADED, "applog,pixels"),
    _row("wayland_actions.semantic_action_no_input_device", "A semantic action works with no input device, display 7", "Calculator", WAYLAND_ONLY, HEADLESS, "atspi"),
    _row("wayland_actions.seat_without_keyboard_reported_as_seat", "A seat with no keyboard is reported as the seat, not the shell", "Calculator", WAYLAND_ONLY, HEADLESS, "atspi"),
    _row("wayland_actions.hit_test_unknown", "hit_test answers Unknown", "Calculator", WAYLAND_ONLY, HEADLESS, "atspi"),
    _row("wayland_actions.focus_window_wayland_session_refusal", "focus-window refuses wayland_session before anything is sent", "Calculator", WAYLAND_ONLY, HEADLESS, "atspi"),
    _row("wayland_actions.list_windows_no_position", "list-windows reports no position", "Calculator", WAYLAND_ONLY, HEADLESS, "atspi"),
    _row("wayland_actions.unhealthy_shell_named_reason", "A shell that never becomes healthy fails the tier with its named reason", "Calculator", WAYLAND_ONLY, HEADLESS, "atspi"),
    _row("wayland_input.first_key_lands", "The product's first key lands by read-back", "Text Editor", WAYLAND_ONLY, HEADLESS, "atspi,clipboard"),
    _row("wayland_input.modifiers_work", "Modifier keys reach the focused window", "Text Editor", WAYLAND_ONLY, HEADLESS, "atspi,clipboard"),
    _row("wayland_input.ctrl_f_two_keycodes", "Ctrl+F reaches the find bar through two keycodes", "Text Editor", WAYLAND_ONLY, HEADLESS, "atspi,clipboard"),
    _row("wayland_input.clipboard_paste_into_app", "A clipboard paste inserts text into the focused window", "Text Editor", WAYLAND_ONLY, HEADLESS, "atspi,clipboard"),
    _row("wayland_input.key_before_throwaway_not_rescued", "A key sent before the throwaway period is not rescued by the harness", "Text Editor", WAYLAND_ONLY, HEADLESS, "atspi,clipboard"),
    _row("wayland_focus.seat_keyboard_makes_active", "A seat keyboard makes the focused window carry raw ACTIVE", "Text Editor", WAYLAND_ONLY, HEADLESS, "atspi"),
    _row("wayland_focus.later_mapped_window_takes_focus", "A later-mapped window takes focus", "Text Editor", WAYLAND_ONLY, HEADLESS, "atspi"),
    _row("wayland_focus.focus_changed_times_out_with_baseline", "focus-changed times out carrying baseline_counts", "Text Editor", WAYLAND_ONLY, HEADLESS, "atspi"),
    _row("wayland_focus.headed_ref_action_refusal", "A headed ref action returns the landed refusal by code and message", "Calculator", WAYLAND_ONLY, HEADED, "atspi"),
)

_BY_ID = {leg.id: leg for leg in REGISTRY}
if len(_BY_ID) != len(REGISTRY):
    raise ValueError("duplicate leg id in the registry")

_IMPLEMENTATIONS: dict[str, Callable] = {}


def leg_by_id(leg_id: str) -> Leg:
    return _BY_ID[leg_id]


def implementation(leg_id: str):
    if leg_id not in _BY_ID:
        raise KeyError(f"implementation for unregistered leg {leg_id}")

    def register(fn):
        if leg_id in _IMPLEMENTATIONS:
            raise ValueError(f"leg {leg_id} has two implementations")
        _IMPLEMENTATIONS[leg_id] = fn
        return fn

    return register


def implementations() -> dict[str, Callable]:
    return dict(_IMPLEMENTATIONS)
