use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;
use std::rc::Rc;

use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics};

use super::super::keycode_sink::KeycodeSink;
use super::KeycodeSession;
use crate::input::keyboard::keysym_step::KeysymStep;
use crate::input::wayland::key_session::KeySession;
use crate::input::wayland::keysym_send::run_chunks;

const OWNER_DE: &str = include_str!("de_nodeadkeys_compose_ralt.xkb");
const KEYSYM_A: u32 = 0x61;
const KEYSYM_Z_UPPER: u32 = 0x5a;
const KEYSYM_EACUTE: u32 = 0xe9;
const KEYSYM_OUT_OF_RANGE: u32 = 0x2000_0000;
const LAYOUT: u32 = 0;

type Log = Rc<RefCell<Vec<String>>>;

struct FakeSink {
    log: Log,
    keymaps: Vec<String>,
    held: BTreeSet<u32>,
    fail_on: Option<&'static str>,
    delivers_failed_key: bool,
}

impl FakeSink {
    fn new(log: &Log) -> Self {
        Self {
            log: Rc::clone(log),
            keymaps: Vec::new(),
            held: BTreeSet::new(),
            fail_on: None,
            delivers_failed_key: false,
        }
    }

    fn record(&mut self, label: String) -> Result<(), AdapterError> {
        self.log.borrow_mut().push(label.clone());
        if self.fail_on == Some(label.as_str()) {
            return Err(AdapterError::internal("the fake sink failed this key"));
        }
        Ok(())
    }
}

impl KeycodeSink for FakeSink {
    fn upload_keymap(&mut self, keymap: &str, _deadline: Deadline) -> Result<(), AdapterError> {
        self.keymaps.push(keymap.to_owned());
        self.record("upload".to_owned())
    }

    fn key(&mut self, code: u32, down: bool, _deadline: Deadline) -> Result<(), AdapterError> {
        let edge = if down { "down" } else { "up" };
        let label = format!("{edge}:{code}");
        let failing = self.fail_on == Some(label.as_str());
        if !failing || self.delivers_failed_key {
            if down {
                self.held.insert(code);
            } else {
                self.held.remove(&code);
            }
        }
        self.record(label)
    }
}

fn live() -> Deadline {
    Deadline::after(5_000).expect("a five second deadline builds")
}

fn edge(keysym: u32, down: bool) -> KeysymStep {
    KeysymStep { keysym, down }
}

fn tap(keysym: u32) -> Vec<KeysymStep> {
    vec![edge(keysym, true), edge(keysym, false)]
}

fn session(log: &Log, chunks: &[&[KeysymStep]]) -> KeycodeSession<FakeSink> {
    KeycodeSession::new(FakeSink::new(log), OWNER_DE.to_owned(), LAYOUT, chunks)
}

fn gate_failing_at(log: &Log, failing_call: u32) -> impl FnMut() -> Result<(), AdapterError> {
    let log = Rc::clone(log);
    let calls = Cell::new(0_u32);
    move || {
        calls.set(calls.get() + 1);
        log.borrow_mut().push("gate".to_owned());
        if calls.get() == failing_call {
            return Err(AdapterError::internal("focus moved to another window"));
        }
        Ok(())
    }
}

fn events(log: &Log) -> Vec<String> {
    log.borrow().clone()
}

#[test]
fn the_keymap_is_uploaded_once_before_the_first_key() {
    let log = Log::default();
    let first = tap(KEYSYM_EACUTE);
    let second = tap(KEYSYM_A);
    let chunks: [&[KeysymStep]; 2] = [&first, &second];
    let mut keyboard = session(&log, &chunks);
    let delivery = run_chunks(&mut keyboard, &chunks, &mut || Ok(()), live()).expect("keys go");
    assert_eq!(delivery.delivered_events, 4);
    let events = events(&log);
    assert_eq!(events.first().map(String::as_str), Some("upload"));
    assert_eq!(events.iter().filter(|event| *event == "upload").count(), 1);
    let sink = keyboard.sink();
    assert_eq!(sink.keymaps.len(), 1);
    assert!(sink.keymaps[0].contains("[ eacute ]"));
    assert!(sink.held.is_empty());
}

