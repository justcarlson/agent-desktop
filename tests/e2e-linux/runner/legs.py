from __future__ import annotations

from dataclasses import dataclass, replace
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
    "ref_actions.focus_headless_refusal",
    "ref_actions.double_click_headless_refusal",
    "ref_actions.triple_click_headless_refusal",
    "ref_actions.hover_headless_refusal",
    "ref_actions.drag_headless_refusal",
    "ref_actions.scroll_headless_refusal",
    "gaps.mutter_display_capture_refused",
    "gaps.mutter_qt_scroll_refused_no_container",
    "gaps.mutter_window_operation_refusals",
    "capture.wayland_route_gated",
    "headed_actions.never_focused_window_refusal",
    "menus.hidden_bar_timeout_message",
    "modal.calculator_about_blocks_parent",
    "modal.files_properties_blocks_parent",
    "modal.terminal_preferences_blocks_parent",
    "occlusion.above_cover_refused_no_effect",
    "select.refusal_without_semantic_route",
    "text_entry.unfocused_window_refusal",
    "toolkit_qt.scroll_refused_no_container",
    "tray.disabled_entry_refused",
    "tray.item_without_activate_refused",
    "wayland_actions.focus_window_wayland_session_refusal",
    "wayland_focus.headed_ref_action_refusal",
})

OUTSTANDING_REASONS = {
    **{f"gaps.event_{token}_{scope}_{kind}": "requires menu/popover surface discovery on the 3.14 branch"
       for token in ("surface-appeared", "surface-dismissed") for scope in ("scoped", "unscoped") for kind in ("success", "quiet")},
    'surfaces.batch_seed_opener_about': "requires a CLI popover/menu opener on the 3.14 branch",
    'surfaces.baseline_after_opener_fails': "requires a CLI popover/menu opener on the 3.14 branch",

    'modal.calculator_about_blocks_parent': "requires popover/menu discovery on the 3.14 branch",
    'modal.terminal_preferences_blocks_parent': "requires popover/menu discovery on the 3.14 branch",
}

HEADED_ACTION_PREFIXES = ("state.", "values.", "disclosure.", "select.", "scroll.")
HEADED_ACTION_LEGS = frozenset({"zero_bounds.history_listbox_outcome"})

OWNER_REASONS = {
    "gaps.event_window-closed_scoped_success": "after correcting the batch field, tracker-extract exited during the sole diagnostic rerun; remove the environment-dependent live close fixture and retain the fake frame/transport contract",
    "gaps.event_window-closed_unscoped_success": "the sole diagnostic rerun reached the Close ref but its bounds stayed unavailable and the product honestly timed out without delivery; retire this version-dependent multiwindow Close target",
    "gaps.event_window-opened_unscoped_quiet": "tracker-extract exited during the quiet wait and changed private process membership; retire this environment-dependent cell and retain the deterministic quiet-baseline contract test",
    "toolkit_electron.refs_and_completeness": "private VS Code never registered a new AT-SPI window within the 35-second observed-window deadline across three launches; launcher exit 0 or still running is not a usable frame; retain deterministic frame-generation checks",
    "toolkit_electron.menu_action_and_waits": "private VS Code has no independently attributed frame in three launch attempts, so no menu/action target exists; do not count launcher success as menu coverage",
    "gaps.gesture_gtk3_first_mouse-click": "Terminal with --show-menubar exposes no raw role-35 Edit target on this installation; retire the version-dependent menu fixture instead of substituting a guessed node",
}

