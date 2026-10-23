use super::*;

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
