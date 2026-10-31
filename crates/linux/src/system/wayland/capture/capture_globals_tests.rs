use super::*;

fn global(name: u32, interface: &str, version: u32) -> AdvertisedGlobal {
    AdvertisedGlobal {
        name,
        interface: interface.into(),
        version,
    }
}

fn advertised() -> Vec<AdvertisedGlobal> {
    vec![
        global(1, "wl_shm", 2),
        global(2, "wl_output", 4),
        global(3, "wl_output", 4),
        global(
            10,
            "ext_foreign_toplevel_image_capture_source_manager_v1",
            1,
        ),
        global(11, "ext_image_copy_capture_manager_v1", 1),
        global(12, "zwlr_screencopy_manager_v1", 3),
    ]
}

#[test]
fn a_toplevel_capture_binds_shm_and_both_ext_capture_managers() {
    let chosen = toplevel(&advertised()).unwrap();
    assert_eq!((chosen.shm.name, chosen.shm.version), (1, 1));
    assert_eq!((chosen.sources.name, chosen.sources.version), (10, 1));
    assert_eq!((chosen.copy.name, chosen.copy.version), (11, 1));
}

#[test]
fn a_missing_ext_capture_global_is_named() {
    let mut globals = advertised();
    globals.retain(|global| global.interface != "ext_image_copy_capture_manager_v1");
    assert_eq!(toplevel(&globals), Err("ext_image_copy_capture_manager_v1"));
    let mut globals = advertised();
    globals.retain(|global| !global.interface.starts_with("ext_foreign_toplevel_image"));
    assert_eq!(
        toplevel(&globals),
        Err("ext_foreign_toplevel_image_capture_source_manager_v1")
    );
}

#[test]
fn an_output_capture_binds_screencopy_at_version_three_and_every_named_output() {
    let mut globals = advertised();
    globals[5].version = 9;
    let chosen = output(&globals).unwrap();
    assert_eq!((chosen.screencopy.name, chosen.screencopy.version), (12, 3));
    let names: Vec<(u32, u32)> = chosen.outputs.iter().map(|o| (o.name, o.version)).collect();
    assert_eq!(names, vec![(2, 4), (3, 4)]);
}

#[test]
fn screencopy_before_version_three_or_outputs_without_names_are_refused() {
    let mut globals = advertised();
    globals[5].version = 2;
    assert_eq!(output(&globals), Err("zwlr_screencopy_manager_v1"));
    let mut globals = advertised();
    globals[1].version = 3;
    globals[2].version = 3;
    assert_eq!(output(&globals), Err("wl_output"));
}
