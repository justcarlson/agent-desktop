use super::*;

const TRUSTED: Standing = Standing {
    calibrated: true,
    descends: true,
    same_window: true,
};

const COVER: Occlusion = Occlusion::CoveredBy {
    window: 7,
    frame: Extents {
        x: 1,
        y: 2,
        width: 3,
        height: 4,
    },
};

fn covered() -> Verdict {
    Verdict::CoveredBy {
        window: 7,
        frame: Extents {
            x: 1,
            y: 2,
            width: 3,
            height: 4,
        },
    }
}

#[test]
fn landing_on_the_target_with_a_clear_stack_reaches_it() {
    assert_eq!(
        decide(TRUSTED, Landing::OnTarget, Occlusion::Clear),
        Verdict::Reaches
    );
}

#[test]
fn landing_on_the_target_is_unknown_unless_the_stack_is_clear() {
    for occlusion in [Occlusion::OverrideRedirectOver, Occlusion::Unknown] {
        assert_eq!(
            decide(TRUSTED, Landing::OnTarget, occlusion),
            Verdict::Unknown
        );
    }
}

#[test]
fn landing_on_an_ancestor_is_unknown() {
    assert_eq!(
        decide(TRUSTED, Landing::Ancestor, Occlusion::Clear),
        Verdict::Unknown
    );
}

#[test]
fn a_trusted_same_window_lander_whose_extents_hold_the_point_intercepts() {
    assert_eq!(
        decide(
            TRUSTED,
            Landing::Elsewhere { contained: true },
            Occlusion::Clear
        ),
        Verdict::SameWindowLander
    );
}

#[test]
fn an_untrusted_toolkit_lander_is_unknown() {
    let untrusted = Standing {
        same_window: false,
        ..TRUSTED
    };
    assert_eq!(
        decide(
            untrusted,
            Landing::Elsewhere { contained: true },
            Occlusion::Clear
        ),
        Verdict::Unknown
    );
}

#[test]
fn a_lander_whose_extents_do_not_hold_the_point_is_unknown() {
    assert_eq!(
        decide(
            TRUSTED,
            Landing::Elsewhere { contained: false },
            Occlusion::Clear
        ),
        Verdict::Unknown
    );
}

#[test]
fn a_same_window_lander_under_an_override_redirect_window_is_unknown() {
    assert_eq!(
        decide(
            TRUSTED,
            Landing::Elsewhere { contained: true },
            Occlusion::OverrideRedirectOver
        ),
        Verdict::Unknown
    );
}

#[test]
fn a_cover_intercepts_whatever_the_descent_landed_on() {
    for landing in [
        Landing::OnTarget,
        Landing::Ancestor,
        Landing::Elsewhere { contained: false },
        Landing::Unresolved,
        Landing::NotRead,
    ] {
        assert_eq!(decide(TRUSTED, landing, COVER), covered());
    }
}

#[test]
fn an_unresolved_descent_with_a_clear_stack_is_unknown() {
    assert_eq!(
        decide(TRUSTED, Landing::Unresolved, Occlusion::Clear),
        Verdict::Unknown
    );
}

#[test]
fn geometry_that_was_not_proved_is_unknown_for_every_input() {
    let uncalibrated = Standing {
        calibrated: false,
        ..TRUSTED
    };
    for landing in [Landing::OnTarget, Landing::Elsewhere { contained: true }] {
        for occlusion in [Occlusion::Clear, COVER] {
            assert_eq!(decide(uncalibrated, landing, occlusion), Verdict::Unknown);
        }
    }
}

#[test]
fn a_toolkit_whose_descent_is_not_trusted_is_unknown_even_when_covered() {
    let blind = Standing {
        descends: false,
        ..TRUSTED
    };
    assert_eq!(decide(blind, Landing::OnTarget, COVER), Verdict::Unknown);
}

#[test]
fn calibrated_gtk_four_is_covered_or_reaches_but_never_a_same_window_lander() {
    let gtk4 = Standing {
        same_window: false,
        ..TRUSTED
    };
    assert_eq!(decide(gtk4, Landing::NotRead, COVER), covered());
    assert_eq!(
        decide(gtk4, Landing::OnTarget, Occlusion::Clear),
        Verdict::Reaches
    );
    assert_eq!(
        decide(
            gtk4,
            Landing::Elsewhere { contained: true },
            Occlusion::Clear
        ),
        Verdict::Unknown
    );
}

#[test]
fn the_descent_runs_only_when_nothing_has_decided_yet() {
    assert!(needs_landing(TRUSTED, Occlusion::Clear));
    assert!(!needs_landing(TRUSTED, COVER));
    assert!(!needs_landing(TRUSTED, Occlusion::Unknown));
    assert!(!needs_landing(
        Standing {
            calibrated: false,
            ..TRUSTED
        },
        Occlusion::Clear
    ));
}
