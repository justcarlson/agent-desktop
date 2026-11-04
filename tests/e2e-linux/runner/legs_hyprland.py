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
)