OWNER_GAPS = {
    "delayed_enable": ("R9 controlled delayed enable and exactly-once dispatch", "Calculator Undo",
                       "L14-1 presents only next-event enable; the removed Undo fixture coalesces actions, so its display cannot distinguish one dispatch from two; a deterministic delayed target needs an owner-approved extension to the toolkit probe"),
    "replace_all_gating": ("R9 Replace All enables and disables with search text", "Text Editor",
                           "the retired search fixture matched the first field and longest toast text; those are not independent search/replacement state oracles; an unambiguous search-control target remains an owner decision"),
    "named_zero_size_action": ("R10 named ref-able zero-size action", "Files, Settings, Terminal, Calculator",
                               "L14-4 found only unnamed zero-axis containers and hidden sentinel geometry; none is a named ref-able zero-size action target"),
    "calculator_history_outcome": ("R10 Calculator history listbox CLI outcome", "Calculator",
                                   "the installed Calculator version has no stable history-listbox role; raw container checks cannot substitute for a CLI action on a missing control"),
    "same_role_name_ambiguous_pair": ("R11 same-role/name pair answers AMBIGUOUS_TARGET without an effect", "Qt probe",
                                     "L14-5 stock duplicates have different canonical roles; the permitted Qt probe has one button, and adding twins requires an explicit target extension; deterministic resolver ambiguity tests remain the owner-boundary proof"),
    "display_value_select_success": ("R15 display-value select success on a supporting toolkit", "Settings, Qt probe",
                                     "L14-10 libadwaita combo exposes no semantic choices; the existing Qt probe has no combo; the executed Settings refusal cannot count as selection success"),
    "wayland_display_capture_consent": ("R22 Wayland display capture with portal consent", "private GNOME portal",
                                        "L13-21 and L29-29 require a person to answer the portal; this headless tier has no consenting seat, and the lane may not touch a live session; window-route refusal is a separate measured contract"),
    "wayland_clipboard_html": ("R22 Wayland HTML content roundtrip", "foreign wl-clipboard client",
                               "the CLI exposes text/image setters and the retired HTML oracle allowed an empty product result; no verified Wayland HTML content route is measured by the text/image legs"),
    "wayland_clipboard_with_seat": ("R22 Wayland successful content/lifetime/clear with a keyboard seat", "foreign wl-clipboard client",
                                    "all four private headless clipboard attempts returned no_keyboard_seat; the foreign offer reader is also unavailable, so neither successful content nor refusal/no-effect is proven; require a private session with an observable keyboard seat"),
    "mutter_minimized_lift": ("R33 minimized lift and refused-activation outcome set", "Calculator",
                             "L31-53/L31-54 minimize/restore throttling invalidated the old timer fixture; a minimized lift requires an independently ready compositor transition, not a retry or an already-active focus target"),
}

OWNER_ACTION_GAPS = frozenset({"delayed_enable", "replace_all_gating", "named_zero_size_action",
                               "same_role_name_ambiguous_pair", "display_value_select_success"})

MUTTER_OWNER_REASONS = {
    "gaps.mutter_maximize_restore": "Calculator did not satisfy the old maximized-state oracle; the sole replacement run on resizable Text Editor then lost tracker-extract during cleanup; retire the live timing/environment cell and retain the deterministic comma-delimited, both-axis state oracle",
    **{f"gaps.event_{token}_{scope}_{kind}": "the raw accessibility owner/name or frame read timed out before any product command; retire the private-bus/environment-dependent Mutter cell and retain the portable event contracts"
       for token in ("app-launched", "window-closed", "window-opened") for scope in ("scoped", "unscoped") for kind in ("success", "quiet")
       if not (token == "window-opened" and scope == "unscoped" and kind == "quiet")},
    **{"gaps.mutter_qt_" + name: "Mutter created a mutter-x11-frames accessibility owner alongside the Qt owner, so launch attribution failed before any product command; remove this environment-dependent launch cell and retain the log/menu contracts"
       for name in ("headed_click_logged_once", "scroll_refused_no_container", "wait_menu_opens", "wait_menu_closes", "menu_already_open")},
    "gaps.gesture_qt_first_mouse-click": "raw accessibility bus timeout prevented attribution of the Qt target; no product input was observed, so this Mutter fixture is retired",
    "gaps.gesture_gtk4_second_mouse-click": "tracker-extract exited during the second-position gesture and changed private process membership; remove the environment-dependent cell without counting its partial effect as a pass",
}


