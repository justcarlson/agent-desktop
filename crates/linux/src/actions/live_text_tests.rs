use agent_desktop_core::{Action, ActionResult};

use super::live_bench::each_toolkit;

fn verified(result: &ActionResult) -> Vec<Option<bool>> {
    result.steps.iter().map(|step| step.verified).collect()
}

#[test]
#[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]
fn set_value_and_clear_land_on_a_text_field_in_every_toolkit() {
    each_toolkit(|toolkit, bench| {
        let field = bench.named("field");
        let result = bench
            .act(&field, Action::SetValue("hello".to_owned()))
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        assert_eq!(bench.text_of(&field), "hello", "{toolkit:?}");
        assert!(verified(&result).contains(&Some(true)), "{toolkit:?}");
        bench
            .act(&field, Action::Clear)
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        assert_eq!(bench.text_of(&field), "", "{toolkit:?}");
    });
}

#[test]
#[ignore = "needs lane A's write engine and lane B's caret read; run through scripts/private-session.sh --wm"]
fn type_inserts_non_ascii_text_into_an_empty_field_exactly() {
    each_toolkit(|toolkit, bench| {
        let field = bench.named("field");
        for piece in [
            "ab",
            "\u{e9}",
            "\u{1F600}",
            "e\u{301}",
            "ab\u{e9}\u{1F600}e\u{301}",
        ] {
            bench
                .act(&field, Action::SetValue(String::new()))
                .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
            bench
                .act(&field, Action::TypeText(piece.to_owned()))
                .unwrap_or_else(|error| panic!("{toolkit:?} {piece:?}: {error}"));
            assert_eq!(bench.text_of(&field), piece, "{toolkit:?} {piece:?}");
        }
    });
}

#[test]
#[ignore = "needs lane A's write engine and lane B's caret read; run through scripts/private-session.sh --wm"]
fn type_after_a_wide_character_lands_at_the_caret_exactly() {
    each_toolkit(|toolkit, bench| {
        let field = bench.named("field");
        bench
            .act(&field, Action::SetValue("a\u{1F600}".to_owned()))
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        bench
            .act(&field, Action::TypeText("\u{e9}".to_owned()))
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        let read = bench.text_of(&field);
        assert!(
            read == "a\u{1F600}\u{e9}" || read == "\u{e9}a\u{1F600}",
            "{toolkit:?}: {read:?}"
        );
    });
}

#[test]
#[ignore = "needs lane A's write engine; run through scripts/private-session.sh --wm"]
fn a_password_field_write_lands_and_the_result_never_carries_the_text() {
    const SECRET: &str = "zx-secret-7731";
    each_toolkit(|toolkit, bench| {
        let secret = bench.named("secret");
        let outcome = bench.act(&secret, Action::SetValue(SECRET.to_owned()));
        let rendered = match &outcome {
            Ok(result) => serde_json::to_string(result).expect("a result serializes"),
            Err(error) => format!("{error:?}"),
        };
        assert!(!rendered.contains(SECRET), "{toolkit:?}: {rendered}");
        if let Ok(result) = &outcome {
            assert!(!verified(result).contains(&Some(true)), "{toolkit:?}");
        }
    });
}
