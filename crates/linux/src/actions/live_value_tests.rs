use agent_desktop_core::{Action, ErrorCode};

use super::live_text_tests::each_toolkit;

const SCROLL_BAR: u32 = 48;

#[test]
#[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]
fn a_spin_button_takes_a_whole_number_and_refuses_a_fraction_in_every_toolkit() {
    each_toolkit(|toolkit, bench| {
        let spin = bench.named("spin");
        bench
            .act(&spin, Action::SetValue("7".to_owned()))
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        assert_eq!(bench.number_of(&spin), 7.0, "{toolkit:?}");
        let refused = bench
            .act(&spin, Action::SetValue("7.5".to_owned()))
            .expect_err("a fraction is refused");
        assert_eq!(refused.code, ErrorCode::InvalidArgs, "{toolkit:?}");
        assert_eq!(bench.number_of(&spin), 7.0, "{toolkit:?}");
    });
}

#[test]
#[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]
fn a_slider_is_clamped_and_the_stored_value_is_not_reported_as_verified() {
    each_toolkit(|toolkit, bench| {
        let scale = bench.named("scale");
        let result = bench
            .act(&scale, Action::SetValue("500".to_owned()))
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        assert_eq!(bench.number_of(&scale), 100.0, "{toolkit:?}");
        let proved = result.steps.iter().any(|step| step.verified == Some(true));
        assert!(!proved, "{toolkit:?}: a clamped write must not verify");
        bench
            .act(&scale, Action::SetValue("0.2".to_owned()))
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        assert!(bench.number_of(&scale) <= 1.0, "{toolkit:?}");
    });
}

#[test]
#[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]
fn a_scroll_bar_write_moves_the_content() {
    each_toolkit(|toolkit, bench| {
        let Some(bar) = bench.with_role(SCROLL_BAR) else {
            println!("skipped {toolkit:?}: no scroll bar in the fixture");
            return;
        };
        let before = bench.number_of(&bar);
        bench
            .act(&bar, Action::SetValue("1000000".to_owned()))
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        assert!(bench.number_of(&bar) > before, "{toolkit:?}");
    });
}