#[test]
fn the_gate_runs_before_every_key_down_including_a_wrapped_shift() {
    let log = Log::default();
    let keys = tap(KEYSYM_Z_UPPER);
    let chunks: [&[KeysymStep]; 1] = [&keys];
    let mut keyboard = session(&log, &chunks);
    let mut gate = gate_failing_at(&log, 0);
    run_chunks(&mut keyboard, &chunks, &mut gate, live()).expect("keys go");
    assert_eq!(
        events(&log),
        [
            "upload", "gate", "gate", "down:42", "up:42", "gate", "gate", "down:42", "down:21",
            "up:21", "up:42",
        ]
    );
}

#[test]
fn a_gate_failure_mid_batch_releases_every_held_key_and_is_uncertain() {
    let log = Log::default();
    let keys = [
        edge(KEYSYM_Z_UPPER, true),
        edge(KEYSYM_A, true),
        edge(KEYSYM_A, false),
        edge(KEYSYM_Z_UPPER, false),
    ];
    let chunks: [&[KeysymStep]; 1] = [&keys];
    let mut keyboard = session(&log, &chunks);
    let mut gate = gate_failing_at(&log, 5);
    let error = run_chunks(&mut keyboard, &chunks, &mut gate, live())
        .expect_err("focus moved before the second key");
    assert_eq!(
        events(&log),
        [
            "upload", "gate", "gate", "down:42", "up:42", "gate", "gate", "down:42", "down:21",
            "gate", "up:21", "up:42",
        ]
    );
    assert!(keyboard.sink().held.is_empty());
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    let details = error.details.expect("delivery evidence");
    assert_eq!(details["physical_delivery_started"], true);
    assert_eq!(details["delivered_events"], 3);
}

#[test]
fn a_sink_failure_inside_a_wrapped_key_releases_its_shift() {
    let log = Log::default();
    let keys = tap(KEYSYM_Z_UPPER);
    let chunks: [&[KeysymStep]; 1] = [&keys];
    let mut keyboard = session(&log, &chunks);
    keyboard.sink.fail_on = Some("down:21");
    let error = run_chunks(&mut keyboard, &chunks, &mut || Ok(()), live())
        .expect_err("the key itself failed");
    assert!(keyboard.sink().held.is_empty(), "{:?}", events(&log));
    assert_eq!(events(&log).last().map(String::as_str), Some("up:42"));
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
    let details = error.details.expect("delivery evidence");
    assert_eq!(details["delivered_events"], 2);
}

#[test]
fn a_key_down_the_sink_delivered_before_failing_is_released() {
    let log = Log::default();
    let keys = tap(KEYSYM_A);
    let chunks: [&[KeysymStep]; 1] = [&keys];
    let mut keyboard = session(&log, &chunks);
    keyboard.sink.fail_on = Some("down:30");
    keyboard.sink.delivers_failed_key = true;
    let error = run_chunks(&mut keyboard, &chunks, &mut || Ok(()), live())
        .expect_err("the key reply was lost");
    assert!(keyboard.sink().held.is_empty(), "{:?}", events(&log));
    assert_eq!(error.disposition, DeliverySemantics::uncertain());
}

#[test]
fn a_keysym_with_no_placement_is_refused_before_any_event() {
    let log = Log::default();
    let keys = tap(KEYSYM_OUT_OF_RANGE);
    let chunks: [&[KeysymStep]; 1] = [&keys];
    let mut keyboard = session(&log, &chunks);
    let mut gate = gate_failing_at(&log, 0);
    let error =
        run_chunks(&mut keyboard, &chunks, &mut gate, live()).expect_err("the keysym has no key");
    assert!(events(&log).is_empty(), "{:?}", events(&log));
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
    let details = error.details.expect("refusal details");
    assert_eq!(details["physical_delivery_started"], false);
    assert_eq!(details["cause"], "keysym_invalid");
}

#[test]
fn the_session_is_never_kept_or_resumed() {
    let log = Log::default();
    let keys = tap(KEYSYM_A);
    let chunks: [&[KeysymStep]; 1] = [&keys];
    let mut keyboard = session(&log, &chunks);
    run_chunks(&mut keyboard, &chunks, &mut || Ok(()), live()).expect("keys go");
    assert!(!keyboard.retain());
    assert!(!keyboard.resumed());
}
