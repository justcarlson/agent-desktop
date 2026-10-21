use super::*;

#[test]
fn text_planning_safety() {
    let map = Keymap::sample();
    let plans = plan_text(&map, "aA!").unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(
        plans[0].steps.first(),
        Some(&Step::Key {
            code: 10,
            down: true
        })
    );
    assert_eq!(
        plans[0].steps.last(),
        Some(&Step::Key {
            code: 8,
            down: false
        })
    );
    assert!(plan_text(&map, "").unwrap().is_empty());

    let boundary = plan_text(&map, &"é".repeat(CHUNK_CHARS + 1)).unwrap();
    assert_eq!(boundary.len(), 2);
    assert_eq!(boundary[0].steps.len(), CHUNK_CHARS * 2);
    assert_eq!(boundary[1].steps.len(), 2);

    let remapped = plan_text(&map, "你你").unwrap();
    let edits: Vec<_> = remapped[0]
        .steps
        .iter()
        .filter_map(|step| match step {
            Step::Keymap(edit) => Some(edit),
            _ => None,
        })
        .collect();
    assert_eq!(edits.len(), 2);
    assert_eq!(edits[0].keycode, 14);
    assert_eq!(edits[0].keysyms, vec![0x0100_4f60; 4]);
    assert_eq!(edits[1].keysyms, vec![0; 4]);
    let key_up = remapped[0]
        .steps
        .iter()
        .rposition(|step| matches!(step, Step::Key { down: false, .. }))
        .unwrap();
    assert_eq!(
        remapped[0].steps[key_up + 1],
        Step::Pause(REMAP_RESTORE_HOLD)
    );

    let control = plan_text(&map, "a\u{7}").unwrap_err();
    assert_eq!(control.code, ErrorCode::InvalidArgs);
    assert_eq!(control.details.unwrap()["physical_delivery_started"], false);
    let no_spare = Keymap::new(
        8,
        2,
        vec![0x61, 0x41],
        2,
        vec![8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    );
    assert_eq!(
        plan_text(&no_spare, "你").unwrap_err().code,
        ErrorCode::ActionNotSupported
    );
    let no_shift = Keymap::new(8, 2, vec![0x61, 0x41], 0, Vec::new());
    assert_eq!(
        plan_text(&no_shift, "A").unwrap_err().code,
        ErrorCode::ActionNotSupported
    );
}
