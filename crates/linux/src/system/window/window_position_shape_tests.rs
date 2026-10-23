use super::*;
use crate::system::window::listed_twins::listed_client;

type Sides = (i32, i32, i32, i32);
type Placed = Option<(f64, f64, f64, f64)>;

fn maximized_gtk4_client() -> X11Client {
    let mut maximized = client(7, 66, 32);
    maximized.bounds = Some(Extents {
        x: 66,
        y: 32,
        width: 1214,
        height: 768,
    });
    maximized
}

fn stale_after_a_maximize(listed: &mut [ListedWindow]) {
    listed[0].frame.extents = Some(Extents {
        x: 0,
        y: 0,
        width: 700,
        height: 520,
    });
}

fn placed(listed: &[ListedWindow]) -> Vec<Placed> {
    listed
        .iter()
        .map(|window| {
            window
                .info
                .bounds
                .map(|bounds| (bounds.x, bounds.y, bounds.width, bounds.height))
        })
        .collect()
}

#[test]
fn a_maximized_gtk4_window_has_the_size_of_its_client_not_the_stale_extents() {
    let mut listed = windows(("gtk", "4.14.5"));
    stale_after_a_maximize(&mut listed);

    place_windows(&mut listed, &ready(vec![maximized_gtk4_client()]), None);

    assert_eq!(placed(&listed), vec![Some((66.0, 32.0, 1214.0, 768.0))]);
}

#[test]
fn a_gtk4_window_follows_its_client_through_a_maximize_and_a_restore() {
    let mut listed = windows(("gtk", "4.14.5"));
    stale_after_a_maximize(&mut listed);
    let mut restored = client(7, 5, -23);
    restored.shadow = (61, 55, 61, 67);
    restored.bounds = Some(Extents {
        x: 5,
        y: -23,
        width: 822,
        height: 642,
    });
    let maximized = Some((66.0, 32.0, 1214.0, 768.0));

    place_windows(&mut listed, &ready(vec![maximized_gtk4_client()]), None);
    assert_eq!(placed(&listed), vec![maximized]);
    place_windows(&mut listed, &ready(vec![restored]), None);
    assert_eq!(placed(&listed), vec![Some((66.0, 32.0, 700.0, 520.0))]);
    place_windows(&mut listed, &ready(vec![maximized_gtk4_client()]), None);
    assert_eq!(placed(&listed), vec![maximized]);
}

#[test]
fn a_gtk4_window_with_a_unique_client_is_sized_without_accessibility_extents() {
    let mut listed = windows(("gtk", "4.14.5"));
    listed[0].frame.extents = None;

    place_windows(&mut listed, &ready(vec![maximized_gtk4_client()]), None);

    assert_eq!(placed(&listed), vec![Some((66.0, 32.0, 1214.0, 768.0))]);
}

#[test]
fn a_gtk4_client_whose_shadow_swallows_its_geometry_gives_no_rectangle() {
    let mut listed = windows(("gtk", "4.14.5"));
    let mut swallowed = client(7, 66, 32);
    swallowed.shadow = (8, 8, 8, 8);
    swallowed.bounds = Some(Extents {
        x: 66,
        y: 32,
        width: 16,
        height: 40,
    });

    place_windows(&mut listed, &ready(vec![swallowed]), None);

    assert_eq!(placed(&listed), vec![None]);
}

const TWIN_SHADOW: Sides = (26, 23, 26, 29);

fn gtk3_twin_client(id: u32, shadow: Sides) -> X11Client {
    let mut twin = client(id, 90, 59);
    twin.shadow = shadow;
    twin.bounds = Some(Extents {
        x: 90,
        y: 59,
        width: 866,
        height: 629,
    });
    twin
}

fn gtk3_twins() -> Vec<ListedWindow> {
    let raw = Extents {
        x: 90,
        y: 59,
        width: 866,
        height: 629,
    };
    twins_of_toolkit(("gtk", "3.24.41"), &[false, false], raw)
}

#[test]
fn tied_gtk3_twins_are_reported_without_the_shadow_their_clients_share() {
    let mut listed = gtk3_twins();
    let state = ready(vec![
        gtk3_twin_client(7, TWIN_SHADOW),
        gtk3_twin_client(8, TWIN_SHADOW),
    ]);

    place_windows(&mut listed, &state, None);

    let visible = Some((116.0, 82.0, 814.0, 577.0));
    assert_eq!(placed(&listed), vec![visible, visible]);
    for index in 0..2 {
        assert_eq!(
            listed_client(&listed, index, &state),
            ClientMatch::Several(vec![7, 8])
        );
    }
}

#[test]
fn tied_gtk3_twins_whose_clients_differ_in_shadow_keep_the_rectangle_they_reported() {
    let mut listed = gtk3_twins();
    let state = ready(vec![
        gtk3_twin_client(7, TWIN_SHADOW),
        gtk3_twin_client(8, (0, 0, 0, 0)),
    ]);

    place_windows(&mut listed, &state, None);

    let raw = Some((90.0, 59.0, 866.0, 629.0));
    assert_eq!(placed(&listed), vec![raw, raw]);
}
