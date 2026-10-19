use super::*;

fn key(code: u8, down: bool) -> Step {
    Step::Key { code, down }
}

fn typed_keys(plans: &[Plan]) -> usize {
    plans
        .iter()
        .flat_map(|plan| &plan.steps)
        .filter(|step| matches!(step, Step::Key { down: true, code } if *code != 8))
        .count()
}

#[test]
fn a_lowercase_character_is_one_press_and_release() {
    let plans = plan_text(&Keymap::sample(), "a").unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!(plans[0].steps, vec![key(10, true), key(10, false)]);
}

#[test]
fn an_uppercase_character_is_wrapped_in_shift() {
    let plans = plan_text(&Keymap::sample(), "A").unwrap();
    assert_eq!(
        plans[0].steps,
        vec![key(8, true), key(10, true), key(10, false), key(8, false)]
    );
}

#[test]
fn a_character_on_a_shifted_level_of_a_digit_key_is_wrapped_in_shift() {
    let plans = plan_text(&Keymap::sample(), "!").unwrap();
    assert_eq!(plans[0].steps.first(), Some(&key(8, true)));
    assert_eq!(plans[0].steps.last(), Some(&key(8, false)));
}

#[test]
fn two_thousand_characters_split_into_chunks_of_at_most_thirty_two() {
    let text = "a".repeat(2000);
    let plans = plan_text(&Keymap::sample(), &text).unwrap();
    assert_eq!(plans.len(), 2000_usize.div_ceil(CHUNK_CHARS));
    assert!(plans.iter().all(|plan| plan.steps.len() <= CHUNK_CHARS * 2));
    assert_eq!(typed_keys(&plans), 2000);
}

#[test]
fn chunks_cover_the_text_without_gaps_or_overlap() {
    let text = "a1A!".repeat(40);
    let plans = plan_text(&Keymap::sample(), &text).unwrap();
    assert_eq!(typed_keys(&plans), text.chars().count());
}

#[test]
fn a_chunk_boundary_counts_characters_not_bytes() {
    let text = "é".repeat(33);
    let plans = plan_text(&Keymap::sample(), &text).unwrap();
    assert_eq!(plans.len(), 2);
    assert_eq!(typed_keys(&plans[..1]), 32);
    assert_eq!(typed_keys(&plans[1..]), 1);
}

#[test]
fn empty_text_plans_nothing() {
    assert!(plan_text(&Keymap::sample(), "").unwrap().is_empty());
}

#[test]
fn the_refuse_route_fails_before_any_plan_and_names_the_class_not_the_text() {
    let error = plan_text_with(&Keymap::sample(), "ab你好", UnmappedRoute::Refuse).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    let rendered = format!("{error:?}");
    assert!(rendered.contains("other_script_letter"));
    assert!(!rendered.contains('你') && !rendered.contains('好'));
    assert_eq!(error.details.unwrap()["physical_delivery_started"], false);
}

#[test]
fn an_emoji_is_classed_as_a_symbol() {
    let error = plan_text_with(&Keymap::sample(), "😀", UnmappedRoute::Refuse).unwrap_err();
    assert!(format!("{error:?}").contains("symbol_or_emoji"));
}

#[test]
fn a_control_character_is_refused_on_both_routes() {
    for route in [UnmappedRoute::Refuse, UnmappedRoute::Remap] {
        let error = plan_text_with(&Keymap::sample(), "\u{7}", route).unwrap_err();
        assert!(format!("{error:?}").contains("control"));
    }
}

#[test]
fn the_remap_route_edits_the_spare_keycode_settles_types_and_restores() {
    let plans = plan_text_with(&Keymap::sample(), "你", UnmappedRoute::Remap).unwrap();
    let steps = &plans[0].steps;
    let keysym = 0x0100_0000 + 0x4f60;
    assert_eq!(
        steps,
        &vec![
            Step::Keymap(KeymapEdit {
                keycode: 14,
                keysyms: vec![keysym; 4]
            }),
            Step::Pause(REMAP_SETTLE),
            key(14, true),
            key(14, false),
            Step::Keymap(KeymapEdit {
                keycode: 14,
                keysyms: vec![0; 4]
            }),
        ]
    );
}

#[test]
fn the_remap_is_restored_once_per_chunk_that_used_it() {
    let text = format!("{}你{}", "a".repeat(32), "a".repeat(32));
    let plans = plan_text_with(&Keymap::sample(), &text, UnmappedRoute::Remap).unwrap();
    let edits = |plan: &Plan| {
        plan.steps
            .iter()
            .filter(|s| matches!(s, Step::Keymap(_)))
            .count()
    };
    assert_eq!(plans.len(), 3);
    assert_eq!(edits(&plans[0]), 0);
    assert_eq!(edits(&plans[1]), 2);
    assert_eq!(edits(&plans[2]), 0);
}

#[test]
fn the_remap_route_without_a_spare_keycode_is_not_supported() {
    let map = Keymap::parse(
        8,
        2,
        vec![0x61, 0x41],
        2,
        vec![8, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
    );
    let error = plan_text_with(&map, "你", UnmappedRoute::Remap).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}

#[test]
fn a_shifted_character_without_a_shift_key_is_not_supported() {
    let map = Keymap::parse(8, 2, vec![0x61, 0x41], 0, Vec::new());
    let error = plan_text(&map, "A").unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
}

#[test]
fn the_shipped_route_is_the_one_the_probes_settled() {
    assert_eq!(UNMAPPED_ROUTE, UnmappedRoute::Refuse);
}

#[test]
fn keysym_text_uses_keysyms_directly_with_no_remap_and_chunks_by_character() {
    let chunks = plan_text_keysyms("你好");
    assert_eq!(chunks.len(), 1);
    assert_eq!(
        chunks[0],
        vec![
            KeysymStep {
                keysym: 0x0100_4f60,
                down: true
            },
            KeysymStep {
                keysym: 0x0100_4f60,
                down: false
            },
            KeysymStep {
                keysym: 0x0100_597d,
                down: true
            },
            KeysymStep {
                keysym: 0x0100_597d,
                down: false
            },
        ]
    );
    assert_eq!(plan_text_keysyms(&"a".repeat(65)).len(), 3);
    assert!(plan_text_keysyms("").is_empty());
}
