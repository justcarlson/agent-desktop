# Linux ignored live-test cost inventory

Baseline: `b66beeb5`. Retired and replaced cases stay in this original roster; see the live-gate runbook for their dispositions. Qualified names were reconciled against the original libtest list.

Static scan of `crates/linux/src`: **247 ignored tests in 64 files**. This is source inventory, not measured runtime. Exact function names and ignore reasons follow; test and helper sleeps are recorded as source lines in each module.

## Highest cost suspects

- `system/close_live_tests.rs` (14 cases) and helper `system/close_live/rig.rs`: repeated app launch/termination cycles, readiness waits, teardown; several 200 ms / 500 ms / 1 s sleeps and multi-second close grace/refusal paths.
- `system/focus_fallback_live_tests.rs` (5) and `focus_lift_live_tests.rs` (1): explicit sequences of 1–2.5 s sleeps for WM focus/overview/minimize behavior. These sleeps are prime event/readiness-poll candidates, preserving active-target/refusal evidence.
- `system/signal_live_tests.rs` (10), `signal_live_surfaces_tests.rs` (7), `signal_inventory_live_tests.rs` (1): event observation/baseline waits, with per-test fixture/helper processes. Keep event routing, stop/refusal, incomplete-vs-unresponsive assertions.
- `system/launch_live_tests.rs` (8), `launch_resident_live_tests.rs` (3): launches plus polling for process/window registration; repeated app startup dominates. Reuse an app only where test independence and distinct process/window attribution remain checked.
- `system/menu_wait_live_tests.rs` (9): interactive app/menu transitions; explicit 1 s and 1.5 s waits. Replace fixed delay with bounded condition polling where possible, retaining open/close direction, timeout-message, stale-target, and stopped-target behavior.
- `tree/physical_live_tests.rs` (15), `physical_live_apps_tests.rs` (5), `physical_live_mutter_tests.rs` (1): real input and read-back; multiple fixed 300–1200 ms delays. Optimize fixture readiness and observation, preserve wrong-element / delivery / unverified / refusal / cleanup coverage.
- `system/capture/wayland_portal_live_tests.rs` (10) plus `wayland_portal_live_support.rs`: each boots GNOME shell/portal; fixed 3s+1s readiness/setup and 1.5s dialog delays, some 3–4s response limits. Portal dialogs/consent require independent buses and runtime directories before parallel execution.
- `input/clipboard/wayland_live_tests.rs` (2) + `live_shell.rs`: shell boot has 3s and 2s sleeps; two tests each create isolated compositor. Candidate for one fixture/session around both tests only if request routing, lifetime/cleanup, and fresh owner identity remain separately verified.
- `tree/perf_live_tests.rs` (1): explicitly min-of-seven with warm-up discard; intentionally expensive measurement, not a suite regression. Retired after measuring the malformed synthetic-miss fixture; see the gate runbook for retained coverage.

## Complete ignored-test roster

### `crates/linux/src/actions/live_click_tests.rs` (5 tests)

- L158 `a_calculator_digit_click_is_delivered_and_unverified_on_its_own_node` — #[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
- L169 `a_click_on_an_insensitive_undo_is_not_delivered` — #[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
- L181 `right_click_without_a_context_menu_verb_is_not_supported` — #[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
- L190 `headless_focus_is_denied_before_any_write` — #[ignore = "live: needs the AT-SPI session and gnome-calculator"]
- L202 `headed_focus_on_a_gtk_button_without_a_focus_verb_is_not_supported` — #[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]

Helper source sleeps in this module:
- L83: `std::thread::sleep(Duration::from_secs(2));`
- L94: `std::thread::sleep(Duration::from_millis(250));`
Timeout/readiness constants:
- L15: `const FRAME_WAIT: Duration = Duration::from_secs(30);`

### `crates/linux/src/actions/live_disclosure_tests.rs` (4 tests)

- L9 `the_calculator_main_menu_expands_and_collapses_through_its_inner_toggle` — #[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
- L22 `collapsing_a_collapsed_menu_button_writes_nothing` — #[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
- L34 `a_digit_button_is_a_leaf_and_is_never_expanded` — #[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
- L43 `a_files_tree_row_expands_and_collapses_in_both_directions` — #[ignore = "live: needs the AT-SPI session, Files (nautilus) with a folder tree and the write engine"]

Helper source sleeps in this module:
- none

### `crates/linux/src/actions/live_select_tests.rs` (4 tests)

- L9 `a_settings_sidebar_row_that_ignores_the_selection_is_a_write_ignored_failure` — #[ignore = "live: needs the AT-SPI session, GNOME Settings and the write engine"]
- L26 `a_value_with_no_match_in_the_sidebar_is_not_found_and_writes_nothing` — #[ignore = "live: needs the AT-SPI session, GNOME Settings and the write engine"]
- L41 `a_button_with_no_selection_route_is_not_supported` — #[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]
- L50 `a_gtk4_drop_down_is_chosen_through_its_popup_and_the_shown_value_follows` — #[ignore = "live: needs the AT-SPI session, zenity and the write engine"]

Helper source sleeps in this module:
- none

### `crates/linux/src/actions/live_text_tests.rs` (4 tests)

- L10 `set_value_and_clear_land_on_a_text_field_in_every_toolkit` — #[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]
- L27 `type_inserts_non_ascii_text_into_an_empty_field_exactly` — #[ignore = "needs lane A's write engine and lane B's caret read; run through scripts/private-session.sh --wm"]
- L50 `type_after_a_wide_character_lands_at_the_caret_exactly` — #[ignore = "needs lane A's write engine and lane B's caret read; run through scripts/private-session.sh --wm"]
- L69 `a_password_field_write_lands_and_the_result_never_carries_the_text` — #[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- none

### `crates/linux/src/actions/live_toggle_tests.rs` (3 tests)

- L13 `settings_hot_corner_checks_and_unchecks_with_read_back` — #[ignore = "live: needs the AT-SPI session, GNOME Settings (Multitasking) and the write engine"]
- L27 `checking_a_checked_switch_writes_nothing` — #[ignore = "live: needs the AT-SPI session, GNOME Settings (Multitasking) and the write engine"]
- L42 `toggle_on_a_push_button_is_not_supported` — #[ignore = "live: needs the AT-SPI session, gnome-calculator and the write engine"]

Helper source sleeps in this module:
- none

### `crates/linux/src/actions/live_value_tests.rs` (3 tests)

- L8 `a_spin_button_takes_a_whole_number_and_refuses_a_fraction_in_every_toolkit` — #[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]
- L30 `a_slider_is_clamped_and_the_stored_value_is_not_reported_as_verified` — #[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]
- L48 `a_scroll_bar_write_moves_the_content` — #[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- none

### `crates/linux/src/actions/live_widget_tests.rs` (9 tests)

- L45 `a_click_runs_the_handler_of_a_button_in_every_toolkit` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L59 `toggle_check_and_uncheck_move_a_check_box_and_never_write_twice` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L103 `select_by_display_value_chooses_the_item_of_a_combo_box` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L135 `select_by_display_value_chooses_the_item_of_a_qt_list` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L151 `a_headed_set_focus_moves_the_focus_to_a_qt_button` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L170 `the_selection_reads_in_utf16_and_the_caret_in_code_points` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L221 `the_read_only_menu_item_of_gnome_terminal_toggles` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh --wm"]
- L297 `a_gecko_page_takes_a_click_and_a_check` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh --wm"]
- L325 `a_plain_qwidget_and_a_qmainwindow_are_both_listed_windows` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L93: `std::thread::sleep(Duration::from_millis(500));`
- L216: `std::thread::sleep(Duration::from_millis(300));`
- L362: `std::thread::sleep(Duration::from_millis(300));`

### `crates/linux/src/actions/live_write_tests.rs` (9 tests)

- L43 `write_live_a_named_action_toggles_a_check_box_and_an_independent_read_sees_it` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L67 `write_live_the_chain_skips_an_absent_verb_and_reports_what_a_re_read_saw` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L120 `write_live_set_text_contents_lands_and_an_independent_read_returns_it` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L148 `write_live_a_value_property_write_moves_a_slider` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L169 `write_live_a_read_only_value_never_reads_back_as_written` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L187 `write_live_an_index_the_node_does_not_have_changes_nothing` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L219 `write_live_a_destroyed_widget_is_stale_where_the_toolkit_reports_it` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L271 `write_live_a_stopped_app_is_a_timeout_that_may_have_landed` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L315 `write_live_a_killed_app_is_a_stale_ref_that_was_not_delivered` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- L178: `std::thread::sleep(Duration::from_millis(600));`
- L205: `std::thread::sleep(Duration::from_millis(600));`
Timeout/readiness constants:
- L24: `const SETTLE: Duration = Duration::from_secs(5);`

### `crates/linux/src/input/clipboard/owner_live_tests.rs` (3 tests)

- L29 `a_record_left_by_a_killed_owner_is_replaced_and_never_signalled` — #[ignore = "needs a private X11 session and the built clipboard owner"]
- L52 `the_interaction_lease_descriptor_is_not_inherited_by_the_owner` — #[ignore = "needs a private X11 session and the built clipboard owner"]
- L86 `a_state_directory_the_owner_cannot_use_leaves_the_earlier_clipboard_untouched` — #[ignore = "needs a private X11 session and the built clipboard owner"]

Helper source sleeps in this module:
- none

### `crates/linux/src/input/clipboard/set_clear_live_tests.rs` (8 tests)

- L19 `text_set_is_served_by_a_detached_owner_and_read_by_a_second_process` — #[ignore = "needs a private X11 session and the built clipboard owner"]
- L74 `the_cli_exits_with_its_stdout_closed_while_the_owner_keeps_serving` — #[ignore = "needs a private X11 session and the built CLI and clipboard owner"]
- L111 `clear_leaves_the_selection_unowned_and_ends_the_recorded_owner` — #[ignore = "needs a private X11 session and the built clipboard owner"]
- L125 `a_second_set_replaces_the_first_owner_and_leaves_one_record` — #[ignore = "needs a private X11 session and the built clipboard owner"]
- L143 `a_text_above_the_incr_threshold_round_trips_through_both_readers` — #[ignore = "needs a private X11 session and the built clipboard owner"]
- L162 `an_image_is_served_as_png_and_read_back` — #[ignore = "needs a private X11 session and the built clipboard owner"]
- L187 `file_paths_are_served_under_both_file_manager_targets` — #[ignore = "needs a private X11 session and the built clipboard owner"]
- L211 `the_owner_exits_when_another_client_takes_the_selection` — #[ignore = "needs a private X11 session and the built clipboard owner"]

Helper source sleeps in this module:
- none

### `crates/linux/src/input/clipboard/wayland_live_tests.rs` (2 tests)

- L265 `text_round_trips_with_foreign_readers_and_writers` — #[ignore = "boots a private headless gnome-shell; run inside scripts/private-session.sh"]
- L313 `images_and_file_lists_round_trip` — #[ignore = "boots a private headless gnome-shell; run inside scripts/private-session.sh"]

Helper source sleeps in this module:
- L72: `std::thread::sleep(Duration::from_secs(2));`
- L102: `std::thread::sleep(Duration::from_millis(200));`
- L107: `std::thread::sleep(Duration::from_secs(3));`
- L143: `std::thread::sleep(Duration::from_millis(300));`
- L146: `std::thread::sleep(Duration::from_millis(100));`
- L148: `std::thread::sleep(Duration::from_secs(1));`
- L289: `std::thread::sleep(Duration::from_secs(2));`
Timeout/readiness constants:
- L15: `const SOCKET_WAIT: Duration = Duration::from_secs(20);`
- L16: `const SEAT_WAIT: Duration = Duration::from_secs(8);`

### `crates/linux/src/input/wayland/live_tests.rs` (2 tests)

- L191 `remote_desktop_answers_on_the_session_bus_and_not_on_the_accessibility_bus` — #[ignore = "needs a private Wayland session with a headless gnome-shell; run through scripts/private-session.sh"]
- L212 `keys_land_in_a_wayland_native_gtk4_entry` — #[ignore = "needs a private Wayland session with a headless gnome-shell; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- L109: `std::thread::sleep(Duration::from_millis(150));`
- L167: `std::thread::sleep(Duration::from_millis(1500));`

### `crates/linux/src/input/x11/live_remap_tests.rs` (1 tests)

- L26 `live_a_remap_left_by_a_killed_client_is_restored_by_the_next_call` — #[ignore = "needs scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L20: `std::thread::sleep(Duration::from_millis(100));`
- L68: `std::thread::sleep(Duration::from_millis(500));`

### `crates/linux/src/input/x11/live_tests.rs` (4 tests)

- L250 `live_typing_reaches_a_gtk3_entry` — #[ignore = "needs scripts/private-session.sh --wm"]
- L256 `live_typing_reaches_a_gtk4_entry` — #[ignore = "needs scripts/private-session.sh --wm"]
- L262 `live_typing_reaches_a_qt_entry` — #[ignore = "needs scripts/private-session.sh --wm"]
- L276 `live_a_key_left_held_by_a_killed_client_is_released_by_the_next_call` — #[ignore = "needs scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L155: `std::thread::sleep(Duration::from_millis(200));`
- L172: `std::thread::sleep(Duration::from_millis(100));`
- L319: `std::thread::sleep(Duration::from_millis(100));`
- L327: `std::thread::sleep(Duration::from_millis(500));`
Timeout/readiness constants:
- L12: `const SETTLE: Duration = Duration::from_secs(30);`
- L14: `const READY_WAIT: Duration = Duration::from_millis(1_500);`

### `crates/linux/src/system/app_alias_live_tests.rs` (2 tests)

- L96 `alias_live_calculator_resolves_by_entry_name_and_stopped_is_unresponsive` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L102 `alias_live_files_resolves_by_entry_name_and_stopped_is_unresponsive` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L67: `std::thread::sleep(Duration::from_millis(200));`
Timeout/readiness constants:
- L11: `const SETTLE: Duration = Duration::from_secs(20);`

### `crates/linux/src/system/app_ops_live_tests.rs` (1 tests)

- L6 `live_desktop_ids_round_trip_through_launch_inventory_selection_and_close` — #[ignore = "launches desktop apps; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- none

### `crates/linux/src/system/capture/capture_live_shadow_tests.rs` (1 tests)

- L18 `grab_live_a_window_capture_leaves_out_the_client_side_shadow` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/system/capture/capture_live_tests.rs` (10 tests)

- L18 `grab_live_window_pixels_are_the_window_paint` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]
- L50 `grab_live_the_window_unit_follows_the_manager` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]
- L85 `grab_live_a_full_screen_grab_equals_the_known_paint` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]
- L113 `grab_live_a_full_screen_is_refused_under_compositing` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]
- L129 `grab_live_a_covered_window_returns_its_own_paint` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]
- L180 `grab_live_a_window_partly_off_screen_reports_off_screen` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]
- L213 `grab_live_the_capture_permission_follows_the_session` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]
- L241 `grab_live_a_chromium_window_is_captured_on_its_client_unit` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]
- L269 `grab_live_a_qt_window_is_captured_plain_and_covered` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]
- L315 `grab_live_a_stopped_window_is_app_unresponsive` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/system/capture/wayland_portal_live_tests.rs` (10 tests)

- L57 `no_answer_is_a_timeout_that_names_the_dialog_and_leaves_no_file` — #[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
- L80 `allow_gives_the_desktop_and_leaves_no_file` — #[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
- L102 `deny_is_permission_denied_and_leaves_no_file` — #[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
- L125 `one_allow_is_remembered_for_the_host_process_and_later_calls_show_no_dialog` — #[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
- L147 `one_deny_is_remembered_and_later_calls_are_refused_at_once` — #[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
- L164 `each_screen_is_its_rectangle_of_the_portals_image_on_two_monitors` — #[ignore = "boots a private headless gnome-shell with two virtual monitors; run inside scripts/private-session.sh"]
- L194 `a_scaled_monitor_is_cropped_in_image_pixels` — #[ignore = "boots a private headless gnome-shell with a scaled monitor; run inside scripts/private-session.sh"]
- L225 `the_portal_calls_go_to_the_session_bus_and_not_the_accessibility_bus` — #[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
- L288 `a_silent_dialog_ends_at_the_portals_limit_and_a_late_allow_leaves_nothing` — #[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]
- L330 `a_second_request_while_a_dialog_is_open_names_the_open_dialog` — #[ignore = "boots a private headless gnome-shell and its portal; run inside scripts/private-session.sh"]