def _owner_gap(name, row, *, modes=HEADLESS, tiers=X11):
    requirement, target, reason = row
    return Leg("gaps.owner_" + name, requirement, target, tiers, modes,
               ("atspi",), "owner-decision", reason)


def _row(leg_id, requirement, target, tiers, modes, oracles):
    disposition = "pass"
    reason = ""
    if leg_id.startswith(("shell_surfaces.", "notifications.", "tray.")):
        disposition = "outstanding"
        reason = "requires shell, menu, tray or notification surfaces on the 3.14 branch"
        OUTSTANDING_REASONS[leg_id] = reason
    elif leg_id in OUTSTANDING_REASONS:
        disposition = "outstanding"
        reason = OUTSTANDING_REASONS[leg_id]
    elif leg_id in OWNER_REASONS:
        disposition = "owner-decision"
        reason = OWNER_REASONS[leg_id]
    elif leg_id in ASSERTED_REFUSALS:
        disposition = "asserted-refusal"
        reason = "the leg asserts the product's refusal as its expected outcome"
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
    _row("ref_actions.expand", "A ref expands the Calculator menu and reveals its children, then restores", "Calculator", OPENBOX_ONLY, BOTH_MODES, "atspi,x11"),
    _row("ref_actions.collapse", "A ref collapses the Calculator menu and hides its children, then restores", "Calculator", OPENBOX_ONLY, BOTH_MODES, "atspi,x11"),
    _row("ref_actions.scroll", "Ref scroll changes visible Settings rows, then restores", "Settings", OPENBOX_ONLY, HEADED, "atspi,x11"),
    _row("ref_actions.scroll_headless_refusal", "Headless ref scroll is refused without changing visible rows", "Settings", OPENBOX_ONLY, HEADLESS, "atspi,x11"),
    _row("ref_actions.scroll_to", "Ref scroll-to reveals an offscreen control and restores the viewport", "Settings", OPENBOX_ONLY, BOTH_MODES, "atspi,x11"),
    _row("ref_actions.focus", "Ref focus changes the raw FOCUSED state and restores it", "Qt window", OPENBOX_ONLY, HEADED, "atspi,x11"),
    _row("ref_actions.focus_headless_refusal", "Headless ref focus is refused without changing focus", "Qt window", OPENBOX_ONLY, HEADLESS, "atspi,x11"),
    _row("ref_actions.double_click", "Ref double-click selects a document word and restores selection", "Text Editor", OPENBOX_ONLY, HEADED, "atspi,x11"),
    _row("ref_actions.double_click_headless_refusal", "Headless double-click is refused without selecting text", "Text Editor", OPENBOX_ONLY, HEADLESS, "atspi,x11"),
    _row("ref_actions.triple_click", "Ref triple-click selects a document line and restores selection", "Text Editor", OPENBOX_ONLY, HEADED, "atspi,x11"),
    _row("ref_actions.triple_click_headless_refusal", "Headless triple-click is refused without selecting text", "Text Editor", OPENBOX_ONLY, HEADLESS, "atspi,x11"),
    _row("ref_actions.right_click", "Ref right-click opens the raw context menu and restores it", "Text Editor", OPENBOX_ONLY, BOTH_MODES, "atspi,x11"),
    _row("ref_actions.hover", "Ref hover exposes native status text and restores it", "Qt window", OPENBOX_ONLY, HEADED, "atspi,x11"),
    _row("ref_actions.hover_headless_refusal", "Headless ref hover is refused without exposing status text", "Qt window", OPENBOX_ONLY, HEADLESS, "atspi,x11"),
    _row("ref_actions.drag", "Ref drag selects document text and restores selection", "Text Editor", OPENBOX_ONLY, HEADED, "atspi,x11"),
    _row("ref_actions.drag_headless_refusal", "Headless ref drag is refused without selecting text", "Text Editor", OPENBOX_ONLY, HEADLESS, "atspi,x11"),
    _row("capture.window_pixels_match_xwd", "A window capture matches the pixels xwd reads", "Text Editor", X11, HEADLESS, "pixels,x11"),
    *(_owner_gap(name, row, modes=BOTH_MODES if name in OWNER_ACTION_GAPS else HEADLESS,
                tiers=MUTTER_ONLY if name == "mutter_minimized_lift" else WAYLAND_ONLY if name.startswith("wayland_") else X11) for name, row in OWNER_GAPS.items()),
    *(_owner_gap(f"gesture_{toolkit}_{position}_{verb}", (f"R32 {toolkit} {position} {verb}", toolkit,
                "the retired generic gesture oracle used numeric names/tab counters and omitted restoration; this cell has no independently measured app effect; requires a deterministic toolkit-specific target before it can pass"),
                modes=HEADED)
      for toolkit in ("gtk3", "gtk4", "qt") for position in (("first", "second") if toolkit == "gtk4" else ("first",))
      for verb in ("click", "double-click", "triple-click", "right-click", "hover", "drag", "mouse-move", "mouse-wheel", "scroll")),
    *(_row(f"gaps.event_{token}_{scope}_{kind}", f"R19 {token} {scope} {kind}", "Calculator About / Terminal Preferences", X11, HEADLESS, "atspi")
      for token in ("surface-appeared", "surface-dismissed") for scope in ("scoped", "unscoped") for kind in ("success", "quiet")),
    *(_row(f"gaps.gesture_{toolkit}_{position}_{verb}", f"R32 {toolkit} {position} {verb} with pointer and raw geometry/effect reads", toolkit, X11, HEADED, "x11,atspi,applog")
      for toolkit in ("gtk3", "gtk4", "qt") for position in (("first", "second") if toolkit == "gtk4" else ("first",)) for verb in ("mouse-click",)),
    *(_row(f"gaps.event_{token}_{scope}_{kind}", f"R19 {token} {scope} {kind} with independent restoration", "Calculator and Text Editor", X11, HEADLESS, "atspi,x11,proc")
      for token in ("window-opened", "window-closed", "app-launched", "app-terminated", "focus-changed")
      for scope in ("scoped", "unscoped") for kind in ("success", "quiet")),
    *(Leg(f"gaps.timeout_{name}", f"R9 disabled action uses {name} timeout without an effect",
          "Calculator Undo", X11, BOTH_MODES, ("atspi",), "owner-decision" if name == "default" else "asserted-refusal",
          "private tracker-extract exited during the default-timeout leg and changed its process baseline; remove the environment-dependent live check and retain the deterministic timeout-band/no-effect test" if name == "default" else
          "permanently disabled Undo must refuse without changing text/state; zero timeout is one immediate ACTION_FAILED check")
      for name in ("default", "explicit", "short", "zero")),
    _row("gaps.value_write_preserves_active_window", "R24 a background value write preserves the active window and pointer", "Settings behind Text Editor", X11, HEADLESS, "gsettings,x11"),
    _row("gaps.display_capture", "R22 display pixels match XWD", "Calculator", OPENBOX_ONLY, HEADLESS, "pixels,x11"),
    _row("gaps.mutter_display_capture_refused", "R22 display capture asserts the compositing refusal", "Calculator", MUTTER_ONLY, HEADLESS, "pixels,x11"),
    Leg("gaps.wayland_window_capture_refused", "R22 Wayland window capture fails closed without changing the app", "Calculator", WAYLAND_ONLY, HEADLESS, ("atspi",), "asserted-refusal", "Wayland has no window-position capture route"),
    *(Leg("gaps.wayland_clipboard_" + name, "R22 Wayland clipboard refusal/content observation: " + name, "foreign wl-paste client", WAYLAND_ONLY, HEADLESS, ("clipboard", "proc"), "owner-decision", "a private process joined during PNG creation/refusal/cleanup; retire the environment-dependent image-file leg without relaxing process membership" if name == "image" else "the product reports no_keyboard_seat, but a foreign reader can be unavailable on that same seat; its failure cannot prove empty offers, so no live no-effect or content pass is accepted") for name in ("text", "image", "lifetime", "clear")),
    *(_row("gaps.mutter_" + name, "R33 Mutter " + name + " has an independently restored effect", "Text Editor" if name == "maximize_restore" else "Calculator", MUTTER_ONLY, HEADLESS, "x11,proc") for name in ("move", "resize", "minimize_restore", "maximize_restore")),
    _row("gaps.mutter_window_operation_refusals", "R33 missing-window move/resize/minimize report not_delivered and leave the real window unchanged", "Calculator", MUTTER_ONLY, HEADLESS, "x11"),
    *(_row("gaps.mutter_qt_" + name, "R35 Qt on Mutter " + name, "Qt window", MUTTER_ONLY, HEADED if name.startswith("headed_") else HEADLESS, "applog") for name in ("headless_click_logged_once", "headed_click_logged_once", "scroll_refused_no_container", "wait_menu_opens", "wait_menu_closes", "menu_already_open")),
    _row("toolkit_electron.refs_and_completeness", "R35 VS Code first snapshot has refs while its frame remains registered", "VS Code", X11, HEADLESS, "atspi,proc"),
    _row("toolkit_electron.menu_action_and_waits", "R35 VS Code File action opens its dropdown and menu predicates succeed and time out both ways", "VS Code", X11, BOTH_MODES, "atspi,proc"),
    _row("modal.terminal_preferences_blocks_parent", "Preferences blocks a ref action on its parent", "Files, Terminal, Calculator", X11, HEADED, "atspi,x11"),
    _row("modal.calculator_about_blocks_parent", "About blocks a ref action on its parent", "Files, Terminal, Calculator", X11, HEADED, "atspi,x11"),
    _row("observation.qualified_refs_survive_snapshots", "A qualified ref still resolves after unrelated snapshots", "Calculator", X11, HEADLESS, "atspi"),
    _row("observation.stale_ref_after_app_killed", "A ref held across an app kill answers STALE_REF or ELEMENT_NOT_FOUND", "Calculator", X11, HEADLESS, "atspi"),
    _row("observation.selector_wait_resolves", "A selector wait returns once the element appears", "Calculator", X11, HEADLESS, "atspi"),
    _row("observation.enabled_state_canonical_and_raw", "is enabled reads canonical enabled while the raw SENSITIVE bit holds", "Calculator", OPENBOX_ONLY, HEADLESS, "atspi"),
    _row("observation.session_namespace_isolation", "A snapshot taken in one session is not visible from another", "Calculator", OPENBOX_ONLY, HEADLESS, "atspi"),
    _row("observation.trace_segment_written", "A traced command writes one trace segment under the selected session", "Calculator", OPENBOX_ONLY, HEADLESS, "proc"),
    _row("state.terminal_copy_follows_selection", "Copy enables and disables with a text selection", "Terminal", OPENBOX_ONLY, HEADLESS, "atspi"),
    _row("state.text_editor_semantic_type", "The semantic type path writes the typed text into the document", "Text Editor", OPENBOX_ONLY, HEADLESS, "atspi"),
    _row("values.hot_corner_flip_restore", "Hot Corner flips, the key confirms it, and it restores", "Settings", OPENBOX_ONLY, HEADLESS, "gsettings"),
    _row("values.pointer_speed_commit", "Pointer Speed takes a value the key confirms after panel reopen", "Settings", OPENBOX_ONLY, HEADLESS, "gsettings"),
    _row("values.workspace_count_commit", "Number of Workspaces takes a value the key confirms after panel reopen", "Settings", OPENBOX_ONLY, HEADLESS, "gsettings"),
    _row("values.terminal_read_only_flip", "The Read-Only check item flips and restores", "Terminal", X11, HEADLESS, "atspi"),
    _row("select.screen_blank_value", "Screen Blank takes a new value the idle-delay key confirms", "Settings", X11, HEADLESS, "gsettings"),
    _row("select.refusal_without_semantic_route", "A combo with no semantic route is refused with no effect", "Settings", X11, HEADLESS, "gsettings"),
    _row("placeholder.settings_apps_search", "Applications search reports its placeholder and no description", "Settings", X11, HEADLESS, "atspi"),
    _row("placeholder.files_search_none", "Files search reports no placeholder", "Files", X11, HEADLESS, "atspi"),
    _row("zero_bounds.files_container_raw_only", "A zero-axis Files container gets no ref and is covered as raw geometry", "Files", X11, HEADLESS, "atspi"),
    _row("zero_bounds.settings_container_raw_only", "A zero-axis Settings container gets no ref and is covered as raw geometry", "Settings", X11, HEADLESS, "atspi"),
    _row("zero_bounds.hidden_menu_items_not_visible", "Hidden menu bar items read as not visible", "Terminal", X11, HEADLESS, "atspi"),
    _row("duplicates.two_processes_distinct_window_ids", "Two Calculator processes have distinct window ids that each snapshot", "Calculator", X11, HEADLESS, "atspi"),
    _row("duplicates.main_menu_nodes_separate", "Main Menu resolves its menubutton and switch as separate nodes", "Calculator", X11, HEADLESS, "atspi"),
    _row("toolkit_qt.refs_and_completeness", "The first snapshot of the Qt window is complete with refs", "Qt window", X11, HEADLESS, "applog"),
    _row("toolkit_qt.headless_click_logged_once", "A headless click logs exactly one activation", "Qt window", OPENBOX_ONLY, HEADLESS, "applog"),
    _row("toolkit_qt.scroll_refused_no_container", "Scroll is refused with no_scroll_container", "Qt window", OPENBOX_ONLY, HEADLESS, "applog"),
    _row("toolkit_qt.wait_menu_opens", "wait --menu succeeds after the menu opens", "Qt window", OPENBOX_ONLY, HEADLESS, "applog"),
    _row("toolkit_qt.wait_menu_closes", "wait --menu-closed succeeds after the menu closes", "Qt window", OPENBOX_ONLY, HEADLESS, "applog"),
    _row("toolkit_qt.menu_already_open", "A menu predicate reads the already-open state at once", "Qt window", OPENBOX_ONLY, HEADLESS, "applog"),
    _row("toolkit_qt.headed_click_logged_once", "A headed click logs exactly one activation", "Qt window", OPENBOX_ONLY, HEADED, "applog"),
    _row("headed_actions.focus_reads_active_window", "The product's focus step makes the target the active window", "Calculator", X11, HEADED, "atspi,x11"),
    _row("headed_actions.type_text", "Headed typing reaches the document", "Text Editor", X11, HEADED, "atspi,x11"),
    _row("pointer.drag_delivery_report", "The drag delivery report agrees with the oracle delivered and undelivered", "Text Editor", OPENBOX_ONLY, HEADED, "x11,atspi"),
    _row("pointer.mouse_wheel_delivery_report", "The wheel delivery report agrees with the oracle delivered and undelivered", "Text Editor", OPENBOX_ONLY, HEADED, "x11,atspi"),
    _row("pointer.mouse_click_delivery_report", "The mouse-click delivery report agrees with the oracle delivered and undelivered", "Calculator", OPENBOX_ONLY, HEADED, "x11,atspi"),
    _row("occlusion.ordinary_cover_removed_click_lands", "An ordinary cover is removed by focus preparation and the click lands", "Calculator over Text Editor", X11, HEADED, "x11,atspi"),
    _row("occlusion.above_cover_refused_no_effect", "An above-layer cover is refused with no effect in either window", "Calculator over Text Editor", OPENBOX_ONLY, HEADED, "x11,atspi"),
    _row("occlusion.above_flag_dropped_click_lands", "Without the above flag the same click lands in the editor", "Calculator over Text Editor", OPENBOX_ONLY, HEADED, "x11,atspi"),
    _row("text_entry.typed_text_reaches_document", "Typed text reaches the document", "Text Editor", OPENBOX_ONLY, HEADED, "atspi,x11"),
    _row("text_entry.held_key_released", "A held key is not left pressed", "Text Editor", X11, HEADED, "atspi,x11"),
    _row("non_interference.pointer_unchanged_by_semantic_click", "A semantic click leaves the pointer where it was", "Calculator", X11, HEADLESS, "x11,atspi"),
    _row("non_interference.active_window_unchanged_by_semantic_click", "A semantic click leaves the active window as it was", "Calculator", X11, HEADLESS, "x11,atspi"),
    _row("non_interference.pointer_unchanged_by_value_write", "A semantic value write leaves the pointer where it was", "Calculator", X11, HEADLESS, "x11,atspi,gsettings"),
    _row("lifecycle.close_app_undelivered_report", "close-app reports not delivered when the process stays", "Calculator", X11, HEADLESS, "x11,proc"),
    _row("lifecycle.focus_window_active_reads", "focus-window makes the target active by the X11 reads", "Calculator", X11, HEADLESS, "x11,proc"),
    _row("lifecycle.resize_window", "resize-window changes the geometry the oracle reads", "Calculator", OPENBOX_ONLY, HEADLESS, "x11,proc"),
    _row("lifecycle.move_window", "move-window changes the position the oracle reads", "Calculator", OPENBOX_ONLY, HEADLESS, "x11,proc"),
    _row("lifecycle.minimize_restore", "minimize and restore change the window state the oracle reads", "Calculator", OPENBOX_ONLY, HEADLESS, "x11,proc"),
    _row("lifecycle.maximize_restore", "maximize and restore change the window state the oracle reads", "Calculator", OPENBOX_ONLY, HEADLESS, "x11,proc"),
    _row("lifecycle.window_operation_delivery_reports", "move, resize and minimize reports agree with the oracle both ways", "Calculator", OPENBOX_ONLY, HEADLESS, "x11,proc"),
    _row("lifecycle.openbox_lift_restored_and_active", "The lift case ends restored_and_active", "Calculator", OPENBOX_ONLY, HEADLESS, "x11"),
    _row("clipboard.text_roundtrip", "Text round trips against a foreign producer and reader", "foreign client", X11, HEADLESS, "clipboard"),
    _row("clipboard.html_roundtrip", "HTML round trips against a foreign producer and reader", "foreign client", X11, HEADLESS, "clipboard"),
    _row("clipboard.image_roundtrip", "An image round trips against a foreign producer and reader", "foreign client", X11, HEADLESS, "clipboard"),
    _row("clipboard.owner_outlives_setter", "The selection owner outlives the setter process", "foreign client", X11, HEADLESS, "clipboard"),
    _row("clipboard.clear_leaves_empty", "clipboard-clear leaves the clipboard empty", "foreign client", X11, HEADLESS, "clipboard"),
    _row("clipboard.foreign_read_after_cli_exit", "A foreign read of a product-set selection works after the CLI exited", "foreign client", X11, HEADLESS, "clipboard"),
    _row("events.window_opened_quiet", "wait --event window-opened times out with no transition", "Calculator, Text Editor, Files, Terminal", OPENBOX_ONLY, HEADLESS, "atspi,proc"),
    _row("events.window_closed_transition", "wait --event window-closed succeeds on a real transition", "Calculator, Text Editor, Files, Terminal", OPENBOX_ONLY, HEADLESS, "atspi,proc"),
    _row("events.window_closed_quiet", "wait --event window-closed times out with no transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.app_launched_transition", "wait --event app-launched succeeds on a real transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.app_launched_quiet", "wait --event app-launched times out with no transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.app_terminated_transition", "wait --event app-terminated succeeds on a real transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.app_terminated_quiet", "wait --event app-terminated times out with no transition", "Calculator, Text Editor, Files, Terminal", X11, HEADLESS, "atspi,proc"),
    _row("events.focus_changed_quiet", "wait --event focus-changed times out when focus stays", "Text Editor", X11, HEADLESS, "atspi,x11"),
    _row("events.scoped_and_unscoped_forms", "Each event wait works scoped to a process and unscoped", "Text Editor", X11, HEADLESS, "atspi,x11"),
    _row("events.open_and_close_inside_one_wait", "A window that opens and closes inside one wait is reported", "Text Editor", X11, HEADLESS, "atspi,x11"),
    _row("events.target_killed_mid_wait", "A target killed mid-wait is reported as such", "Text Editor", X11, HEADLESS, "atspi,x11,proc"),
    _row("menus.wait_menu_opens_calculator", "wait --menu succeeds on the Calculator main menu", "Calculator, Terminal", X11, HEADLESS, "atspi"),
    _row("menus.wait_menu_closes_calculator", "wait --menu-closed succeeds on the Calculator main menu", "Calculator, Terminal", X11, HEADLESS, "atspi"),
    _row("menus.wait_menu_already_open", "A menu predicate reads the already-open state at once", "Calculator, Terminal", X11, HEADLESS, "atspi"),
    _row("menus.hidden_bar_timeout_message", "A hidden menu bar times out with the landed message", "Calculator, Terminal", X11, HEADLESS, "atspi"),
    _row("identity.restart_old_id_window_not_found", "After a restart the old window id answers WINDOW_NOT_FOUND", "Calculator", X11, HEADLESS, "atspi,proc"),
    _row("identity.restart_new_id_resolves", "After a restart the new window id resolves", "Calculator", X11, HEADLESS, "atspi,proc"),
    _row("identity.id_survives_unrelated_launch", "A window id survives the launch of an unrelated app", "Calculator", X11, HEADLESS, "atspi,proc"),
    _row("surfaces.batch_seed_opener_about", "One seeded batch opens About and surface-appeared is satisfied with no title", "Calculator", X11, HEADLESS, "atspi"),
    _row("surfaces.baseline_after_opener_fails", "A baseline seeded after the opener fails the same batch", "Calculator", X11, HEADLESS, "atspi"),
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
    _row("tray.headed_click_opens_shell_popup", "A headed click on an item without Activate opens the shell popup", "harness tray item", MUTTER_ONLY, HEADED, "applog,pixels,atspi"),
    _row("wayland_actions.focus_window_wayland_session_refusal", "focus-window refuses wayland_session before anything is sent", "Calculator", WAYLAND_ONLY, HEADLESS, "atspi,x11"),
    _row("wayland_actions.list_windows_no_position", "list-windows reports no position", "Calculator", WAYLAND_ONLY, HEADLESS, "atspi,x11"),
    _row("wayland_actions.unhealthy_shell_named_reason", "A shell that never becomes healthy fails the tier with its named reason", "Calculator", WAYLAND_ONLY, HEADLESS, "proc"),
)

def _retained_cells(registry):
    for leg in registry:
        reason = MUTTER_OWNER_REASONS.get(leg.id)
        if not reason or "x11-mutter" not in leg.tiers:
            yield leg
            continue
        retained = tuple(tier for tier in leg.tiers if tier != "x11-mutter")
        if retained:
            yield replace(leg, tiers=retained)
        yield replace(leg, id=leg.id + ".mutter_owner" if retained else leg.id,
                      tiers=MUTTER_ONLY, disposition="owner-decision", reason=reason)


REGISTRY = tuple(_retained_cells(REGISTRY))
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
