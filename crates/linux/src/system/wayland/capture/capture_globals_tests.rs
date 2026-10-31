use super::*;

fn global(name: u32, interface: &str, version: u32) -> Global {
    Global {
        name,
        interface: interface.into(),
        version,
    }
}

fn advertised() -> Vec<Global> {
    vec![
        global(1, "wl_shm", 2),
        global(2, "wl_output", 4),
        global(3, "wl_output", 4),
        global(12, "zwlr_screencopy_manager_v1", 3),
    ]
}

fn chosen(globals: Vec<Global>) -> Result<OutputCaptureGlobals, &'static str> {
    output(&Exchange::advertising(globals))
}

#[test]
fn an_output_capture_binds_shm_screencopy_and_every_named_output() {
    let chosen = chosen(advertised()).unwrap();
    assert_eq!(chosen.shm.name, 1);
    assert_eq!(chosen.screencopy.name, 12);
    let names: Vec<u32> = chosen.outputs.iter().map(|output| output.name).collect();
    assert_eq!(names, vec![2, 3]);
}

#[test]
fn screencopy_before_version_three_or_outputs_without_names_are_refused() {
    let mut globals = advertised();
    globals[3].version = 2;
    assert_eq!(chosen(globals), Err("zwlr_screencopy_manager_v1"));
    let mut globals = advertised();
    globals[1].version = 3;
    globals[2].version = 3;
    assert_eq!(chosen(globals), Err("wl_output"));
    let mut globals = advertised();
    globals.remove(0);
    assert_eq!(chosen(globals), Err("wl_shm"));
}