Helper source sleeps in this module:
- L37: `std::thread::sleep(DIALOG_SHOWN);`
- L86: `std::thread::sleep(Duration::from_millis(400));`
- L131: `std::thread::sleep(Duration::from_millis(400));`
- L199: `std::thread::sleep(Duration::from_secs(3));`
- L313: `std::thread::sleep(Duration::from_millis(400));`
- L315: `std::thread::sleep(Duration::from_secs(4));`

### `crates/linux/src/system/close_live_tests.rs` (14 tests)

- L68 `close_live_calculator_graceful` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L79 `close_live_gtk3_graceful` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L86 `close_live_gtk4_graceful` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L93 `close_live_qt_graceful` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L100 `close_live_chromium_graceful` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L107 `close_live_text_editor_two_windows_graceful` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L126 `close_live_a_stopped_app_is_unresponsive_gracefully_and_killed_by_force` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L157 `close_live_a_child_that_ignores_term_is_killed_after_the_grace` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L179 `close_live_a_plain_term_is_enough_and_kill_is_never_sent_first` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L195 `close_live_a_changed_token_leaves_the_live_process_alone` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L207 `close_live_protected_decoys_are_refused_and_keep_running` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L255 `close_live_text_editor_unsaved_document_times_out_without_a_signal` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L322 `close_live_the_private_sessions_own_processes_are_refused` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L375 `close_live_a_private_gnome_shell_is_refused_and_keeps_running` — #[ignore = "needs a private session without a window manager; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- L48: `std::thread::sleep(Duration::from_millis(200));`
- L120: `std::thread::sleep(Duration::from_millis(200));`
- L145: `std::thread::sleep(Duration::from_millis(20));`
- L250: `std::thread::sleep(Duration::from_secs(1));`
- L368: `std::thread::sleep(Duration::from_millis(250));`
Timeout/readiness constants:
- L15: `const SETTLE: Duration = Duration::from_secs(40);`

### `crates/linux/src/system/cursor_overlay_cli_tests.rs` (2 tests)

- L77 `live_x11_cursor_cli_reports_rendered_and_paints_agent_positions` — #[ignore = "private X11 desktop and AGENT_DESKTOP_TEST_BINARY pointing to the candidate CLI"]
- L107 `live_wayland_cursor_cli_reports_unrendered_without_x11_fallback` — #[ignore = "private headless Wayland and AGENT_DESKTOP_TEST_BINARY pointing to the candidate CLI"]

Helper source sleeps in this module:
- none

### `crates/linux/src/system/cursor_overlay_tests.rs` (2 tests)

- L60 `live_x11_cursor_adapter_paints_agent_positions_and_retires_resources` — #[ignore = "needs a private X11 desktop; run through scripts/private-session.sh"]
- L184 `live_wayland_cursor_refusal_reports_unrendered_without_an_x11_window` — #[ignore = "needs scripts/private-session.sh --wm -- scripts/private-wayland-shell.sh --"]

Helper source sleeps in this module:
- none

### `crates/linux/src/system/display_live_tests.rs` (1 tests)

- L7 `display_live_lists_the_private_screen_even_with_a_stale_wayland_variable` — #[ignore = "needs a private X session; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/system/focus_activation_live_tests.rs` (1 tests)

- L101 `focus_window_takes_focus_back_from_a_window_another_tool_activated` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- L54: `std::thread::sleep(Duration::from_millis(200));`
- L97: `std::thread::sleep(Duration::from_millis(500));`
Timeout/readiness constants:
- L11: `const SETTLE: Duration = Duration::from_secs(40);`

### `crates/linux/src/system/focus_fallback_live_tests.rs` (5 tests)

- L68 `focus_window_live_f1_never_focused_conditions_are_measured` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L143 `focus_window_live_a_modal_dialog_is_named_and_the_window_is_not_reported_active` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L164 `focus_window_live_a_stopped_application_is_refused_before_any_request` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L203 `focus_window_live_a_window_on_another_desktop_switches_the_desktop` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L228 `focus_window_live_a_zenity_dialog_the_toolkit_never_reports_active_is_focused` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- L16: `std::thread::sleep(Duration::from_millis(100));`
- L82: `std::thread::sleep(Duration::from_secs(2));`
- L97: `std::thread::sleep(Duration::from_secs(1));`
- L112: `std::thread::sleep(Duration::from_secs(1));`
- L126: `std::thread::sleep(Duration::from_secs(2));`
- L149: `std::thread::sleep(Duration::from_secs(2));`
- L218: `std::thread::sleep(Duration::from_millis(1_500));`
- L221: `std::thread::sleep(Duration::from_millis(1_000));`
- L245: `std::thread::sleep(Duration::from_millis(200));`

### `crates/linux/src/system/focus_lift_live_tests.rs` (1 tests)

- L225 `focus_window_live_a_minimized_window_ends_as_measured_for_the_manager` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- L97: `std::thread::sleep(Duration::from_millis(200));`
- L114: `std::thread::sleep(Duration::from_millis(100));`
- L184: `std::thread::sleep(Duration::from_millis(200));`
- L200: `std::thread::sleep(Duration::from_millis(2_500));`
- L244: `std::thread::sleep(Duration::from_millis(100));`
- L247: `std::thread::sleep(Duration::from_millis(PAUSE_MS));`
- L249: `std::thread::sleep(Duration::from_millis(PAUSE_MS));`
Timeout/readiness constants:
- L11: `pub(super) const SETTLE: Duration = Duration::from_secs(40);`

### `crates/linux/src/system/focus_overview_live_tests.rs` (1 tests)

- L89 `focus_window_names_an_open_overview_and_succeeds_once_it_is_closed` — #[ignore = "drives a real desktop session; run through scripts/private-session.sh, or a private gnome-shell with AGENT_DESKTOP_LIVE_WM=mutter"]

Helper source sleeps in this module:
- L50: `std::thread::sleep(Duration::from_millis(150));`
- L69: `std::thread::sleep(Duration::from_millis(200));`
Timeout/readiness constants:
- L12: `const SETTLE: Duration = Duration::from_secs(40);`

### `crates/linux/src/system/focus_window_live_tests.rs` (2 tests)

- L259 `focus_window_live_activates_a_background_window_per_toolkit` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
- L297 `focus_window_live_stacking_matches_the_root_list_and_sees_an_override_redirect_cover` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]

Helper source sleeps in this module:
- L127: `std::thread::sleep(Duration::from_millis(200));`
- L162: `std::thread::sleep(Duration::from_millis(150));`
- L182: `std::thread::sleep(Duration::from_millis(100));`
- L194: `std::thread::sleep(Duration::from_millis(1_500));`
- L291: `std::thread::sleep(Duration::from_millis(200));`
- L358: `std::thread::sleep(Duration::from_millis(200));`
Timeout/readiness constants:
- L15: `const SETTLE: Duration = Duration::from_secs(40);`

### `crates/linux/src/system/key_dispatch_live_tests.rs` (2 tests)

- L193 `key_dispatch_live_an_active_target_gets_the_chord_under_either_policy` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L224 `key_dispatch_live_a_background_target_gets_nothing_under_either_policy` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- L91: `std::thread::sleep(Duration::from_millis(2_500));`
- L122: `std::thread::sleep(Duration::from_millis(200));`
- L163: `std::thread::sleep(Duration::from_millis(400));`
- L187: `std::thread::sleep(Duration::from_millis(100));`
- L201: `std::thread::sleep(Duration::from_secs(1));`
- L242: `std::thread::sleep(Duration::from_millis(500));`
Timeout/readiness constants:
- L15: `const SETTLE: Duration = Duration::from_secs(40);`

### `crates/linux/src/system/launch_live_tests.rs` (8 tests)

- L143 `live_calculator_launches_by_command_id_path_and_entry_name` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L160 `live_a_second_calculator_attaches_without_a_spawn_and_with_arguments_is_a_new_owner` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L183 `live_text_editor_by_desktop_id_then_a_file_reaches_the_first_instance` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L231 `live_a_qt_app_launches_by_path_with_the_environment_given` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L277 `live_chrome_launches_as_chromium_with_a_private_profile` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L300 `live_terminal_is_attributed_to_its_server_and_a_second_launch_adds_a_window` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L319 `live_a_missing_name_is_app_not_found_and_nothing_started` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L326 `live_an_app_that_exits_after_registering_and_before_a_window_is_not_a_launch` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L56: `std::thread::sleep(Duration::from_millis(100));`
- L106: `std::thread::sleep(Duration::from_millis(200));`
- L332: `"import gi, time\ngi.require_version('Gtk', '3.0')\nfrom gi.repository import Gtk, GLib\nGtk.init()\nend = time.time() + 1.5\nwhile time.time() < end:\n    GLib.MainContext.default().iteration(False)\n    time.sleep(0.05)\n",`
Timeout/readiness constants:
- L15: `const SETTLE: Duration = Duration::from_secs(20);`

### `crates/linux/src/system/launch_resident_live_tests.rs` (3 tests)

- L27 `live_a_running_app_with_no_window_is_started_again_and_a_window_is_reported` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L76 `live_a_forwarded_launch_reports_the_new_window_not_the_old_one` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L92 `live_a_launch_that_shows_no_window_does_not_wait_for_the_whole_timeout` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L43: `std::thread::sleep(Duration::from_millis(200));`

### `crates/linux/src/system/menu_wait_live_tests.rs` (9 tests)

- L124 `menu_wait_live_calculator_main_menu_both_directions` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L130 `menu_wait_live_files_main_menu_both_directions` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L165 `menu_wait_live_terminal_bar_menu_opens_and_closes_by_escape` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L189 `menu_wait_live_terminal_header_popover_opens_and_closes_by_click` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L206 `menu_wait_live_terminal_hidden_bar_times_out_with_the_direction_message` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L232 `menu_wait_live_qt_file_menu_both_directions` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L254 `menu_wait_live_chrome_menu_opens_and_closes_by_escape` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L283 `menu_wait_live_target_exiting_mid_wait_is_stale_ref` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L315 `menu_wait_live_stopped_target_is_app_unresponsive_without_the_full_wait` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L54: `std::thread::sleep(Duration::from_millis(1_000));`
- L293: `std::thread::sleep(Duration::from_millis(1_500));`

### `crates/linux/src/system/process_state_live_tests.rs` (3 tests)

- L46 `process_state_live_calculator_running_unresponsive_running_exited` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L90 `process_state_live_files_keeps_its_name_across_stop_and_continue` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L123 `process_state_live_a_blocked_main_loop_is_unresponsive_while_it_sleeps_in_proc` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L40: `std::thread::sleep(Duration::from_millis(100));`
- L118: `GLib.timeout_add(4000, lambda: time.sleep(120))`
- L143: `std::thread::sleep(Duration::from_millis(200));`
Timeout/readiness constants:
- L13: `const SETTLE: Duration = Duration::from_secs(20);`

### `crates/linux/src/system/registry_live_tests.rs` (3 tests)

- L44 `registry_live_lists_apps_and_windows_and_keeps_a_stopped_app` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
- L130 `registry_live_names_a_running_process_that_is_not_on_the_bus` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
- L156 `registry_live_focus_agrees_with_the_window_manager` — #[ignore = "drives real desktop applications under a window manager; run through scripts/remote-linux.sh"]

Helper source sleeps in this module:
- L34: `std::thread::sleep(Duration::from_millis(200));`
- L212: `std::thread::sleep(Duration::from_millis(300));`
Timeout/readiness constants:
- L10: `const SETTLE: Duration = Duration::from_secs(40);`

### `crates/linux/src/system/sheet_live_tests.rs` (3 tests)

- L298 `sheet_live_gtk4_about_is_a_modal_frame_sheet` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L304 `sheet_live_gtk4_keyboard_shortcuts_is_a_modal_panel_window` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L310 `sheet_live_gtk3_about_is_a_showing_dialog_sheet` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- L194: `std::thread::sleep(Duration::from_millis(100));`
Timeout/readiness constants:
- L12: `const SETTLE: Duration = Duration::from_secs(40);`
- L19: `const NODE_LIMIT: usize = 4000;`

### `crates/linux/src/system/signal_inventory_live_tests.rs` (1 tests)

- L65 `signal_inventory_live_a_stopped_target_is_skipped_slow_roots_read_timed_out_and_resolution_names_it` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L32: `std::thread::sleep(Duration::from_millis(200));`
- L165: `std::thread::sleep(Duration::from_millis(200));`
- L178: `std::thread::sleep(Duration::from_millis(200));`
Timeout/readiness constants:
- L12: `const SETTLE: Duration = Duration::from_secs(40);`

### `crates/linux/src/system/signal_live_surfaces_tests.rs` (7 tests)

- L91 `signal_live_gtk4_about_dialog_is_a_sheet_by_identity` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L101 `signal_live_gtk4_keyboard_shortcuts_unnamed_panel_is_a_sheet_by_identity` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L111 `signal_live_chrome_menu_is_not_window_opened_and_two_windows_diff_correctly` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L184 `signal_live_stopped_target_scoped_wait_answers_app_unresponsive_at_once` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L214 `signal_live_window_taken_before_a_stop_resolves_to_app_unresponsive` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L238 `signal_live_unscoped_wait_with_an_unrelated_stopped_app_reports_no_events` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L281 `signal_live_slow_but_alive_target_is_incomplete_and_not_unresponsive` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L292: `std::thread::sleep(Duration::from_millis(800));`

### `crates/linux/src/system/signal_live_tests.rs` (10 tests)

- L37 `signal_live_text_editor_new_window_handoff_is_window_opened_without_app_launched` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L72 `signal_live_calculator_launch_and_close_are_app_and_window_events` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L126 `signal_live_second_calculator_process_is_a_new_app_with_a_distinct_id` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L159 `signal_live_text_editor_ctrl_n_is_window_opened_under_the_same_pid` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L193 `signal_live_sigkilled_calculator_is_window_closed_and_app_terminated` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L232 `signal_live_helper_with_no_frames_is_not_an_app` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L258 `signal_live_two_windows_focus_changes_under_the_window_manager` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L289 `signal_live_wayland_without_input_times_out_with_baseline_counts` — #[ignore = "needs a Wayland session with no input device; run through the owner's Wayland harness"]
- L314 `signal_live_window_that_opens_and_closes_inside_one_wait_is_reported` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]
- L352 `signal_live_batch_seed_before_the_action_is_complete_and_the_close_is_found` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L335: `std::thread::sleep(Duration::from_millis(2_000));`

### `crates/linux/src/system/stopped_live_tests.rs` (2 tests)

- L130 `stopped_live_calculator_keeps_its_name_and_reports_unresponsive` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L136 `stopped_live_files_keeps_its_name_and_reports_unresponsive` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- L68: `std::thread::sleep(Duration::from_millis(200));`
- L124: `std::thread::sleep(Duration::from_millis(200));`
Timeout/readiness constants:
- L13: `const SETTLE: Duration = Duration::from_secs(40);`

### `crates/linux/src/system/window_op_live_env_tests.rs` (6 tests)

