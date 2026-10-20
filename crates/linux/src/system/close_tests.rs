use agent_desktop_core::{DeliverySemantics, ErrorCode};

use super::fake::{Fake, facts};
use super::{Request, run};
use crate::system::liveness::Liveness;

fn request(force: bool) -> Request<'static> {
    Request {
        name: "Calculator",
        pid: 42,
        instance: Some("t"),
        force,
    }
}

const TABLE: &[&str] = &["gnome-shell", "Xorg"];

#[test]
fn a_protected_listed_name_is_refused_before_any_read() {
    let mut world = Fake::new(Liveness::Alive);
    world.protected = TABLE;
    let named = Request {
        name: "gnome-shell",
        ..request(false)
    };
    let error = run(&world, &named).expect_err("refused");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(world.count("facts"), 0);
    assert_eq!(world.count("liveness"), 0);
}

#[test]
fn a_benign_name_on_a_protected_process_is_refused_before_any_send() {
    for force in [false, true] {
        let mut world = Fake::new(Liveness::Alive);
        world.protected = TABLE;
        world.facts = facts("Xorg");
        let error = run(&world, &request(force)).expect_err("refused");
        assert_eq!(error.code, ErrorCode::InvalidArgs);
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        for sent in ["toplevels", "close", "term", "kill"] {
            assert_eq!(world.count(sent), 0, "{sent} force={force}");
        }
    }
}

#[test]
fn both_the_command_name_and_the_executable_name_are_consulted() {
    let mut world = Fake::new(Liveness::Alive);
    world.protected = TABLE;
    world.facts =
        crate::system::process_facts::Lookup::Found(crate::system::process_facts::ProcessFacts {
            comm: "renamed".to_owned(),
            exe_basename: Some("gnome-shell".to_owned()),
            instance: "t".to_owned(),
            web_family: None,
        });
    let error = run(&world, &request(true)).expect_err("refused by executable");
    assert_eq!(error.code, ErrorCode::InvalidArgs);
    assert_eq!(world.count("term"), 0);
}

#[test]
fn a_process_whose_facts_cannot_be_read_is_not_closed() {
    let mut world = Fake::new(Liveness::Alive);
    world.facts = crate::system::process_facts::Lookup::Unreadable;
    let error = run(&world, &request(true)).expect_err("refused");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    assert_eq!(world.count("term"), 0);
}

#[test]
fn a_missing_token_is_invalid_args_and_sends_nothing() {
    for force in [false, true] {
        let world = Fake::new(Liveness::Alive);
        let tokenless = Request {
            instance: None,
            ..request(force)
        };
        let error = run(&world, &tokenless).expect_err("refused");
        assert_eq!(error.code, ErrorCode::InvalidArgs);
        assert_eq!(error.disposition, DeliverySemantics::not_delivered());
        for sent in ["toplevels", "close", "term", "kill"] {
            assert_eq!(world.count(sent), 0, "{sent} force={force}");
        }
    }
}

#[test]
fn a_changed_token_is_a_benign_ok_that_signals_nothing() {
    for force in [false, true] {
        let world = Fake::new(Liveness::Gone);
        run(&world, &request(force)).expect("benign");
        for sent in ["toplevels", "close", "term", "kill"] {
            assert_eq!(world.count(sent), 0, "{sent} force={force}");
        }
    }
}
