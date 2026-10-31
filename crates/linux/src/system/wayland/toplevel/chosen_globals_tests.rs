use super::*;

fn global(name: u32, interface: &str, version: u32) -> AdvertisedGlobal {
    AdvertisedGlobal {
        name,
        interface: interface.into(),
        version,
    }
}

fn full() -> Vec<AdvertisedGlobal> {
    vec![
        global(1, "wl_compositor", 6),
        global(7, "wl_seat", 9),
        global(30, "ext_foreign_toplevel_list_v1", 1),
        global(31, "zwlr_foreign_toplevel_manager_v1", 3),
        global(44, "hyprland_toplevel_mapping_manager_v1", 1),
    ]
}

#[test]
fn every_toplevel_global_is_chosen_by_name_at_the_highest_version_this_client_speaks() {
    let mut globals = full();
    globals[3].version = 5;

    let chosen = choose(&globals).expect("chosen");

    assert_eq!((chosen.ext_list.name, chosen.ext_list.version), (30, 1));
    assert_eq!(
        (chosen.wlr_manager.name, chosen.wlr_manager.version),
        (31, 3)
    );
    assert_eq!((chosen.mapping.name, chosen.mapping.version), (44, 1));
    assert_eq!(
        chosen.seat.map(|seat| (seat.name, seat.version)),
        Some((7, 1))
    );
}

#[test]
fn a_missing_toplevel_global_is_named() {
    for missing in [
        "ext_foreign_toplevel_list_v1",
        "zwlr_foreign_toplevel_manager_v1",
        "hyprland_toplevel_mapping_manager_v1",
    ] {
        let globals: Vec<_> = full()
            .into_iter()
            .filter(|global| global.interface != missing)
            .collect();

        assert_eq!(choose(&globals).err(), Some(missing));
    }
}

#[test]
fn a_global_at_version_zero_is_missing() {
    let mut globals = full();
    globals[3].version = 0;

    assert_eq!(
        choose(&globals).err(),
        Some("zwlr_foreign_toplevel_manager_v1")
    );
}

#[test]
fn a_session_without_a_seat_is_chosen_without_one() {
    let globals: Vec<_> = full()
        .into_iter()
        .filter(|global| global.interface != "wl_seat")
        .collect();

    assert!(choose(&globals).expect("chosen").seat.is_none());
}