- L34 `window_op_live_a_stale_wayland_display_still_proceeds_on_x11` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
- L42 `window_op_live_a_session_without_a_display_is_refused_before_any_write` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
- L53 `window_op_live_child_stale_wayland` — #[ignore = "runs only inside the parent test that sets its environment"]
- L68 `window_op_live_child_no_display` — #[ignore = "runs only inside the parent test that sets its environment"]
- L297 `window_op_live_an_xwayland_window_is_operated_and_a_native_wayland_window_is_refused` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
- L312 `window_op_live_child_xwayland` — #[ignore = "runs only inside the parent test that sets its environment"]

Helper source sleeps in this module:
- L124: `std::thread::sleep(Duration::from_millis(100));`
- L173: `std::thread::sleep(Duration::from_millis(100));`
- L228: `std::thread::sleep(Duration::from_secs(1));`
- L273: `std::thread::sleep(Duration::from_millis(200));`
- L275: `std::thread::sleep(Duration::from_secs(8));`
- L292: `std::thread::sleep(Duration::from_secs(2));`
- L326: `std::thread::sleep(Duration::from_millis(1_500));`

### `crates/linux/src/system/window_op_live_tests.rs` (5 tests)

- L305 `window_op_live_gtk3` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
- L311 `window_op_live_gtk4` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
- L317 `window_op_live_qt` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
- L323 `window_op_live_a_destroyed_window_is_bad_window_and_not_success` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]
- L353 `window_op_live_a_restore_of_a_minimized_window_works_every_time` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]

Helper source sleeps in this module:
- L132: `std::thread::sleep(Duration::from_millis(200));`
- L156: `std::thread::sleep(Duration::from_millis(200));`
- L217: `std::thread::sleep(Duration::from_millis(50));`
- L228: `std::thread::sleep(Duration::from_millis(1_500));`
- L293: `std::thread::sleep(Duration::from_millis(2_500));`
- L362: `std::thread::sleep(Duration::from_millis(1_500));`
Timeout/readiness constants:
- L18: `const SETTLE: Duration = Duration::from_secs(40);`

### `crates/linux/src/system/x11/grab_live_tests.rs` (6 tests)

- L120 `grab_live_a_flat_window_reads_back_its_fill_and_matches_an_independent_capture` — #[ignore = "needs a private X session; run through scripts/private-session.sh --wm"]
- L165 `grab_live_describes_the_window_and_names_each_refusal` — #[ignore = "needs a private X session; run through scripts/private-session.sh --wm"]
- L218 `grab_live_the_root_grab_and_the_compositing_read_answer_on_the_private_screen` — #[ignore = "needs a private X session; run through scripts/private-session.sh --wm"]
- L239 `grab_live_the_root_is_drawable_zero_in_root_coordinates` — #[ignore = "needs a private X session; run through scripts/private-session.sh --wm"]
- L285 `grab_live_a_framed_window_has_a_parent_and_every_rect_is_in_root_coordinates` — #[ignore = "needs a private X session; run through scripts/private-session.sh --wm"]
- L325 `grab_live_a_covered_window_is_blank_directly_and_painted_through_the_redirect` — #[ignore = "needs a private X session; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L114: `Err(_) => std::thread::sleep(Duration::from_millis(100)),`
- L300: `std::thread::sleep(Duration::from_millis(100));`
- L339: `std::thread::sleep(Duration::from_millis(300));`

### `crates/linux/src/tree/absence_live_tests.rs` (4 tests)

- L77 `absence_live_a_killed_app_is_a_settled_stale_ref_in_one_pass_for_a_ref_and_a_held_handle` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L109 `absence_live_a_stopped_app_is_unresponsive_within_the_budget_and_never_stale` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L150 `absence_live_a_removed_element_is_a_complete_miss_and_a_held_handle_is_recorded_per_toolkit` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L200 `absence_live_a_gtk4_page_that_leaves_the_tree_is_stale_and_returns_at_the_same_path` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/capability_live_tests.rs` (1 tests)

- L181 `capability_live_every_token_is_backed_by_an_interface_or_an_action` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/chromium_live_tests.rs` (2 tests)

- L45 `chromium_live_a_killed_vscode_is_a_settled_stale_ref_in_one_pass` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L76 `chromium_live_opening_a_second_file_shifts_no_web_content_path` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- L98: `std::thread::sleep(Duration::from_secs(3));`

### `crates/linux/src/tree/find_live_tests.rs` (1 tests)

- L210 `find_live_returns_matches_and_refs_that_resolve_back_to_their_nodes` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/hit_live_tests/cases.rs` (7 tests)

- L12 `hit_live_gtk3_terminal_cover_in_both_directions` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L19 `hit_live_qt_cover_in_both_directions` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L26 `hit_live_gtk4_calculator_at_two_positions` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L39 `hit_live_gtk4_files_at_two_positions` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L52 `hit_live_vscode_is_never_a_wrong_verdict` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L62 `hit_live_chrome_cover_in_both_directions` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L88 `hit_live_files_zero_axis_group_is_unknown` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/live_horizontal_live_tests.rs` (2 tests)

- L133 `strip_live_gtk3_horizontal_bar_scrolls_a_far_right_item_into_view` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L139 `strip_live_gtk4_horizontal_bar_scrolls_a_far_right_item_into_view` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/live_offscreen_live_tests.rs` (7 tests)

- L222 `offscreen_live_gtk3_scrolled_list_realises_an_offscreen_row` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L228 `offscreen_live_gtk4_scrolled_list_realises_an_offscreen_row` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L234 `offscreen_live_qt_list_item_cannot_be_scrolled_to` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L281 `offscreen_live_files_grid_realises_an_offscreen_row` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L305 `offscreen_live_terminal_survey_agrees_with_the_oracle` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L320 `offscreen_live_chrome_page_realises_an_offscreen_button` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L356 `offscreen_live_vscode_scroll_rungs_are_measured` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/live_read_live_tests.rs` (3 tests)

- L136 `live_read_every_projection_agrees_with_a_second_read_on_each_toolkit` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]
- L157 `live_read_a_calculator_display_follows_a_button_press_made_by_the_harness` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L227 `live_read_a_password_value_is_withheld_on_gtk3_gtk4_and_qt` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/live_timeout.rs` (1 tests)

- L34 `walker_live_stopped_app_returns_a_partial_walk_at_the_deadline` — #[ignore = "drives a real desktop application; run through scripts/remote-linux.sh"]

Helper source sleeps in this module:
- none
Timeout/readiness constants:
- L14: `const DEADLINE_MS: u64 = 3_000;`

### `crates/linux/src/tree/navigate_live_tests.rs` (3 tests)

- L20 `navigate_live_a_settings_panel_ref_is_stale_after_another_panel_replaces_it` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L59 `navigate_live_every_ref_of_a_relaunched_app_is_stale_through_the_gate` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L102 `navigate_live_closing_one_files_window_leaves_its_refs_stale_and_the_other_window_resolving` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/observe_live_tests.rs` (1 tests)

- L22 `observe_live_calculator_returns_a_complete_reffed_tree` — #[ignore = "drives a real desktop application; run through scripts/remote-linux.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/perf_live_tests.rs` (1 tests)

- L194 `perf_live_costs_min_of_seven_with_the_warm_up_discarded` — #[ignore = "measures real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/physical_live_apps_tests.rs` (5 tests)

- L61 `live_text_editor_receives_typed_text_with_remapped_characters` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L70 `live_calculator_evaluates_typed_keys` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L79 `live_the_wheel_scrolls_the_files_list` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L93 `live_qt_headless_reveal_has_no_container_while_the_headed_wheel_moves_the_content` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L144 `live_the_wheel_scrolls_a_chromium_page` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L20: `std::thread::sleep(Duration::from_millis(500));`
- L35: `std::thread::sleep(Duration::from_millis(300));`
- L38: `std::thread::sleep(Duration::from_millis(100));`
- L109: `std::thread::sleep(Duration::from_millis(800));`
- L138: `std::thread::sleep(Duration::from_millis(200));`

### `crates/linux/src/tree/physical_live_mutter_tests.rs` (1 tests)

- L44 `live_a_raw_click_activates_a_window_only_once_the_overview_is_closed` — #[ignore = "drives its own private gnome-shell --x11; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L68: `std::thread::sleep(Duration::from_millis(200));`
Timeout/readiness constants:
- L14: `const SETTLE: Duration = Duration::from_secs(15);`
- L40: `const CHILD_BUDGET: Duration = Duration::from_secs(120);`

### `crates/linux/src/tree/physical_live_tests.rs` (15 tests)

- L121 `live_type_into_a_gtk3_entry_reads_back_with_remapped_characters` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L127 `live_type_into_a_gtk4_entry_reads_back_with_remapped_characters` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L133 `live_type_into_a_qt_entry_reads_back_with_remapped_characters` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L139 `live_key_combos_select_and_delete_in_a_gtk3_entry` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L145 `live_key_combos_select_and_delete_in_a_gtk4_entry` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L151 `live_key_combos_select_and_delete_in_a_qt_entry` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L157 `live_a_click_at_the_bounds_centre_presses_a_gtk3_button` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L163 `live_a_click_at_the_bounds_centre_presses_a_gtk4_button` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L169 `live_a_click_at_the_bounds_centre_presses_a_qt_button` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L175 `live_the_wheel_scrolls_a_gtk3_list_and_back` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L181 `live_the_wheel_scrolls_a_gtk4_list_and_back` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L187 `live_the_wheel_scrolls_a_qt_list_and_back` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L193 `live_a_drag_presses_moves_and_releases_the_button` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L242 `live_a_frame_that_is_not_active_refuses_with_window_active` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]
- L266 `live_keys_under_a_menu_grab_are_reported_unverified` — #[ignore = "drives real applications; run through scripts/private-session.sh --wm"]

Helper source sleeps in this module:
- L36: `std::thread::sleep(Duration::from_millis(300));`
- L252: `std::thread::sleep(Duration::from_secs(1));`
- L260: `std::thread::sleep(Duration::from_millis(500));`
- L275: `std::thread::sleep(Duration::from_millis(1200));`
- L277: `std::thread::sleep(Duration::from_millis(800));`
- L301: `std::thread::sleep(Duration::from_millis(500));`

### `crates/linux/src/tree/qt_live_tests.rs` (3 tests)

- L44 `qt_live_twins_resolve_to_their_own_button_after_the_window_moves` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L90 `qt_live_rows_after_a_resort_resolve_to_their_own_text_or_are_refused_never_swapped` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L132 `qt_live_a_process_with_two_bus_connections_resolves_through_the_window_owning_one` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- L103: `std::thread::sleep(Duration::from_millis(600));`

### `crates/linux/src/tree/registry_toolkit_live_tests.rs` (1 tests)

- L38 `registry_toolkit_live_reads_gtk3_and_qt_that_refuse_the_group_read` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/resolve_action_live_tests.rs` (1 tests)

- L58 `live_a_fresh_files_click_changes_native_window_state` — #[ignore = "private X11 desktop and AGENT_DESKTOP_TEST_BINARY pointing to the candidate CLI"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/resolve_live_tests.rs` (1 tests)

- L182 `resolve_live_every_snapshot_ref_resolves_to_the_node_it_was_taken_from` — #[ignore = "drives real desktop applications; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/resolve_twin_live_tests.rs` (3 tests)

- L115 `resolve_live_files_view_options_nodes_each_resolve_to_their_own_node` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L166 `resolve_live_twin_buttons_are_told_apart_by_path_and_by_stored_size_and_stay_ambiguous_without` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]
- L220 `resolve_live_files_row_refs_survive_a_sibling_row_leaving` — #[ignore = "drives a real desktop application; run through scripts/private-session.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/vocabulary_live_tests.rs` (1 tests)

- L293 `vocabulary_live_real_apps` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]

Helper source sleeps in this module:
- none

### `crates/linux/src/tree/walker_live_tests.rs` (1 tests)

- L269 `walker_live_real_apps` — #[ignore = "drives real desktop applications; run through scripts/remote-linux.sh"]

Helper source sleeps in this module:
- L37: `std::thread::sleep(POLL);`
- L168: `std::thread::sleep(POLL);`
Timeout/readiness constants:
- L15: `const SETTLE_LIMIT: Duration = Duration::from_secs(40);`
- L16: `const POLL: Duration = Duration::from_millis(100);`
- L17: `const STABLE_POLLS: usize = 3;`

## Shared harness cost outside the test files

- `scripts/private-session.sh`: waits for session bus (up to 10 s at 100 ms polling), Xvfb (up to 5 s per candidate; candidates 100–149), and Openbox (up to 10 s at 100 ms polling); always sleeps 1 s during teardown before SIGKILL. These are environment startup/cleanup delays, not test coverage. Share one private session for the full ignored-test binary rather than invoking the script once per test.
- `tree/live_launch.rs`: every `Launched` drop kills child process groups, reaps direct children, scans `/proc` to terminate only this session’s D-Bus activated terminal servers, then removes fixture scratch dirs. Preserve this; session-wide reuse needs explicit ownership cleanup.
- `tree/live_rig.rs`: `settled` delegates to 100 ms polling `walker_live_tests::settle`; `wait_until` polls every 100 ms; adapter bus/read/resolve calls have 5 s budgets. Replace elapsed-time sleeps with these bounded state checks when their predicate establishes the required state.
- `input/clipboard/set_clear_live_support.rs`: clipboard live tests serialize through a mutex, build binary/helper once with `Once`, and `wait_owner` polls every 50 ms. Per-test binary build is already amortized within the process.
- Shell `sleep` is only harness readiness/cleanup and should remain bounded; timeout budgets can be reduced only after measuring cold-start distribution.

## Narrow optimization guidance

1. Keep the user’s new-owner rule: tests that only pass/fail based on a noisy environment should be deleted or replaced by deterministic behavioral tests. Keep every assertion for wrong-element targeting, delivery/uncertain delivery, refusal, cleanup/leak, routing, and clipboard/capture.
2. Run all ignored tests in one private-session invocation per mode (`--wm`/non-WM, X11/Wayland), not one process/session per test. Current shell teardown contributes a fixed 1 s per invocation.
3. Convert fixed sleeps to bounded polling against the exact asserted postcondition (window exists, focus is active, menu state changed, event arrived, clipboard owner changed). Do not lower operation deadlines or remove the final independent readback.
4. For toolkit matrices, reuse app processes only when tests retain isolated fixture state and each action still gets independent readback; otherwise group actions within a single fixture only if assertions remain per action.
5. Measure perf min-of-seven independently before deciding its disposition. Run portal/WM-specific cases in dedicated lanes and keep their coverage.
## Shared live-helper sleep inventory

These sleeps live outside the ignored test bodies and are called by live suites. Polling intervals are waits within bounded conditions; the other entries are fixed setup, observation, or cleanup delays.

### `tree/live_rig.rs`

- L129 100 ms `wait_until` polling

### `tree/walker_live_tests.rs`

- L37 `POLL` (100 ms) while settling a frame
- L168 `POLL` (100 ms) while waiting for the target

### `tree/physical_live_rig.rs`

- L158 150 ms
- L179 100 ms

### `tree/physical_live_x11.rs`

- L40 `POLL` (100 ms)
- L48 `POLL` (100 ms)

### `system/close_live/rig.rs`

- L94 20 ms
- L133 200 ms
- L164 10 ms
- L176 20 ms
- L293 500 ms

### `system/signal_live_apps.rs`

- L124 100 ms
- L267 400 ms

### `system/signal_live_launch.rs`

- L24 embedded fixture process sleeps 60 s
- L47 1.2 s
- L137 500 ms
- L145 500 ms
- L187 500 ms

### `system/capture/capture_live_support.rs`

- L201 1.2 s
- L204 200 ms
- L218 1.2 s
- L224 200 ms

### `system/capture/wayland_portal_live_support.rs`

- L104 3 s
- L110 200 ms
- L164 1 s
- L235 300 ms
- L252 1 s
- L280 2 s
- L360 1.5 s
- L362 3 s

### `input/wayland/live_shell.rs`

- L55 400 ms
- L96 200 ms
- L101 3 s
- L124 2 s

### `input/clipboard/set_clear_live_support.rs`

