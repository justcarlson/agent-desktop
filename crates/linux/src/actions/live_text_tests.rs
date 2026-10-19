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
fn type_inserts_ascii_emoji_and_a_combining_mark_at_the_caret() {
    each_toolkit(|toolkit, bench| {
        let field = bench.named("field");
        bench
            .act(&field, Action::SetValue(String::new()))
            .unwrap_or_else(|error| panic!("{toolkit:?}: {error}"));
        for piece in ["ab", "\u{1F600}", "e\u{301}"] {
            bench
                .act(&field, Action::TypeText(piece.to_owned()))
                .unwrap_or_else(|error| panic!("{toolkit:?} {piece:?}: {error}"));
        }
        let read = bench.text_of(&field);
        assert!(read.contains('\u{1F600}'), "{toolkit:?}: {read:?}");
        assert!(read.contains("e\u{301}"), "{toolkit:?}: {read:?}");
        assert!(read.contains("ab"), "{toolkit:?}: {read:?}");
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
