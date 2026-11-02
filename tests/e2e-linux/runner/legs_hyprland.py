"""Registry rows the Hyprland tier added, kept beside legs.py so it stays under the runner line cap.

Each row holds the fields of runner.legs.Leg in order; legs.py builds the Leg rows. legs.py registers FIRST_ROWS
before the zoo rows every tier shares and ROWS after them, so each tier keeps the run order it had when every row
lived in legs.py.
"""

HYPRLAND_ONLY = ("hyprland",)
HEADLESS = ("headless",)
HEADED = ("headed",)
BOTH_MODES = HEADLESS + HEADED

FIRST_ROWS = (
    ('non_interference.headed_changes_only_target', 'Headed commands leave the bystander focus, the cursor, the workspaces and the second instance as they were; the comparison detects a drifted bystander and a drifted second instance', 'Calculator, bystander and a second instance', HYPRLAND_ONLY, HEADED, ('hyprland', 'atspi'), 'pass'),
    ('non_interference.headless_route_preserves_desktop', 'Every headless inventory, snapshot, display and semantic command keeps focus, cursor and workspaces, polled while it runs, and its own effect is read from AT-SPI', 'Calculator and bystander', HYPRLAND_ONLY, HEADLESS, ('hyprland', 'atspi'), 'pass'),
    ('session.cancel_cleanup_recorded_generations', 'Cancelling a ready runner through its receipt removes its slice, adopted generations, state and second instance, and leaves the compositor, buses, shell processes and bystander alive', 'Hyprland session', HYPRLAND_ONLY, BOTH_MODES, ('hyprland', 'proc'), 'pass'),
    ('error_contract.hyprland_delivery_boundaries', 'Refusals on Hyprland carry upstream codes, exit 1 and physical_delivery_started false with not_delivered and safe retry, and the app, bystander and cursor show no effect', 'Calculator and bystander', HYPRLAND_ONLY, BOTH_MODES, ('hyprland', 'atspi', 'proc'), 'pass'),
    ('observation.hyprland_displays_match_monitors', 'list-displays over xdg_output names every Hyprland output with the origin primary first, its logical bounds and scale as hyprctl -j monitors reports them, on the session compositor, on a two-output second instance and after that instance takes a fractional scale', 'Hyprland session and a second instance', HYPRLAND_ONLY, HEADLESS, ('hyprland',), 'pass'),
)

ROWS = (
    ('session.hyprland_silent_ipc_refused', 'A Hyprland endpoint that never accepts, or accepts and never replies, refuses list-displays when named, with or without the session display, and when discovery finds it beside the session instance; the session pair lists its outputs once the endpoint is gone; the desktop and its monitors stay unchanged', 'Hyprland session and socket fixtures', HYPRLAND_ONLY, HEADLESS, ('hyprland',), 'asserted-refusal', 'TIMEOUT with display_unavailable silent, retryable true and the endpoint socket as detail, exit 1, not_delivered within the budget; desktop and monitors unchanged'),
    ('session.hyprland_several_instances_ambiguous', 'With two live Hyprland instances, discovery with only the runtime directory, and with the session display, refuses list-displays as hyprland_ambiguous naming both signatures; each named pair still lists its own outputs; both desktops stay unchanged', 'Hyprland session and a second instance', HYPRLAND_ONLY, HEADLESS, ('hyprland',), 'asserted-refusal', 'PLATFORM_NOT_SUPPORTED with hyprland_ambiguous and details.signatures equal to the two live signatures, exit 1, not_delivered within the budget; both desktops and monitors unchanged'),
    ('session.hyprland_swapped_pair_mismatch', 'A signature paired with the other live instance Wayland display refuses list-displays as hyprland_mismatch, both ways round; both desktops stay unchanged', 'Hyprland session and a second instance', HYPRLAND_ONLY, HEADLESS, ('hyprland',), 'asserted-refusal', 'PLATFORM_NOT_SUPPORTED with hyprland_mismatch naming the bound signature and the foreign Wayland socket, exit 1, not_delivered within the budget; both desktops and monitors unchanged'),
    ('session.hyprland_effects_stay_on_bound_instance', 'list-windows under each pair lists compositor-only toplevels of that instance alone, under ids it issued, so only the second pair lists the sentinel; the sentinel id through the session pair is WINDOW_NOT_FOUND and both desktops stay unchanged; minimize and restore of the sentinel through the second pair show in its j/clients and leave the session instance j/clients and desktop unchanged; minimize and restore of a runner-owned window through the session pair show in the session j/clients and leave the second instance desktop and stable j/clients unchanged; each unchanged side is polled while the other acts', 'Hyprland session, a runner-owned Zoo GTK3 window, a second instance and its sentinel', HYPRLAND_ONLY, HEADLESS, ('hyprland', 'proc'), 'pass'),
    ('identity.hyprland_two_processes_one_title', 'Two processes with one window title and one size each bind their own toplevel: minimize and restore by each frame id move that process toplevel in j/clients and leave the other process toplevel unchanged; focus and the desktop stay unchanged', 'Two Zoo GTK3 processes and bystander', HYPRLAND_ONLY, HEADLESS, ('hyprland', 'atspi'), 'pass'),
)
