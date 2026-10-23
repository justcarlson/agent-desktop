use super::*;
use crate::system::window::listed_twins::listed_client;

type Sides = (i32, i32, i32, i32);
type Placed = Option<(f64, f64, f64, f64)>;

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