- L191 50 ms `wait_owner` polling



## Per-test qualified inventory and direct costs
Qualified module names follow the Rust `mod`/`#[path]` registration (including nested `owner_tests`, `resident`, `environment`, and `cases`). Each case maps to one shared profile below; runner-level private-session cost is once per test-binary invocation, not once per test. “Direct” means written in that test function. Helper behavior is assigned through the profile and shared-helper appendix above. `-` means no literal fixed sleep in that function; it does not imply zero waiting.

| Qualified test | Shared profile | Direct fixed sleeps | Direct polling sleep | Deadline / wait refs | Direct launch / session calls |
|---|---|---|---|---|---|
| `actions::live_click_tests::a_calculator_digit_click_is_delivered_and_unverified_on_its_own_node` | `actions_live` | - | - | - | - |
| `actions::live_click_tests::a_click_on_an_insensitive_undo_is_not_delivered` | `actions_live` | - | - | - | - |
| `actions::live_click_tests::right_click_without_a_context_menu_verb_is_not_supported` | `actions_live` | - | - | - | - |
| `actions::live_click_tests::headless_focus_is_denied_before_any_write` | `actions_live` | - | - | - | - |
| `actions::live_click_tests::headed_focus_on_a_gtk_button_without_a_focus_verb_is_not_supported` | `actions_live` | - | - | - | - |
| `actions::live_disclosure_tests::the_calculator_main_menu_expands_and_collapses_through_its_inner_toggle` | `actions_live` | - | - | - | - |
| `actions::live_disclosure_tests::collapsing_a_collapsed_menu_button_writes_nothing` | `actions_live` | - | - | - | - |
| `actions::live_disclosure_tests::a_digit_button_is_a_leaf_and_is_never_expanded` | `actions_live` | - | - | - | - |
| `actions::live_disclosure_tests::a_files_tree_row_expands_and_collapses_in_both_directions` | `actions_live` | - | - | - | - |
| `actions::live_select_tests::a_settings_sidebar_row_that_ignores_the_selection_is_a_write_ignored_failure` | `actions_live` | - | - | - | - |
| `actions::live_select_tests::a_value_with_no_match_in_the_sidebar_is_not_found_and_writes_nothing` | `actions_live` | - | - | - | - |
| `actions::live_select_tests::a_button_with_no_selection_route_is_not_supported` | `actions_live` | - | - | - | - |
| `actions::live_select_tests::a_gtk4_drop_down_is_chosen_through_its_popup_and_the_shown_value_follows` | `actions_live` | - | - | - | - |
| `actions::live_text_tests::set_value_and_clear_land_on_a_text_field_in_every_toolkit` | `actions_live` | - | - | - | - |
| `actions::live_text_tests::type_inserts_non_ascii_text_into_an_empty_field_exactly` | `actions_live` | - | - | - | - |
| `actions::live_text_tests::type_after_a_wide_character_lands_at_the_caret_exactly` | `actions_live` | - | - | - | - |
| `actions::live_text_tests::a_password_field_write_lands_and_the_result_never_carries_the_text` | `actions_live` | - | - | - | - |
| `actions::live_toggle_tests::settings_hot_corner_checks_and_unchecks_with_read_back` | `actions_live` | - | - | - | - |
| `actions::live_toggle_tests::checking_a_checked_switch_writes_nothing` | `actions_live` | - | - | - | - |
| `actions::live_toggle_tests::toggle_on_a_push_button_is_not_supported` | `actions_live` | - | - | - | - |
| `actions::live_value_tests::a_spin_button_takes_a_whole_number_and_refuses_a_fraction_in_every_toolkit` | `actions_live` | - | - | - | - |
| `actions::live_value_tests::a_slider_is_clamped_and_the_stored_value_is_not_reported_as_verified` | `actions_live` | - | - | - | - |
| `actions::live_value_tests::a_scroll_bar_write_moves_the_content` | `actions_live` | - | - | - | - |
| `actions::live_widget_tests::a_click_runs_the_handler_of_a_button_in_every_toolkit` | `actions_live` | - | - | - | - |
| `actions::live_widget_tests::toggle_check_and_uncheck_move_a_check_box_and_never_write_twice` | `actions_live` | L93 `std::thread::sleep(Duration::from_millis(500));` | - | Duration::from_millis(500) | - |
| `actions::live_widget_tests::select_by_display_value_chooses_the_item_of_a_combo_box` | `actions_live` | - | - | - | - |
| `actions::live_widget_tests::select_by_display_value_chooses_the_item_of_a_qt_list` | `actions_live` | - | - | - | - |
| `actions::live_widget_tests::a_headed_set_focus_moves_the_focus_to_a_qt_button` | `actions_live` | - | - | - | - |
| `actions::live_widget_tests::the_selection_reads_in_utf16_and_the_caret_in_code_points` | `actions_live` | - | - | Deadline::after(5_000) | - |
| `actions::live_widget_tests::the_read_only_menu_item_of_gnome_terminal_toggles` | `actions_live` | - | - | - | let mut launched = Launched::new();; launched.spawn( |
| `actions::live_widget_tests::a_gecko_page_takes_a_click_and_a_check` | `actions_live` | - | - | - | - |
| `actions::live_widget_tests::a_plain_qwidget_and_a_qmainwindow_are_both_listed_windows` | `actions_live` | - | - | - | let mut launched = Launched::new(); |
| `actions::live_write_tests::write_live_a_named_action_toggles_a_check_box_and_an_independent_read_sees_it` | `actions_live` | - | - | SETTLE | let app = launch(label); |
| `actions::live_write_tests::write_live_the_chain_skips_an_absent_verb_and_reports_what_a_re_read_saw` | `actions_live` | - | - | SETTLE | let app = launch(label); |
| `actions::live_write_tests::write_live_set_text_contents_lands_and_an_independent_read_returns_it` | `actions_live` | - | - | SETTLE | let app = launch(label); |
| `actions::live_write_tests::write_live_a_value_property_write_moves_a_slider` | `actions_live` | - | - | SETTLE | let app = launch(label); |
| `actions::live_write_tests::write_live_a_read_only_value_never_reads_back_as_written` | `actions_live` | L178 `std::thread::sleep(Duration::from_millis(600));` | - | Duration::from_millis(600) | let app = launch(label); |
| `actions::live_write_tests::write_live_an_index_the_node_does_not_have_changes_nothing` | `actions_live` | L205 `std::thread::sleep(Duration::from_millis(600));` | - | Duration::from_millis(600) | let app = launch(label); |
| `actions::live_write_tests::write_live_a_destroyed_widget_is_stale_where_the_toolkit_reports_it` | `actions_live` | - | - | SETTLE | let app = launch(label); |
| `actions::live_write_tests::write_live_a_stopped_app_is_a_timeout_that_may_have_landed` | `actions_live` | - | - | Duration::from_millis(3_500), SETTLE | let app = launch(label); |
| `actions::live_write_tests::write_live_a_killed_app_is_a_stale_ref_that_was_not_delivered` | `actions_live` | - | - | SETTLE | let app = launch(label); |
| `input::clipboard::set_clear::live_tests::owner_tests::a_record_left_by_a_killed_owner_is_replaced_and_never_signalled` | `clipboard_x11` | - | - | - | let session = private_session(); |
| `input::clipboard::set_clear::live_tests::owner_tests::the_interaction_lease_descriptor_is_not_inherited_by_the_owner` | `clipboard_x11` | - | - | - | let session = private_session(); |
| `input::clipboard::set_clear::live_tests::owner_tests::a_state_directory_the_owner_cannot_use_leaves_the_earlier_clipboard_untouched` | `clipboard_x11` | - | - | - | let session = private_session(); |
| `input::clipboard::set_clear::live_tests::text_set_is_served_by_a_detached_owner_and_read_by_a_second_process` | `clipboard_x11` | - | - | - | let session = private_session(); |
| `input::clipboard::set_clear::live_tests::the_cli_exits_with_its_stdout_closed_while_the_owner_keeps_serving` | `clipboard_x11` | - | - | Duration::from_secs(15) | let output = Command::new(&cli); let cleared = Command::new(&cli); let session = private_session(); |
| `input::clipboard::set_clear::live_tests::clear_leaves_the_selection_unowned_and_ends_the_recorded_owner` | `clipboard_x11` | - | - | - | let session = private_session(); |
| `input::clipboard::set_clear::live_tests::a_second_set_replaces_the_first_owner_and_leaves_one_record` | `clipboard_x11` | - | - | - | let session = private_session(); |
| `input::clipboard::set_clear::live_tests::a_text_above_the_incr_threshold_round_trips_through_both_readers` | `clipboard_x11` | - | - | - | let session = private_session(); |
| `input::clipboard::set_clear::live_tests::an_image_is_served_as_png_and_read_back` | `clipboard_x11` | - | - | - | let session = private_session(); |
| `input::clipboard::set_clear::live_tests::file_paths_are_served_under_both_file_manager_targets` | `clipboard_x11` | - | - | - | let session = private_session(); |
| `input::clipboard::set_clear::live_tests::the_owner_exits_when_another_client_takes_the_selection` | `clipboard_x11` | - | - | - | let mut taker = Command::new("setsid"); .spawn(); let session = private_session(); |
| `input::clipboard::wayland::live_tests::text_round_trips_with_foreign_readers_and_writers` | `clipboard_wayland` | L289 `std::thread::sleep(Duration::from_secs(2));` | - | Deadline::after(8_000), Duration::from_secs(2) | let world = boot(); |
| `input::clipboard::wayland::live_tests::images_and_file_lists_round_trip` | `clipboard_wayland` | - | - | Deadline::after(8_000) | let world = boot(); |
| `input::wayland::live_tests::remote_desktop_answers_on_the_session_bus_and_not_on_the_accessibility_bus` | `wayland_remote` | - | - | deadline() | let _launched = Launched::new();; let _shell = Shell::start();; let mut desktop = RemoteDesktop::open(deadline()).expect("the session bus opens");; desktop.create(deadline()).expect("CreateSession succeeds");; desktop.start(deadline()).expect("Start succeeds");; .create(deadline()) |
| `input::wayland::live_tests::keys_land_in_a_wayland_native_gtk4_entry` | `wayland_remote` | - | - | - | let _guard = Launched::new();; let mut launched = Launched::new();; let shell = Shell::start(); |
| `input::x11::live_remap_tests::live_a_remap_left_by_a_killed_client_is_restored_by_the_next_call` | `x11_input` | L68 `std::thread::sleep(Duration::from_millis(500));` | - | Duration::from_millis(500), Duration::from_secs(60), deadline(70_000), deadline(8_000) | let mut child = Command::new(std::env::current_exe().expect("exe")); .spawn(); Command::new("kill") |
| `input::x11::live_tests::live_typing_reaches_a_gtk3_entry` | `x11_input` | - | - | - | - |
| `input::x11::live_tests::live_typing_reaches_a_gtk4_entry` | `x11_input` | - | - | - | - |
| `input::x11::live_tests::live_typing_reaches_a_qt_entry` | `x11_input` | - | - | - | - |
| `input::x11::live_tests::live_a_key_left_held_by_a_killed_client_is_released_by_the_next_call` | `x11_input` | L319 `std::thread::sleep(Duration::from_millis(100));`; L327 `std::thread::sleep(Duration::from_millis(500));` | - | Duration::from_millis(100), Duration::from_millis(500), Duration::from_secs(20), Duration::from_secs(60), deadline(70_000), deadline(8_000) | let mut child = Command::new(std::env::current_exe().expect("exe")); .spawn(); Command::new("kill") |
| `system::app_alias_live_tests::alias_live_calculator_resolves_by_entry_name_and_stopped_is_unresponsive` | `system_wm` | - | - | - | - |
| `system::app_alias_live_tests::alias_live_files_resolves_by_entry_name_and_stopped_is_unresponsive` | `system_wm` | - | - | - | - |
| `system::app_ops::live_tests::live_desktop_ids_round_trip_through_launch_inventory_selection_and_close` | `system_plain` | - | - | Deadline::after(30_000), deadline() | - |
| `system::capture::capture_live_shadow_tests::grab_live_a_window_capture_leaves_out_the_client_side_shadow` | `capture_x11` | - | - | deadline(20_000) | let mut launched = Launched::new(); |
| `system::capture::capture_live_tests::grab_live_window_pixels_are_the_window_paint` | `capture_x11` | - | - | deadline(20_000) | let mut launched = Launched::new(); |
| `system::capture::capture_live_tests::grab_live_the_window_unit_follows_the_manager` | `capture_x11` | - | - | deadline(20_000) | let mut launched = Launched::new(); |
| `system::capture::capture_live_tests::grab_live_a_full_screen_grab_equals_the_known_paint` | `capture_x11` | - | - | deadline(20_000) | let mut launched = Launched::new(); |
| `system::capture::capture_live_tests::grab_live_a_full_screen_is_refused_under_compositing` | `capture_x11` | - | - | deadline(20_000) | - |
| `system::capture::capture_live_tests::grab_live_a_covered_window_returns_its_own_paint` | `capture_x11` | - | - | deadline(20_000) | let mut launched = Launched::new(); |
| `system::capture::capture_live_tests::grab_live_a_window_partly_off_screen_reports_off_screen` | `capture_x11` | - | - | deadline(20_000) | let mut launched = Launched::new(); |
| `system::capture::capture_live_tests::grab_live_the_capture_permission_follows_the_session` | `capture_x11` | - | - | Deadline::after(10_000) | - |
| `system::capture::capture_live_tests::grab_live_a_chromium_window_is_captured_on_its_client_unit` | `capture_x11` | - | - | deadline(20_000) | let mut launched = Launched::new(); |
| `system::capture::capture_live_tests::grab_live_a_qt_window_is_captured_plain_and_covered` | `capture_x11` | - | - | deadline(20_000) | let mut launched = Launched::new(); |
| `system::capture::capture_live_tests::grab_live_a_stopped_window_is_app_unresponsive` | `capture_x11` | - | - | Duration::from_secs(15), deadline(20_000) | let mut launched = Launched::new(); |
| `system::capture::wayland_portal_live_tests::no_answer_is_a_timeout_that_names_the_dialog_and_leaves_no_file` | `capture_portal` | - | - | Duration::from_secs(20) | let world = boot(); |
| `system::capture::wayland_portal_live_tests::allow_gives_the_desktop_and_leaves_no_file` | `capture_portal` | L86 `std::thread::sleep(Duration::from_millis(400));` | - | Duration::from_millis(400) | let world = boot(); |
| `system::capture::wayland_portal_live_tests::deny_is_permission_denied_and_leaves_no_file` | `capture_portal` | - | - | - | let world = boot(); |
| `system::capture::wayland_portal_live_tests::one_allow_is_remembered_for_the_host_process_and_later_calls_show_no_dialog` | `capture_portal` | L131 `std::thread::sleep(Duration::from_millis(400));` | - | Duration::from_millis(400), Duration::from_secs(5) | let world = boot(); |
| `system::capture::wayland_portal_live_tests::one_deny_is_remembered_and_later_calls_are_refused_at_once` | `capture_portal` | - | - | Duration::from_secs(5) | let world = boot(); |
| `system::capture::wayland_portal_live_tests::each_screen_is_its_rectangle_of_the_portals_image_on_two_monitors` | `capture_portal` | - | - | Deadline::after(30_000) | let world = boot_with(&["1280x800", "800x600"]); |
| `system::capture::wayland_portal_live_tests::a_scaled_monitor_is_cropped_in_image_pixels` | `capture_portal` | L199 `std::thread::sleep(Duration::from_secs(3));` | - | Duration::from_secs(3) | let world = boot_with(&["2560x1600", "800x600"]); |
| `system::capture::wayland_portal_live_tests::the_portal_calls_go_to_the_session_bus_and_not_the_accessibility_bus` | `capture_portal` | - | - | Deadline::after(10_000) | let world = boot(); |
| `system::capture::wayland_portal_live_tests::a_silent_dialog_ends_at_the_portals_limit_and_a_late_allow_leaves_nothing` | `capture_portal` | L313 `std::thread::sleep(Duration::from_millis(400));`; L315 `std::thread::sleep(Duration::from_secs(4));` | - | Duration::from_millis(400), Duration::from_secs(4) | let world = boot(); |
| `system::capture::wayland_portal_live_tests::a_second_request_while_a_dialog_is_open_names_the_open_dialog` | `capture_portal` | - | - | Duration::from_secs(5) | let world = boot(); |
| `system::close_live_tests::close_live_calculator_graceful` | `system_wm` | - | - | deadline() | - |
| `system::close_live_tests::close_live_gtk3_graceful` | `system_wm` | - | - | - | - |
| `system::close_live_tests::close_live_gtk4_graceful` | `system_wm` | - | - | - | - |
| `system::close_live_tests::close_live_qt_graceful` | `system_wm` | - | - | - | - |
| `system::close_live_tests::close_live_chromium_graceful` | `system_wm` | - | - | - | - |
| `system::close_live_tests::close_live_text_editor_two_windows_graceful` | `system_wm` | L120 `std::thread::sleep(Duration::from_millis(200));` | - | Duration::from_millis(200), SETTLE | let second = std::process::Command::new("gnome-text-editor") |
| `system::close_live_tests::close_live_a_stopped_app_is_unresponsive_gracefully_and_killed_by_force` | `system_wm` | L145 `std::thread::sleep(Duration::from_millis(20));` | - | Duration::from_millis(20), Duration::from_secs(5), deadline() | - |
| `system::close_live_tests::close_live_a_child_that_ignores_term_is_killed_after_the_grace` | `system_wm` | - | - | Duration::from_secs(2) | - |
| `system::close_live_tests::close_live_a_plain_term_is_enough_and_kill_is_never_sent_first` | `system_wm` | - | - | - | - |
| `system::close_live_tests::close_live_a_changed_token_leaves_the_live_process_alone` | `system_wm` | - | - | - | - |
| `system::close_live_tests::close_live_protected_decoys_are_refused_and_keep_running` | `system_wm` | - | - | - | - |
| `system::close_live_tests::close_live_text_editor_unsaved_document_times_out_without_a_signal` | `system_wm` | - | - | Duration::from_millis(2_500) | - |
| `system::close_live_tests::close_live_the_private_sessions_own_processes_are_refused` | `system_wm` | - | - | - | - |
| `system::close_live_tests::close_live_a_private_gnome_shell_is_refused_and_keeps_running` | `system_wm` | - | - | - | - |
| `system::cursor_overlay::cli_tests::live_x11_cursor_cli_reports_rendered_and_paints_agent_positions` | `system_plain` | - | - | - | - |
| `system::cursor_overlay::cli_tests::live_wayland_cursor_cli_reports_unrendered_without_x11_fallback` | `system_plain` | - | - | - | - |
| `system::cursor_overlay::tests::live_x11_cursor_adapter_paints_agent_positions_and_retires_resources` | `system_plain` | - | - | Deadline::after(4_000) | - |
| `system::cursor_overlay::tests::live_wayland_cursor_refusal_reports_unrendered_without_an_x11_window` | `system_wm` | - | - | - | - |
| `system::display_live_tests::display_live_lists_the_private_screen_even_with_a_stale_wayland_variable` | `system_plain` | - | - | Deadline::after(10_000) | - |
| `system::focus_activation_live_tests::focus_window_takes_focus_back_from_a_window_another_tool_activated` | `system_wm` | - | - | deadline(8_000) | let mut launched = Launched::new(); |
| `system::focus_window::fallback_live_tests::focus_window_live_f1_never_focused_conditions_are_measured` | `system_wm` | L82 `std::thread::sleep(Duration::from_secs(2));`; L97 `std::thread::sleep(Duration::from_secs(1));`; L112 `std::thread::sleep(Duration::from_secs(1));`; L126 `std::thread::sleep(Duration::from_secs(2));` | - | Duration::from_secs(1), Duration::from_secs(2) | let mut launched = Launched::new();; let mut launched = Launched::new();; let mut launched = Launched::new();; let mut launched = Launched::new(); |
| `system::focus_window::fallback_live_tests::focus_window_live_a_modal_dialog_is_named_and_the_window_is_not_reported_active` | `system_wm` | L149 `std::thread::sleep(Duration::from_secs(2));` | - | Duration::from_secs(2) | let mut launched = Launched::new(); |
| `system::focus_window::fallback_live_tests::focus_window_live_a_stopped_application_is_refused_before_any_request` | `system_wm` | - | - | Duration::from_millis(500) | let mut launched = Launched::new();; Command::new("kill"); let _ = Command::new("kill").args(["-CONT", &pid]).status(); |
| `system::focus_window::fallback_live_tests::focus_window_live_a_window_on_another_desktop_switches_the_desktop` | `system_wm` | L218 `std::thread::sleep(Duration::from_millis(1_500));`; L221 `std::thread::sleep(Duration::from_millis(1_000));` | - | Duration::from_millis(1_000), Duration::from_millis(1_500), Duration::from_secs(12) | let mut launched = Launched::new(); |
| `system::focus_window::fallback_live_tests::focus_window_live_a_zenity_dialog_the_toolkit_never_reports_active_is_focused` | `system_wm` | L245 `std::thread::sleep(Duration::from_millis(200));` | - | Duration::from_millis(200), Duration::from_secs(20) | let mut launched = Launched::new();; let pid = launched.spawn( |
| `system::focus_window::lift_live_tests::focus_window_live_a_minimized_window_ends_as_measured_for_the_manager` | `system_wm` | L244 `std::thread::sleep(Duration::from_millis(100));`; L247 `std::thread::sleep(Duration::from_millis(PAUSE_MS));`; L249 `std::thread::sleep(Duration::from_millis(PAUSE_MS));` | - | Duration::from_millis(100), Duration::from_millis(PAUSE_MS), Duration::from_secs(5) | let mut launched = Launched::new(); |
| `system::focus_overview_live_tests::focus_window_names_an_open_overview_and_succeeds_once_it_is_closed` | `system_wm` | - | - | - | let mut launched = Launched::new();; let pid = launched.spawn("python3", &[&path]); |
| `system::focus_window_live_tests::focus_window_live_activates_a_background_window_per_toolkit` | `system_wm` | - | - | - | - |
| `system::focus_window_live_tests::focus_window_live_stacking_matches_the_root_list_and_sees_an_override_redirect_cover` | `system_wm` | L358 `std::thread::sleep(Duration::from_millis(200));` | - | Duration::from_millis(200), SETTLE | let mut launched = Launched::new();; let cover_pid = launched.spawn("python3", &[&script_path("adf-popup", POPUP)]); |
| `system::key_dispatch_live_tests::key_dispatch_live_an_active_target_gets_the_chord_under_either_policy` | `system_wm` | L201 `std::thread::sleep(Duration::from_secs(1));` | - | Duration::from_secs(1) | let mut launched = Launched::new(); |
| `system::key_dispatch_live_tests::key_dispatch_live_a_background_target_gets_nothing_under_either_policy` | `system_wm` | L242 `std::thread::sleep(Duration::from_millis(500));` | - | Duration::from_millis(500) | let mut launched = Launched::new(); |
| `system::launch_live_tests::live_calculator_launches_by_command_id_path_and_entry_name` | `system_wm` | - | - | - | - |
| `system::launch_live_tests::live_a_second_calculator_attaches_without_a_spawn_and_with_arguments_is_a_new_owner` | `system_wm` | - | - | - | let refused = launch( |
| `system::launch_live_tests::live_text_editor_by_desktop_id_then_a_file_reaches_the_first_instance` | `system_wm` | - | - | - | let again = launch("org.gnome.TextEditor", &with_file).expect("the forwarding launch"); |
| `system::launch_live_tests::live_a_qt_app_launches_by_path_with_the_environment_given` | `system_wm` | - | - | - | let result = launch(&python, &launch_options).expect("the Qt launch"); |
| `system::launch_live_tests::live_chrome_launches_as_chromium_with_a_private_profile` | `system_wm` | - | - | - | - |
| `system::launch_live_tests::live_terminal_is_attributed_to_its_server_and_a_second_launch_adds_a_window` | `system_wm` | - | - | - | - |
| `system::launch_live_tests::live_a_missing_name_is_app_not_found_and_nothing_started` | `system_wm` | - | - | - | let error = launch("no-such-application-xyz", &options(&[])).expect_err("not found"); |
| `system::launch_live_tests::live_an_app_that_exits_after_registering_and_before_a_window_is_not_a_launch` | `system_wm` | - | L332 `"import gi, time\ngi.require_version('Gtk', '3.0')\nfrom gi.repository import Gtk, GLib\nGtk.init()\nend = time.time() + 1.5\nwhile time.time() < end:\n    GLib.MainContext.default().iteration(False)\n    time.sleep(0.05)\n",` | Duration::from_secs(10) | let error = launch("python3", &options(&[&script])).expect_err("a dead pid is not a launch"); |
| `system::launch_live_tests::resident::live_a_running_app_with_no_window_is_started_again_and_a_window_is_reported` | `system_wm` | L43 `std::thread::sleep(Duration::from_millis(200));` | - | Duration::from_millis(200), SETTLE | let first = launch(&script, &resident).expect("the first launch");; let second = launch(&script, &normal).expect("a launch of an app with no window"); |
| `system::launch_live_tests::resident::live_a_forwarded_launch_reports_the_new_window_not_the_old_one` | `system_wm` | - | - | - | let first = launch("nautilus", &options(&[])).expect("the first launch");; let second = launch("nautilus", &options(&["--new-window"])).expect("a forwarded launch"); |
| `system::launch_live_tests::resident::live_a_launch_that_shows_no_window_does_not_wait_for_the_whole_timeout` | `system_wm` | - | - | Duration::from_secs(8) | let result = launch(&script, &options).expect("a launch"); |
| `system::menu_wait_live_tests::menu_wait_live_calculator_main_menu_both_directions` | `system_wm` | - | - | - | - |
| `system::menu_wait_live_tests::menu_wait_live_files_main_menu_both_directions` | `system_wm` | - | - | - | - |
| `system::menu_wait_live_tests::menu_wait_live_terminal_bar_menu_opens_and_closes_by_escape` | `system_wm` | - | - | - | - |
| `system::menu_wait_live_tests::menu_wait_live_terminal_header_popover_opens_and_closes_by_click` | `system_wm` | - | - | - | - |
| `system::menu_wait_live_tests::menu_wait_live_terminal_hidden_bar_times_out_with_the_direction_message` | `system_wm` | - | - | Duration::from_millis(6_000) | - |
| `system::menu_wait_live_tests::menu_wait_live_qt_file_menu_both_directions` | `system_wm` | - | - | - | - |
| `system::menu_wait_live_tests::menu_wait_live_chrome_menu_opens_and_closes_by_escape` | `system_wm` | - | - | - | - |
| `system::menu_wait_live_tests::menu_wait_live_target_exiting_mid_wait_is_stale_ref` | `system_wm` | L293 `std::thread::sleep(Duration::from_millis(1_500));` | - | Duration::from_millis(1_500) | let pid = rig.spawn("gnome-calculator", &[]);; let waiter = scope.spawn(\|\| wait_menu(&rig, &app, true, 20_000)); |
| `system::menu_wait_live_tests::menu_wait_live_stopped_target_is_app_unresponsive_without_the_full_wait` | `system_wm` | - | - | Duration::from_secs(10) | let pid = rig.spawn("gnome-calculator", &[]); |
| `system::process_state_live_tests::process_state_live_calculator_running_unresponsive_running_exited` | `system_wm` | - | - | deadline() | - |
| `system::process_state_live_tests::process_state_live_files_keeps_its_name_across_stop_and_continue` | `system_wm` | - | - | - | - |
| `system::process_state_live_tests::process_state_live_a_blocked_main_loop_is_unresponsive_while_it_sleeps_in_proc` | `system_wm` | L143 `std::thread::sleep(Duration::from_millis(200));` | - | Duration::from_millis(200), Duration::from_secs(20) | - |
| `system::registry_live_tests::registry_live_lists_apps_and_windows_and_keeps_a_stopped_app` | `system_plain` | - | - | Duration::from_secs(4), deadline(10_000) | let mut launched = Launched::new();; let pid = launched.spawn(CALCULATOR, &[]); |
| `system::registry_live_tests::registry_live_names_a_running_process_that_is_not_on_the_bus` | `system_plain` | - | - | deadline(10_000) | let mut launched = Launched::new();; let pid = launched.spawn(&program.to_string_lossy(), &["60"]); |
| `system::registry_live_tests::registry_live_focus_agrees_with_the_window_manager` | `system_plain` | L212 `std::thread::sleep(Duration::from_millis(300));` | - | Duration::from_millis(300), SETTLE, deadline(10_000) | let mut launched = Launched::new();; let calculator = launched.spawn(CALCULATOR, &[]);; let editor = launched.spawn("gnome-text-editor", &[]); |
| `system::sheet_live_tests::sheet_live_gtk4_about_is_a_modal_frame_sheet` | `system_wm` | - | - | - | - |
| `system::sheet_live_tests::sheet_live_gtk4_keyboard_shortcuts_is_a_modal_panel_window` | `system_wm` | - | - | - | - |
| `system::sheet_live_tests::sheet_live_gtk3_about_is_a_showing_dialog_sheet` | `system_wm` | - | - | deadline() | let mut launched = Launched::new();; launched.spawn("gnome-terminal", &[]); |
| `system::signal_inventory::live_tests::signal_inventory_live_a_stopped_target_is_skipped_slow_roots_read_timed_out_and_resolution_names_it` | `system_plain` | L165 `std::thread::sleep(Duration::from_millis(200));`; L178 `std::thread::sleep(Duration::from_millis(200));` | - | Duration::from_millis(200), SETTLE, deadline() | let mut launched = Launched::new();; let pid = launched.spawn("gnome-calculator", &[]); |
| `system::signal_live_surfaces_tests::signal_live_gtk4_about_dialog_is_a_sheet_by_identity` | `system_wm` | - | - | - | - |
| `system::signal_live_surfaces_tests::signal_live_gtk4_keyboard_shortcuts_unnamed_panel_is_a_sheet_by_identity` | `system_wm` | - | - | - | - |
| `system::signal_live_surfaces_tests::signal_live_chrome_menu_is_not_window_opened_and_two_windows_diff_correctly` | `system_wm` | - | - | - | - |
| `system::signal_live_surfaces_tests::signal_live_stopped_target_scoped_wait_answers_app_unresponsive_at_once` | `system_wm` | - | - | Duration::from_secs(3), Duration::from_secs(5) | - |
| `system::signal_live_surfaces_tests::signal_live_window_taken_before_a_stop_resolves_to_app_unresponsive` | `system_wm` | - | - | deadline(10_000) | - |
| `system::signal_live_surfaces_tests::signal_live_unscoped_wait_with_an_unrelated_stopped_app_reports_no_events` | `system_wm` | - | - | Duration::from_millis(7_000), Duration::from_secs(8) | let editor = rig.spawn("gnome-text-editor", &["--new-window"]); |
| `system::signal_live_surfaces_tests::signal_live_slow_but_alive_target_is_incomplete_and_not_unresponsive` | `system_wm` | L292 `std::thread::sleep(Duration::from_millis(800));` | - | Duration::from_millis(3_500), Duration::from_millis(800), Duration::from_millis(8_000) | - |
| `system::signal_live_tests::signal_live_text_editor_new_window_handoff_is_window_opened_without_app_launched` | `system_wm` | - | - | - | let second = rig.spawn("gnome-text-editor", &["--new-window"]); |
| `system::signal_live_tests::signal_live_calculator_launch_and_close_are_app_and_window_events` | `system_wm` | - | - | - | let pid = rig.spawn("gnome-calculator", &[]); |
| `system::signal_live_tests::signal_live_second_calculator_process_is_a_new_app_with_a_distinct_id` | `system_wm` | - | - | - | let second = rig.spawn("gnome-calculator", &[]); |
| `system::signal_live_tests::signal_live_text_editor_ctrl_n_is_window_opened_under_the_same_pid` | `system_wm` | - | - | - | - |
| `system::signal_live_tests::signal_live_sigkilled_calculator_is_window_closed_and_app_terminated` | `system_wm` | - | - | Duration::from_secs(5) | - |
| `system::signal_live_tests::signal_live_helper_with_no_frames_is_not_an_app` | `system_wm` | - | - | - | - |
| `system::signal_live_tests::signal_live_two_windows_focus_changes_under_the_window_manager` | `system_wm` | - | - | deadline(10_000) | - |
| `system::signal_live_tests::signal_live_wayland_without_input_times_out_with_baseline_counts` | `system_wm` | - | - | - | let pid = rig.spawn("gnome-calculator", &[]); |
| `system::signal_live_tests::signal_live_window_that_opens_and_closes_inside_one_wait_is_reported` | `system_wm` | L335 `std::thread::sleep(Duration::from_millis(2_000));` | - | Duration::from_millis(2_000) | let waiter = scope.spawn(\|\| { |
| `system::signal_live_tests::signal_live_batch_seed_before_the_action_is_complete_and_the_close_is_found` | `system_wm` | - | - | - | - |
| `system::stopped_live_tests::stopped_live_calculator_keeps_its_name_and_reports_unresponsive` | `system_wm` | - | - | - | - |
| `system::stopped_live_tests::stopped_live_files_keeps_its_name_and_reports_unresponsive` | `system_wm` | - | - | - | - |
| `system::window_op_live_tests::environment::window_op_live_a_stale_wayland_display_still_proceeds_on_x11` | `system_env` | - | - | - | - |
| `system::window_op_live_tests::environment::window_op_live_a_session_without_a_display_is_refused_before_any_write` | `system_env` | - | - | - | - |
| `system::window_op_live_tests::environment::window_op_live_child_stale_wayland` | `system_env` | - | - | - | let mut launched = Launched::new(); |
| `system::window_op_live_tests::environment::window_op_live_child_no_display` | `system_env` | - | - | - | let mut launched = Launched::new(); |
| `system::window_op_live_tests::environment::window_op_live_an_xwayland_window_is_operated_and_a_native_wayland_window_is_refused` | `system_env` | - | - | - | - |
| `system::window_op_live_tests::environment::window_op_live_child_xwayland` | `system_env` | L326 `std::thread::sleep(Duration::from_millis(1_500));` | - | Duration::from_millis(1_500), deadline(8_000) | let mut launched = Launched::new(); |
| `system::window_op_live_tests::window_op_live_gtk3` | `system_wm` | - | - | - | - |
| `system::window_op_live_tests::window_op_live_gtk4` | `system_wm` | - | - | - | - |
| `system::window_op_live_tests::window_op_live_qt` | `system_wm` | - | - | - | - |
| `system::window_op_live_tests::window_op_live_a_destroyed_window_is_bad_window_and_not_success` | `system_wm` | - | - | deadline(5_000) | let mut launched = Launched::new(); |
| `system::window_op_live_tests::window_op_live_a_restore_of_a_minimized_window_works_every_time` | `system_wm` | L362 `std::thread::sleep(Duration::from_millis(1_500));` | - | Duration::from_millis(1_500) | let mut launched = Launched::new(); |
| `system::x11::grab::live_tests::grab_live_a_flat_window_reads_back_its_fill_and_matches_an_independent_capture` | `x11_capture` | - | - | deadline() | - |
| `system::x11::grab::live_tests::grab_live_describes_the_window_and_names_each_refusal` | `x11_capture` | - | - | deadline() | - |
| `system::x11::grab::live_tests::grab_live_the_root_grab_and_the_compositing_read_answer_on_the_private_screen` | `x11_capture` | - | - | deadline() | - |
| `system::x11::grab::live_tests::grab_live_the_root_is_drawable_zero_in_root_coordinates` | `x11_capture` | - | - | deadline() | - |
| `system::x11::grab::live_tests::grab_live_a_framed_window_has_a_parent_and_every_rect_is_in_root_coordinates` | `x11_capture` | L300 `std::thread::sleep(Duration::from_millis(100));` | - | Duration::from_millis(100), Duration::from_secs(10), deadline() | - |
| `system::x11::grab::live_tests::grab_live_a_covered_window_is_blank_directly_and_painted_through_the_redirect` | `x11_capture` | L339 `std::thread::sleep(Duration::from_millis(300));` | - | Duration::from_millis(300), deadline() | - |
| `tree::absence_live_tests::absence_live_a_killed_app_is_a_settled_stale_ref_in_one_pass_for_a_ref_and_a_held_handle` | `tree_live_app` | - | - | Duration::from_millis(1_500) | - |
| `tree::absence_live_tests::absence_live_a_stopped_app_is_unresponsive_within_the_budget_and_never_stale` | `tree_live_app` | - | - | Duration::from_secs(8), deadline(2_000) | - |
| `tree::absence_live_tests::absence_live_a_removed_element_is_a_complete_miss_and_a_held_handle_is_recorded_per_toolkit` | `tree_live_app` | - | - | Duration::from_millis(1_500), Duration::from_secs(5), Duration::from_secs(if label == "gtk3" { 8 } else { 25 }) | - |
| `tree::absence_live_tests::absence_live_a_gtk4_page_that_leaves_the_tree_is_stale_and_returns_at_the_same_path` | `tree_live_app` | - | - | Duration::from_secs(5) | - |
| `tree::capability_live_tests::capability_live_every_token_is_backed_by_an_interface_or_an_action` | `tree_live_app` | - | - | - | - |
| `tree::chromium_live_tests::chromium_live_a_killed_vscode_is_a_settled_stale_ref_in_one_pass` | `tree_live_app` | - | - | Duration::from_millis(1_500), deadline(4_000) | let (pid, _, _) = launch(&mut rig); |
| `tree::chromium_live_tests::chromium_live_opening_a_second_file_shifts_no_web_content_path` | `tree_live_app` | L98 `std::thread::sleep(Duration::from_secs(3));` | - | Duration::from_secs(3), Duration::from_secs(60) | let (pid, program, data) = launch(&mut rig);; rig.launched.spawn(&program, &arguments); |
| `tree::find_live_tests::find_live_returns_matches_and_refs_that_resolve_back_to_their_nodes` | `tree_live_app` | - | - | - | - |
| `tree::hit_live_tests::cases::hit_live_gtk3_terminal_cover_in_both_directions` | `tree_live_app` | - | - | - | - |
| `tree::hit_live_tests::cases::hit_live_qt_cover_in_both_directions` | `tree_live_app` | - | - | - | - |
| `tree::hit_live_tests::cases::hit_live_gtk4_calculator_at_two_positions` | `tree_live_app` | - | - | - | - |
| `tree::hit_live_tests::cases::hit_live_gtk4_files_at_two_positions` | `tree_live_app` | - | - | - | - |
| `tree::hit_live_tests::cases::hit_live_vscode_is_never_a_wrong_verdict` | `tree_live_app` | - | - | - | - |
| `tree::hit_live_tests::cases::hit_live_chrome_cover_in_both_directions` | `tree_live_app` | - | - | - | - |
| `tree::hit_live_tests::cases::hit_live_files_zero_axis_group_is_unknown` | `tree_live_app` | - | - | deadline(10_000) | - |
| `tree::live_horizontal_live_tests::strip_live_gtk3_horizontal_bar_scrolls_a_far_right_item_into_view` | `tree_live_app` | - | - | - | - |
| `tree::live_horizontal_live_tests::strip_live_gtk4_horizontal_bar_scrolls_a_far_right_item_into_view` | `tree_live_app` | - | - | - | - |
| `tree::live_offscreen_live_tests::offscreen_live_gtk3_scrolled_list_realises_an_offscreen_row` | `tree_live_app` | - | - | - | - |
| `tree::live_offscreen_live_tests::offscreen_live_gtk4_scrolled_list_realises_an_offscreen_row` | `tree_live_app` | - | - | - | - |
| `tree::live_offscreen_live_tests::offscreen_live_qt_list_item_cannot_be_scrolled_to` | `tree_live_app` | - | - | - | - |
| `tree::live_offscreen_live_tests::offscreen_live_files_grid_realises_an_offscreen_row` | `tree_live_app` | - | - | - | - |
| `tree::live_offscreen_live_tests::offscreen_live_terminal_survey_agrees_with_the_oracle` | `tree_live_app` | - | - | - | - |
| `tree::live_offscreen_live_tests::offscreen_live_chrome_page_realises_an_offscreen_button` | `tree_live_app` | - | - | - | - |
| `tree::live_offscreen_live_tests::offscreen_live_vscode_scroll_rungs_are_measured` | `tree_live_app` | - | - | - | - |
| `tree::live_read_live_tests::live_read_every_projection_agrees_with_a_second_read_on_each_toolkit` | `tree_live_app` | - | - | - | - |
| `tree::live_read_live_tests::live_read_a_calculator_display_follows_a_button_press_made_by_the_harness` | `tree_live_app` | - | - | Duration::from_secs(5), deadline(5_000) | let pid = rig.launched.spawn("gnome-calculator", &[]); |
| `tree::live_read_live_tests::live_read_a_password_value_is_withheld_on_gtk3_gtk4_and_qt` | `tree_live_app` | - | - | - | - |
| `tree::live_timeout::walker_live_stopped_app_returns_a_partial_walk_at_the_deadline` | `tree_live_app` | - | - | Deadline::after(DEADLINE_MS), Duration::from_millis(200), Duration::from_millis(DEADLINE_MS) | let mut launched = Launched::new();; let pid = launched.spawn("gnome-calculator", &[]); |
| `tree::navigate_live_tests::navigate_live_a_settings_panel_ref_is_stale_after_another_panel_replaces_it` | `tree_live_app` | - | - | Duration::from_secs(8) | let pid = rig.launched.spawn("gnome-control-center", &["network"]);; rig.launched.spawn("gnome-control-center", &["sound"]); |
| `tree::navigate_live_tests::navigate_live_every_ref_of_a_relaunched_app_is_stale_through_the_gate` | `tree_live_app` | - | - | deadline(5_000) | let first = rig.launched.spawn("gnome-calculator", &[]);; let second = rig.launched.spawn("gnome-calculator", &[]); |
| `tree::navigate_live_tests::navigate_live_closing_one_files_window_leaves_its_refs_stale_and_the_other_window_resolving` | `tree_live_app` | - | - | Duration::from_secs(10), Duration::from_secs(40), deadline(5_000) | let first = rig.launched.spawn("nautilus", &["--new-window", &path]);; rig.launched.spawn("nautilus", &["--new-window", &path]); |
| `tree::observe_live_tests::observe_live_calculator_returns_a_complete_reffed_tree` | `tree_live_app` | - | - | Deadline::after(10_000) | let mut launched = Launched::new();; let pid = launched.spawn("gnome-calculator", &[]); |
| `tree::perf_live_tests::perf_live_costs_min_of_seven_with_the_warm_up_discarded` | `tree_perf` | - | - | - | - |
| `tree::physical_live_apps_tests::live_text_editor_receives_typed_text_with_remapped_characters` | `tree_physical` | - | - | - | - |
| `tree::physical_live_apps_tests::live_calculator_evaluates_typed_keys` | `tree_physical` | - | - | - | - |
| `tree::physical_live_apps_tests::live_the_wheel_scrolls_the_files_list` | `tree_physical` | - | - | - | - |
| `tree::physical_live_apps_tests::live_qt_headless_reveal_has_no_container_while_the_headed_wheel_moves_the_content` | `tree_physical` | L109 `std::thread::sleep(Duration::from_millis(800));` | - | Duration::from_millis(800) | - |
| `tree::physical_live_apps_tests::live_the_wheel_scrolls_a_chromium_page` | `tree_physical` | - | - | - | - |
| `tree::physical_live_mutter_tests::live_a_raw_click_activates_a_window_only_once_the_overview_is_closed` | `tree_physical` | L68 `std::thread::sleep(Duration::from_millis(200));` | - | Duration::from_millis(200) | let mut child = Command::new(std::env::current_exe().expect("exe")); .spawn(); let shell = X11Shell::start(); |
| `tree::physical_live_tests::live_type_into_a_gtk3_entry_reads_back_with_remapped_characters` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_type_into_a_gtk4_entry_reads_back_with_remapped_characters` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_type_into_a_qt_entry_reads_back_with_remapped_characters` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_key_combos_select_and_delete_in_a_gtk3_entry` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_key_combos_select_and_delete_in_a_gtk4_entry` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_key_combos_select_and_delete_in_a_qt_entry` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_a_click_at_the_bounds_centre_presses_a_gtk3_button` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_a_click_at_the_bounds_centre_presses_a_gtk4_button` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_a_click_at_the_bounds_centre_presses_a_qt_button` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_the_wheel_scrolls_a_gtk3_list_and_back` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_the_wheel_scrolls_a_gtk4_list_and_back` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_the_wheel_scrolls_a_qt_list_and_back` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_a_drag_presses_moves_and_releases_the_button` | `tree_physical` | - | - | - | - |
| `tree::physical_live_tests::live_a_frame_that_is_not_active_refuses_with_window_active` | `tree_physical` | L252 `std::thread::sleep(Duration::from_secs(1));`; L260 `std::thread::sleep(Duration::from_millis(500));` | - | Duration::from_millis(500), Duration::from_secs(1) | let mut launched = Launched::new(); |
| `tree::physical_live_tests::live_keys_under_a_menu_grab_are_reported_unverified` | `tree_physical` | L275 `std::thread::sleep(Duration::from_millis(1200));`; L277 `std::thread::sleep(Duration::from_millis(800));`; L301 `std::thread::sleep(Duration::from_millis(500));` | - | Duration::from_millis(1200), Duration::from_millis(500), Duration::from_millis(800), Duration::from_secs(15), Duration::from_secs(5) | let mut launched = Launched::new(); |
| `tree::qt_live_tests::qt_live_twins_resolve_to_their_own_button_after_the_window_moves` | `tree_live_app` | - | - | Duration::from_secs(8) | - |
| `tree::qt_live_tests::qt_live_rows_after_a_resort_resolve_to_their_own_text_or_are_refused_never_swapped` | `tree_live_app` | L103 `std::thread::sleep(Duration::from_millis(600));` | - | Duration::from_millis(600) | - |
| `tree::qt_live_tests::qt_live_a_process_with_two_bus_connections_resolves_through_the_window_owning_one` | `tree_live_app` | - | - | deadline(5_000) | - |
| `tree::registry_toolkit_live_tests::registry_toolkit_live_reads_gtk3_and_qt_that_refuse_the_group_read` | `tree_live_app` | - | - | - | - |
| `tree::resolve_source::action_live_tests::live_a_fresh_files_click_changes_native_window_state` | `tree_live_app` | - | - | Duration::from_secs(20), Duration::from_secs(3) | let mut launched = Launched::new();; let pid = launched.spawn("nautilus", &["--new-window", &private]); |
| `tree::resolve_live_tests::resolve_live_every_snapshot_ref_resolves_to_the_node_it_was_taken_from` | `tree_live_app` | - | - | - | - |
| `tree::resolve_twin_live_tests::resolve_live_files_view_options_nodes_each_resolve_to_their_own_node` | `tree_live_app` | - | - | - | let mut launched = Launched::new();; let pid = launched.spawn("nautilus", &["--new-window"]); |
| `tree::resolve_twin_live_tests::resolve_live_twin_buttons_are_told_apart_by_path_and_by_stored_size_and_stay_ambiguous_without` | `tree_live_app` | - | - | Deadline::after(5_000) | let mut launched = Launched::new(); |
| `tree::resolve_twin_live_tests::resolve_live_files_row_refs_survive_a_sibling_row_leaving` | `tree_live_app` | - | - | Deadline::after(5_000), Duration::from_secs(2), Duration::from_secs(20) | let mut launched = Launched::new();; let pid = launched.spawn("nautilus", &["--new-window", &folder.to_string_lossy()]); |
| `tree::vocabulary_live_tests::vocabulary_live_real_apps` | `tree_live_app` | - | - | - | - |
| `tree::walker_live_tests::walker_live_real_apps` | `tree_live_app` | - | - | - | - |

### Shared cost profiles

- `capture_x11`: One private X11 session per test binary invocation (external harness); each test creates an adapter/bus and launches its fixture window(s) through shared capture support. Coverage remains independent per capture/refusal case.
- `capture_portal`: Each test calls boot()/boot_with(): starts a fresh private headless GNOME Shell and portal; shared support uses fixed 3s/1s/2s startup/dialog waits plus 1.5s/3s portal settle paths. Serial consent/session state.
- `clipboard_x11`: Private X11 session per invocation; each test calls private_session(). Binary/helper build is process-wide Once; owner/CLI/reader subprocesses are per test. Shared wait_owner polls every 50ms; cleanup retires owner and record.
- `clipboard_wayland`: Each test calls boot() and starts a private Wayland GNOME Shell; fixed boot waits in live_shell (400ms/200ms/3s/2s) and per-test clipboard/copy/paste subprocesses.
- `wayland_remote`: Private Wayland compositor per ignored test through Shell::start; test starts RemoteDesktop session(s) via open/create/start and launches GTK fixture for key delivery.
- `x11_input`: Private X11 + Openbox per invocation; each typing case launches a GTK3/GTK4/Qt fixture and polls for readiness. Held-key/remap cases spawn a nested test-harness child, intentionally hold 60s, kill it, then verify next-call release/restoration.
- `tree_physical`: Private X11 + Openbox per invocation; physical_live_rig creates a fresh toolkit Scene per test, with 150ms/100ms helper polls; performs real input and independently reads state.
- `tree_live_app`: Private X11 session per invocation; each test launches one or more real toolkit/browser apps via Launched or helper launch; walker settle helper polls every 100ms, typically under a 40s bound.
- `tree_perf`: Private X11 session; launches/attaches measured app(s), discards warm-up and takes seven samples by design. The baseline benchmark was retired; retained functional owners are named in the gate runbook.
- `system_wm`: Private X11 + Openbox per invocation; fixture apps/helpers are launched per case. Test-local wait loops and close/focus/menu/signal deadlines apply; Launched cleanup kills/reaps owned process groups and session-owned activated terminal servers.
- `system_plain`: Private X11 session per invocation; each test launches/queries app(s) as declared in its body, with bounded registry/readiness polling. No Openbox unless ignore reason says --wm.
- `system_env`: Private X11 session per invocation; case exercises display-environment routing/refusal, may launch nested child test process or app; preserve refusal-before-write assertions.
- `x11_capture`: Private X11 session per invocation, no WM necessarily; uses X11 drawable/capture fixtures and independent pixel oracle. Preserve compositing, covered-window, root-coordinate, and refusal cases.
- `actions_live`: Private X11 session per invocation; action fixture tests start per-test GTK/Qt/GNOME/browser widgets through Launched/Bench. Bench.wait_for and related waits poll the exact value/state; fixed sleeps are called out per row.


# Linux ignored live tests: shared-helper launch and wait accounting

Scope: static source tracing for the original 247-test baseline at `b66beeb5`, using the current Rust sources and the qualified names recorded in `docs/runbooks/linux-live-costs.md`. This appendix accounts for shared fixture entrypoints that hide process launches and readiness waits. Counts are launches issued by the test/helper path, not guaranteed new OS application instances: D-Bus activation, app reuse/forwarding, early exit, and unavailable prerequisites are branch dependent. A private-session wrapper is one session start per test binary invocation (not per test); Wayland shell boot helpers may create their own nested compositor/session resources.

## Shared fixture entrypoints

| Entry point | Proven launch/session count | Waits and deadline path |
|---|---|---|
| `tree::live_apps::each_app` | Exactly 7 launch attempts per call, sequentially: Nautilus (`--new-window`), gnome-calculator, gnome-control-center, gnome-text-editor (`--new-window`), gnome-terminal (`--wait`, server/window), Qt fixture via `live_qt::launch`, VS Code with fresh user-data dir. The helper eagerly builds all 7 result entries. New process count is 0–7 depending on app activation/reuse/availability; terminal is server-owned and may involve D-Bus activation. | Each app case gets `settle` (complete tree, node count threshold, 3 stable observations), 100 ms poll, 40 s overall bound, 5 s walk deadline per attempt. App labels/min nodes are in `live_apps.rs`. VS Code scratch profile removed after its call; each app's `Launched` drop kills/reaps children and session-owned terminal server. |
| `actions::live_bench::each_toolkit` / `Bench::open` | Exactly 1 Python fixture launch per toolkit iteration: GTK3, GTK4, Qt. Thus 3 per `each_toolkit` call. | `wait_for_frame`: up to 20 s, enumerates windows and AT-SPI frame on each poll; sleeps 250 ms; inner list/bus calls use 5 s deadlines. `Bench::named` can poll every 250 ms for up to 10 s. Other action/read calls typically use 5 s, tree walk 10 s, interaction lease 30 s. |
| `actions::live_bench::each_scripts` | Exactly 1 fixture launch for each toolkit supplied by that test: GTK3/GTK4/Qt scripts are Python fixtures; Qt starts under configured `qtvenv`. `ALL` means 3; `[Qt]` means 1; `[Gtk3, Qt]` means 2. | Same `Bench::open_scripts` / `wait_for_frame` bounds as above. `Bench::attach` creates no process; it adopts the caller's launched fixture. |
| `actions::live_write_fixture::launch` | Exactly 1 Python process per call; toolkit selects GTK3, GTK4, or Qt script. It creates one scratch directory and owns one `Launched`. | `wait_until` is a 150 ms polling loop bounded by caller's 5 s `SETTLE`. Most writes also use 4–5 s operation deadlines. Test functions loop over all 3 toolkits, so exactly 3 fixture launches per such test (9 tests in this module; 27 launch attempts total for the module). |
| `actions::live_click_tests::open` | Exactly 1 process launch per call, with program/args passed by each case. | Waits until the target gets an AT-SPI frame; 250 ms polling, 30 s maximum; then fixed 2 s settle. This is a helper-local fixed sleep. |
| `tree::live_rig::rig` | Zero app launches. It creates `LinuxAdapter`, `SharedBus`, and an empty `Launched` owner. Test code chooses whether/what to spawn; helper does not cache an app or session. | `settled` uses `walker_live_tests::settle`: 100 ms poll, complete stable tree for 3 polls, 40 s bound, 5 s tree-walk deadline. `wait_until` polls every 100 ms for caller's exact bound. `resolve` and raw reads use 5 s. |
| `tree::physical_live_rig::Scene::open` | Exactly 1 GTK or Qt fixture process per call. `Scene::attach` is zero-launch adoption of an already launched pid. | `private_display` validates selected X11 display; `settle` as above (40 s/100 ms, 5 s per walk); target resolve 10 s; input lease 30 s. Helpers `settled_y` poll at 150 ms and 100 ms (source lines in `physical_live_rig.rs`). |
| `system::close_live::Orphan::spawn` / `script_app` | Exactly 1 detached process per `Orphan::spawn` or script-app call. Direct tests may create multiple orphans (decoy arrays, Chromium helper, shell retry). `gtk3`, `gtk4`, `qt`, and `chromium` each create one. A `sleep 120` fixture is still one child process, not a GUI app. | `Orphan::spawn` polls process facts every 20 ms, max 5 s; `wait_for_window` polls X11 every 200 ms, max 40 s, each control request 3 s; ignored-signal poll 10 ms/max 5 s; proc disappearance poll 20 ms/max 5 s. `listed` has its own 40 s bound. Close-operation deadlines are test supplied; do not sum sequential alternatives as one runtime. |
| `system::signal_live_apps::Rig::new` | Zero app launches; opens one adapter/accessibility connection, owns launched processes and adopted pids. `Rig::spawn` is exactly one direct child launch. Helpers `launch_qt`, `launch_chrome`, `terminal_server` each issue one launch attempt; terminal result can be D-Bus server pid rather than the client pid. | `poll` is 100 ms, 40 s bound. Inventory/deadline calls commonly 10 s. Some helper app scripts intentionally sleep (fixture process only, not host test thread). |
| `system::menu_wait_live_tests` | `Rig::new` zero-launch. The calculator/files cases issue one `Rig::spawn`; terminal cases call `terminal_server` once (one gnome-terminal attempt, then poll for server); Qt calls `launch_qt` once; Chrome calls `launch_chrome` once. | App/window/menu-node readiness uses `poll` (100 ms, 40 s). `both_directions` performs two state transitions, each waiter allows 20 s, plus one fixed 1 s “must still be pending” synchronization sleep per transition (2 s/test for the bidirectional cases). Tests with hidden-bar timeout use 3 s API timeout and assert under 6 s; stopped target requests 25 s but must return under 10 s; exit-mid-wait uses 20 s request and fixed 1.5 s before kill. Chrome/terminal teardown helpers can add repeated bounded polls and 500 ms sleeps when dismissing dialogs. |
| `system::launch_live_tests::launch` / `launched` | These are product launch API calls, not raw helper spawns. Counts are the call count in the case; whether the call spawns, forwards to an existing process, or is refused is precisely what each case measures. Calculator second launch, editor file launch, terminal `--window`, and resident app launches intentionally test forwarding/new-owner branches. | `wait_until` uses polling with 40 s bound; registry/window/process predicates have 100 ms sleeps. Adapter launch lease 30 s; inventory 10 s. Early-exit script itself runs 1.5 s and polls its event loop every 50 ms; assertion requires API to notice exit within 10 s. |
| `system::capture::wayland_portal_live_support::boot[_with]` | One `boot` call per portal test; starts a fresh headless GNOME Shell attempt. Successful attempt: 1 shell + 1 GNOME portal backend + 1 portal frontend + 1 GTK painter = 4 child launch attempts, plus one RemoteDesktop `CreateSession`/`Start` for keyboard control (not an app process). Retries are bounded at 8. Earlier failure points yield fewer children; partially started groups are dropped. Therefore launch-attempt bounds per boot: shell 1–8; backend 0–8; frontend 0–8; painter 0–1; total app/service launches 1–25, successful first attempt = 4. `boot_with` changes monitor args only. | Shell socket wait 25 s per attempt with 200 ms polls, then fixed 3 s settle. Keyboard session open bounded by 8 s; `open_keys` fixed 1 s. Portal owner checks each up to 30 s, 300 ms poll. `start_portal`: fixed 1 s after killing stale portal owners, starts backend then frontend, fixed 2 s settle. `boot_with` after painter: fixed 1.5 s Escape/dialog settle + 3 s second settle; up to 8 boot attempts. The call-path exposes why the portal cases have high and variance-heavy cold-start cost. |
| `input::wayland::live_shell::Shell::start` | One private shell process per start attempt, retried internally up to 8. Each shell health check opens up to 5 RemoteDesktop sessions (open/create/start/stop), returning early on first failure; a healthy shell therefore opens exactly 5. Actual shell starts 1–8. Each successful test later drops the shell and adds 2 s teardown; each unhealthy started shell also drops and adds 2 s. GTK4 input case separately launches one GTK client; session-bus routing case creates/starts/stops a RemoteDesktop session but no GTK app. | Open/create/start attempts use caller `Deadline`; shell readiness includes 400/200 ms polls, fixed 3 s and 2 s sleeps. Session startup should be counted as retryable compositor/session work, not multiplied as application launches. |
| `input::clipboard::set_clear_live_support` | Private session guard creates no process. `built_helper` and `built_cli` build once per test process through `Once`; each set operation may start/replace a detached owner, and foreign-reader/taker/CLI calls are additional subprocesses as written by each case. | Owner readiness polls every 50 ms under request deadline. Cleanup waits/terminates owner and removes record. Count direct test commands plus owner helper; do not count binary builds once per test. |

## Test-to-entrypoint call table

| Ignored test(s) | Fixture path and app launch count |
|---|---|
| `tree::capability_live_tests::capability_live_every_token_is_backed_by_an_interface_or_an_action`; `tree::live_read_live_tests::live_read_every_projection_agrees_with_a_second_read_on_each_toolkit`; `tree::find_live_tests::find_live_returns_matches_and_refs_that_resolve_back_to_their_nodes`; `tree::resolve_live_tests::resolve_live_every_snapshot_ref_resolves_to_the_node_it_was_taken_from` | Each calls `each_app` once: exactly 7 launch attempts per test (the seven named apps above), sequentially. |
| `actions::live_text_tests::{set_value_and_clear_land_on_a_text_field_in_every_toolkit,type_inserts_non_ascii_text_into_an_empty_field_exactly,type_after_a_wide_character_lands_at_the_caret_exactly,a_password_field_write_lands_and_the_result_never_carries_the_text}` | Each calls `each_toolkit`: exactly 3 launches (GTK3, GTK4, Qt). |
| `actions::live_value_tests::{a_spin_button_takes_a_whole_number_and_refuses_a_fraction_in_every_toolkit,a_slider_is_clamped_and_the_stored_value_is_not_reported_as_verified,a_scroll_bar_write_moves_the_content}` | Each calls `each_toolkit`: exactly 3 launches. |
| `actions::live_widget_tests::a_click_runs_the_handler_of_a_button_in_every_toolkit`; `toggle_check_and_uncheck_move_a_check_box_and_never_write_twice` | `each_scripts(ALL)`: exactly 3 launches per test (GTK3, GTK4, Qt). |
| `actions::live_widget_tests::select_by_display_value_chooses_the_item_of_a_combo_box` | `each_scripts([Gtk3, Qt])`: exactly 2 launches. |
| `actions::live_widget_tests::select_by_display_value_chooses_the_item_of_a_qt_list`; `a_headed_set_focus_moves_the_focus_to_a_qt_button` | `each_scripts([Qt])`: exactly 1 launch each. |
| `actions::live_widget_tests::the_selection_reads_in_utf16_and_the_caret_in_code_points` | `each_scripts([Gtk3, Gtk4])`: exactly 2 launches. |
| `actions::live_write_tests::write_live_*` (all 9 tests) | Each loops over 3 toolkit labels and calls fixture `launch` once per label: exactly 3 launch attempts/test; GTK3, GTK4, Qt. Total 27. Two test cases add a direct 600 ms fixed sleep per toolkit, so 1.8 s each. |
| `actions::live_click_tests` (all 5) | Each calls `open` once: one app launch attempt/test. `open` adds a 2 s settle after frame discovery; readiness bound 30 s and 250 ms poll. |
| `tree::physical_live_tests` (15), `physical_live_apps_tests` (5), `physical_live_mutter_tests` (1) | Each `Scene::open` is exactly one fixture process. `Scene::attach` adopts a process already spawned by the test; count those explicit prior `Launched::spawn` calls separately. The `physical_live_apps` helpers `attach` are zero-launch themselves. `physical_live_mutter` starts one helper WM/shell process with branch-dependent startup retries plus one GTK Scene. |
| `system::close_live_tests` (14) and `system::process_state_live_tests` (3) | Each `Orphan::spawn` is one process launch. GUI fixture wrappers `gtk3`, `gtk4`, `qt`, `chromium` each mean one launch. Script-backed GUI fixtures add no second app process beyond the one Python/Qt/Chrome launch. Protected-decoy test launches one process per configured decoy in a fixed array (read exact array at `close_live_tests.rs`); private-shell fixture tries at most three shell starts. Process-state helper has one Orphan each. |
| `system::menu_wait_live_tests` (9) | One app launch attempt each: calculator, Nautilus, gnome-terminal server, Qt fixture, Chrome, calculator (exit-mid-wait), calculator (stopped). All tests construct `Rig`; `Rig::new` itself launches none. The Files menu case additionally makes a scratch directory, not another app. |
| `system::signal_live_tests` (10), `signal_live_surfaces_tests` (7), `signal_inventory::live_tests` (1) | `Rig::new` launches none. `Rig::spawn`/`launch_*` helper count is the per-test direct spawn count; app-launch variants include Qt, Chrome and terminal-server helpers. Cases that test unscoped/stop/slow targets may launch 0/1/2 processes according to their body; nested waiter `scope.spawn` is a thread, not an app. See per-test `Rig::spawn`, `launch_qt`, `launch_chrome`, and `terminal_server` calls in these three files. This is branch-dependent app start count, not one launch per `Rig`. |
| `system::launch_live_tests` (8) and nested `resident` (3) | Counts are exact API attempts from the body: calculator alias loop 4 (each followed by kill); second-calculator case 4 (first + attach + refused no-spawn + argumented second); editor handoff 2; Qt 1; Chrome 1; terminal 2; missing app 1 (no process); early-exit Python 1. Resident tests: resident-then-normal 2; forwarded Nautilus 2; no-window case 2 rounds / 2 attempts. Attempts can forward, refuse, or spawn; those actual process outcomes are explicitly asserted. |
| `system::capture::capture_live_tests` (10) and `capture_live_shadow_tests` (1) | Each paint fixture call starts one process; covered-window cases start two (target + cover); Qt plain/covered likewise starts two. Permission/compositing refusal tests can start zero. Use `capture_live_support::{paint,paint_qt,chrome}` call sites per function for exact case count. Capture operation deadline is 20 s; wait for app/client window uses bounded polling. |
| `system::capture::wayland_portal_live_tests` (10) | Exactly one `boot`/`boot_with` call per test. All therefore use the retry envelope above. Number of capture requests after boot is test-dependent: remembered allow makes 1 prompted + 2 quick; remembered deny 1 prompted + 1 quick; overlapping-dialog case has 2 concurrent requests; multi-monitor cases make full-screen + screen 0 + screen 1 requests. `capture_while` adds a 4.5 s fixed dialog-visible sleep/request. |

## Bounds and interpretation

- These are structural launch counts and explicit source deadlines, not elapsed runtime. An async/scope waiter is not an app launch. Shell commands, dbus activation, process adoption, and retries are counted separately where source makes that distinction.
- A timeout such as `settle(..., 40s)` bounds one helper call; repeated calls in a test can occur sequentially. API deadlines passed concurrently with helper waits do not necessarily add linearly.
- Profile table in `docs/runbooks/linux-live-costs.md` remains the accounting baseline. This appendix clarifies opaque fixture edges so a future optimization can change setup reuse while preserving each test's independent wrong-element, delivery, refusal, cleanup, routing, clipboard, and capture assertion.


## Case-level fixture map for opaque helper calls

### Close and process-state tests

The process fixture count below includes one process per helper constructor/Orphan call. The product close path can spawn no new process or signal that fixture; this table does not count helper shell commands (`sh`, `kill`, `xdotool`) as app fixtures.

| Test | Fixture launches and target |
|---|---|
| `system::close_live_tests::close_live_calculator_graceful` | 1 `gnome-calculator` Orphan. |
| `...::close_live_gtk3_graceful` / `close_live_gtk4_graceful` / `close_live_qt_graceful` / `close_live_chromium_graceful` | 1 each: GTK3 Python, GTK4 Python, Qt Python, Chrome respectively, via wrapper helper. |
| `...::close_live_text_editor_two_windows_graceful` | 1 orphan `gnome-text-editor` + 1 second `gnome-text-editor --new-window` launcher invocation, expected to forward to same process; count 2 launch attempts, 1 expected app process. |
| `...::close_live_a_stopped_app_is_unresponsive_gracefully_and_killed_by_force` | 1 `gnome-calculator`. |
| `...::close_live_a_child_that_ignores_term_is_killed_after_the_grace` | 1 `sh -c` child with TERM trap and nested `sleep 1` loop; one owned process group. |
| `...::close_live_a_plain_term_is_enough_and_kill_is_never_sent_first` / `close_live_a_changed_token_leaves_the_live_process_alone` | 1 `sleep 120` each. |
| `...::close_live_protected_decoys_are_refused_and_keep_running` | 23 `sleep 120`-equivalent copied executables (one per protected-name row), sequential; source constant has 23 rows. Each is tested against 4 metadata/force combinations. |
| `...::close_live_text_editor_unsaved_document_times_out_without_a_signal` | 1 `gnome-text-editor` with file path; helper types text, then fixed 1 s delay. |
| `...::close_live_the_private_sessions_own_processes_are_refused` | 0 fixture apps; enumerates private-session members. |
| `...::close_live_a_private_gnome_shell_is_refused_and_keeps_running` | 1–3 `gnome-shell --x11 --replace` attempts, 30 s WM-owner poll each; failed attempt is dropped before retry. |
| `system::process_state_live_tests::process_state_live_calculator_running_unresponsive_running_exited` | 1 `gnome-calculator` Orphan. |
| `...::process_state_live_files_keeps_its_name_across_stop_and_continue` | 1 Nautilus Orphan (`--new-window`). |
| `...::process_state_live_a_blocked_main_loop_is_unresponsive_while_it_sleeps_in_proc` | 1 Python/GTK child; fixture's GLib callback blocks its UI thread in a 120 s sleep after flag creation. |

Common bounds: process identity acquisition 5 s / 20 ms polling; `listed` up to 40 s with 10 s inventory/windows calls and 200 ms polling; window readiness 40 s with 3 s control calls and 200 ms polling; close lease budgets are explicitly 4, 5, 10, 15, or 20 s at each call site. Test-only measured contract can be shorter (e.g. unsaved dialog under 2.5 s). The 200 ms / 20 ms helper sleeps are polling, not fixed setup delays; the unsaved-editor's 1 s and Orphan `type_text` 500 ms are fixed.

### Signal event and surface suites

`Rig::new` is zero app launches. `Rig::spawn`, `launch_qt`, `launch_chrome`, and `terminal_server` each issue one app launch attempt. These cases use a fresh `Rig`, so there is no cross-test fixture reuse.

| Test | Per-test app launch attempts |
|---|---|
| `system::signal_live_tests::signal_live_text_editor_new_window_handoff_is_window_opened_without_app_launched` | 2: editor fixture + second editor `--new-window` forwarding attempt; expected same app pid. |
| `...::signal_live_calculator_launch_and_close_are_app_and_window_events` | 1 calculator. |
| `...::signal_live_second_calculator_process_is_a_new_app_with_a_distinct_id` | 2 calculators; second must be a distinct pid. |
| `...::signal_live_text_editor_ctrl_n_is_window_opened_under_the_same_pid` / `signal_live_sigkilled_calculator_is_window_closed_and_app_terminated` | 1 editor / 1 calculator respectively. |
| `...::signal_live_helper_with_no_frames_is_not_an_app` | 1 gnome-terminal launch attempt; observes server pid after the client/helper handoff. |
| `...::signal_live_two_windows_focus_changes_under_the_window_manager` | 1 editor + 1 calculator. |
| `...::signal_live_wayland_without_input_times_out_with_baseline_counts` | 1 calculator. |
| `...::signal_live_window_that_opens_and_closes_inside_one_wait_is_reported` / `signal_live_batch_seed_before_the_action_is_complete_and_the_close_is_found` | 1 editor each. |
| `system::signal_live_surfaces_tests::signal_live_gtk4_about_dialog_is_a_sheet_by_identity` / `signal_live_gtk4_keyboard_shortcuts_unnamed_panel_is_a_sheet_by_identity` | 1 calculator each; opens/dismisses the in-app dialog. |
| `...::signal_live_chrome_menu_is_not_window_opened_and_two_windows_diff_correctly` | 1 Chrome profile process + 1 second Chrome launch attempt on same profile, expected to attach/open second window in same pid. |
| `...::signal_live_stopped_target_scoped_wait_answers_app_unresponsive_at_once` / `signal_live_window_taken_before_a_stop_resolves_to_app_unresponsive` | 1 calculator each. |
| `...::signal_live_unscoped_wait_with_an_unrelated_stopped_app_reports_no_events` | 1 calculator + 1 editor. |
| `...::signal_live_slow_but_alive_target_is_incomplete_and_not_unresponsive` | 1 Qt fixture, with blocking flag. |
| `system::signal_inventory::live_tests::signal_inventory_live_a_stopped_target_is_skipped_slow_roots_read_timed_out_and_resolution_names_it` | 1 calculator. |

Signal common waits: `poll` is a 100 ms retry bounded at 40 s. The action wait API receives per-call timeouts such as `OPEN_MS=15 s`, 3/4/40 s, or explicit 10 s capture/bus deadlines; these calls may run concurrently with test setup. `open_gtk4_menu_item` adds a fixed 1.2 s sleep. `dismiss_dialog` can add repeated 500 ms waits inside its 40 s poll. The one 2 s sleep in the opens-and-closes test deliberately leaves the newly opened window observable before closing it. The 800 ms sleep in slow-alive is fixture coordination.

### Tree shared rig and physical Scene call sites

`Rig::new` itself has no app; each `rig.launched.spawn` or helper `launch` below is per-test. `Scene::open` always starts one process. Tests using wrapper `single(...)` or `one(...)` still incur one Scene open; attach functions adopt already-started pids. Cases with two windows usually call Scene twice or use one Scene plus a second child, as noted.

- `tree::absence_live_tests`: two ignored cases construct `rig`; each then launches a fixture in its body/helper. One-call cases launch one toolkit app; removed-element matrix iterates GTK3/GTK4/Qt (3 app attempts) and the `rig()` does not add another.
- `tree::chromium_live_tests`: each of 2 cases calls `rig()` then local `launch`, exactly one VS Code process per case, fresh user-data directory. Frame settle is 40 s / 100 ms polling; the second-file case has a separate content wait up to 60 s.
- `tree::navigate_live_tests`: settings case starts one GNOME Settings process and opens another panel with a second launch/forward attempt; calculator relaunch case starts two calculator processes; Files case starts two Nautilus launch attempts to create two windows (forwarding/instance behavior is under test).
- `tree::qt_live_tests`: each of 3 cases uses `rig()` and one Qt launched process except the two-bus-connection case, which establishes two accessibility bus connections for the same app process. Resolve/read waits use 8 s bounds; app startup via settle 40 s.
- `tree::resolve_twin_live_tests`: the first Files sibling case starts one Nautilus process with a private temp directory; the other two cases use one launched app each per their body. `rig()` is ownership only.
- `tree::observe_live_tests`, `live_timeout`, `vocabulary_live_tests`, and `walker_live_tests`: one direct app per test function (calculator, Nautilus, terminal, Qt, VS Code, per source); terminal remains server-attributed. The walker multi-app test is one test function that sequentially exercises its declared app corpus; each `walk_app` call launches one app and pays settle+timed walks, so count is the literal `walk_app` call count in the function, not one.
- `tree::physical_live_tests`: the first twelve toolkit cases each call helper `single`/`scroll` exactly once: one Scene each. Drag case one GTK Scene. Inactive-frame refusal starts a target Scene and one decoy process (two fixture launches); menu-grab refusal starts one Scene with menu-grab fixture (one launch).
- `tree::physical_live_apps_tests`: each case starts one target app; Files/terminal/VS Code/Chrome use the named external application and app cases add helper app(s) only where explicit. Qt reveal is one Qt Scene. The helper's `attach` call itself has zero launches.
- `tree::physical_live_mutter_tests`: one Mutter/GNOME Shell process start attempt loop (source `X11Shell::start`) plus one GTK Scene; app launches total 2 after successful setup, with shell retry count bounded by its constant.



### Launch API and other multi-app test bodies

| Test | Calls to fixture/app path |
|---|---|
| `system::launch_live_tests::live_calculator_launches_by_command_id_path_and_entry_name` | 4 launch API attempts, sequentially killed between cases; expected to identify/launch Calculator through each identifier. |
| `...::live_a_second_calculator_attaches_without_a_spawn_and_with_arguments_is_a_new_owner` | 4 calls: first Calculator, alias attach, attach-disabled refusal (no spawned app), argumented second Calculator. Expected OS app instances: 2. |
| `...::live_text_editor_by_desktop_id_then_a_file_reaches_the_first_instance` | 2 attempts; second forwards file to same Text Editor process. |
| `...::live_a_qt_app_launches_by_path_with_the_environment_given` | 1 Qt launch attempt. |
| `...::live_chrome_launches_as_chromium_with_a_private_profile` | 1 Chrome launch. |
| `...::live_terminal_is_attributed_to_its_server_and_a_second_launch_adds_a_window` | 2 terminal launch attempts; expected same server pid, second window. |
| `...::live_a_missing_name_is_app_not_found_and_nothing_started` | 1 launch API attempt; zero process launches expected. |
| `...::live_an_app_that_exits_after_registering_and_before_a_window_is_not_a_launch` | 1 Python fixture launch; process has a 1.5 s loop with 50 ms internal pause and is expected to exit before app-window resolution. |
| `system::launch_live_tests::resident::live_a_running_app_with_no_window_is_started_again_and_a_window_is_reported` | 2 launch attempts and 2 resident fixture pids; first stays running with no window, second gets a window. |
| `...::resident::live_a_forwarded_launch_reports_the_new_window_not_the_old_one` | 2 Nautilus launch attempts; same pid, distinct window id. |
| `...::resident::live_a_launch_that_shows_no_window_does_not_wait_for_the_whole_timeout` | 2 rounds, 2 no-window launch attempts and owned pids. |

### One-test real-app matrices

- `tree::walker_live_tests::walker_live_real_apps` executes five `walk_app` helpers sequentially: Nautilus, gnome-terminal/server, calculator, Qt, VS Code. Exactly five launch attempts; each `walk_app` does 40 s/100 ms settle and then 8 walks total (warm-up discarded, 7 timed samples), each using a standard deadline.
- `tree::vocabulary_live_tests::vocabulary_live_real_apps` executes six helpers sequentially: Nautilus, gnome-terminal/server, calculator, Qt, VS Code, Firefox. Exactly six launch attempts. Each helper waits for complete tree using its 40 s/100 ms settle route; Firefox has a private fresh profile and VS Code a private user-data dir.
- `tree::live_apps::each_app` has four callers listed above; unlike those one-test matrices, each test launches 7 apps in a single test function.

### Shell helper retry bounds

- `capture::wayland_portal_live_support::boot_with`: `ATTEMPTS=8`, with one `gnome-shell` per reached attempt. On a successful attempt there are exactly four child launches (shell, portal GNOME backend, portal frontend, painter). A failed attempt can launch shell and zero, one, or two portal processes before returning; because a fully successful portal startup immediately proceeds to the painter and returns, only the last attempt gets a painter. Proven total child-launch bound per boot is 4–25. Keyboard `CreateSession` is up to once per reached attempt and can fail under its 8 s receive timeout.
- `input::wayland::live_shell::Shell::start`: `ATTEMPTS=8`, one GNOME shell process per reached attempt, plus up to 5 RemoteDesktop sessions per shell health attempt; health exits early at first failed open/create/start. Up to 8 shell stops plus final shell stop; each Drop has fixed 2 s.
- `tree::physical_live_shell::X11Shell::start`: one Xvfb process per test, then up to 5 GNOME Shell starts on its private display. Xvfb socket wait is 10 s/200 ms poll; each shell health wait is 20 s/200 ms poll for Mutter Overview; failed shells are killed/reaped. This is one outer helper start in the Mutter physical test, followed by one GTK Scene process.
