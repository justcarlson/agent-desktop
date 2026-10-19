use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::time::Duration;

use super::{MenuWorld, app_label, run};
use crate::system::liveness::Liveness;
use crate::system::process_facts::ProcessFacts;
use agent_desktop_core::{AdapterError, Deadline, DeliverySemantics, ErrorCode, ProcessIdentity};

struct Fake {
    process: ProcessIdentity,
    reads: RefCell<VecDeque<Result<bool, AdapterError>>>,
    lives: RefCell<VecDeque<Liveness>>,
    read_count: Cell<usize>,
    pauses: RefCell<Vec<Duration>>,
}

impl Fake {
    fn new(reads: Vec<Result<bool, AdapterError>>) -> Self {
        Self {
            process: ProcessIdentity::new(7u32, "boot:1"),
            reads: RefCell::new(reads.into()),
            lives: RefCell::new(VecDeque::new()),
            read_count: Cell::new(0),
            pauses: RefCell::new(Vec::new()),
        }
    }

    fn with_lives(self, lives: Vec<Liveness>) -> Self {
        *self.lives.borrow_mut() = lives.into();
        self
    }
}

impl MenuWorld for Fake {
    fn process(&self) -> &ProcessIdentity {
        &self.process
    }

    fn liveness(&self) -> Liveness {
        let mut lives = self.lives.borrow_mut();
        if lives.len() > 1 {
            lives.pop_front().unwrap_or(Liveness::Alive)
        } else {
            lives.front().copied().unwrap_or(Liveness::Alive)
        }
    }

    fn menu_open(&self, _deadline: Deadline) -> Result<bool, AdapterError> {
        self.read_count.set(self.read_count.get() + 1);
        let mut reads = self.reads.borrow_mut();
        let next = if reads.len() > 1 {
            reads.pop_front()
        } else {
            reads.front().map(|read| match read {
                Ok(state) => Ok(*state),
                Err(error) => Err(error.clone()),
            })
        };
        next.unwrap_or(Ok(false))
    }

    fn label(&self) -> String {
        "calculator".to_owned()
    }

    fn pause(&self, gap: Duration) {
        self.pauses.borrow_mut().push(gap);
        std::thread::sleep(gap);
    }
}

fn deadline(ms: u64) -> Deadline {
    Deadline::after(ms).expect("deadline")
}

#[test]
fn an_open_menu_satisfies_the_open_wait_after_one_read() {
    let world = Fake::new(vec![Ok(true)]);
    assert!(run(&world, true, deadline(5000)).is_ok());
    assert_eq!(world.read_count.get(), 1);
    assert!(world.pauses.borrow().is_empty());
}

#[test]
fn a_closed_state_satisfies_the_closed_wait_after_one_read() {
    let world = Fake::new(vec![Ok(false)]);
    assert!(run(&world, false, deadline(5000)).is_ok());
    assert_eq!(world.read_count.get(), 1);
}

#[test]
fn a_read_that_flips_on_the_third_returns_on_the_third_at_the_fifty_millisecond_cadence() {
    let world = Fake::new(vec![Ok(false), Ok(false), Ok(true)]);
    assert!(run(&world, true, deadline(5000)).is_ok());
    assert_eq!(world.read_count.get(), 3);
    assert_eq!(
        *world.pauses.borrow(),
        [Duration::from_millis(50), Duration::from_millis(50)]
    );
}

#[test]
fn a_read_timeout_is_absorbed_and_the_next_read_runs() {
    let world = Fake::new(vec![Err(AdapterError::timeout("slow")), Ok(true)]);
    assert!(run(&world, true, deadline(5000)).is_ok());
    assert_eq!(world.read_count.get(), 2);
}

#[test]
fn expiry_with_no_successful_read_says_the_state_was_never_read() {
    let world = Fake::new(vec![Err(AdapterError::timeout("slow"))]);
    let error = run(&world, true, deadline(120)).expect_err("expires");
    assert_eq!(error.code, ErrorCode::Timeout);
    let detail = error.platform_detail.unwrap_or_default();
    assert!(detail.contains("No menu opened"), "{detail}");
    assert!(detail.contains("no poll read the menu state"), "{detail}");
}

#[test]
fn expiry_after_a_successful_read_names_the_direction_only() {
    let world = Fake::new(vec![Ok(false)]);
    let error = run(&world, true, deadline(120)).expect_err("expires");
    let detail = error.platform_detail.unwrap_or_default();
    assert!(detail.contains("No menu opened"), "{detail}");
    assert!(!detail.contains("no poll read"), "{detail}");
    let closed = Fake::new(vec![Ok(true)]);
    let error = run(&closed, false, deadline(120)).expect_err("expires");
    let detail = error.platform_detail.unwrap_or_default();
    assert!(detail.contains("Menu did not close"), "{detail}");
}

#[test]
fn a_target_that_dies_between_the_read_and_the_return_is_stale_and_never_ok() {
    let world = Fake::new(vec![Ok(true)]).with_lives(vec![Liveness::Alive, Liveness::Gone]);
    let error = run(&world, true, deadline(5000)).expect_err("stale");
    assert_eq!(error.code, ErrorCode::StaleRef);
    let details = error.details.expect("details");
    assert_eq!(details["pid"], 7);
    assert_eq!(details["process_instance"], "boot:1");
    assert_eq!(error.disposition, DeliverySemantics::not_delivered());
}

#[test]
fn a_target_gone_before_the_first_read_is_stale_without_a_read() {
    let world = Fake::new(vec![Ok(true)]).with_lives(vec![Liveness::Gone]);
    let error = run(&world, true, deadline(5000)).expect_err("stale");
    assert_eq!(error.code, ErrorCode::StaleRef);
    assert_eq!(world.read_count.get(), 0);
}

#[test]
fn a_read_error_from_a_target_that_died_is_stale() {
    let world = Fake::new(vec![Err(AdapterError::new(ErrorCode::Internal, "boom"))])
        .with_lives(vec![Liveness::Alive, Liveness::Gone]);
    let error = run(&world, true, deadline(5000)).expect_err("stale");
    assert_eq!(error.code, ErrorCode::StaleRef);
}

#[test]
fn a_read_error_from_a_live_target_propagates() {
    let world = Fake::new(vec![Err(AdapterError::new(ErrorCode::Internal, "boom"))]);
    let error = run(&world, true, deadline(5000)).expect_err("propagates");
    assert_eq!(error.code, ErrorCode::Internal);
}

#[test]
fn a_stopped_target_is_app_unresponsive_at_once() {
    let world = Fake::new(vec![Ok(false)]).with_lives(vec![Liveness::Stopped]);
    let started = std::time::Instant::now();
    let error = run(&world, true, deadline(25_000)).expect_err("stopped");
    assert_eq!(error.code, ErrorCode::AppUnresponsive);
    assert_eq!(world.read_count.get(), 0);
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn a_budget_too_small_for_the_first_read_is_a_timeout() {
    let world = Fake::new(vec![Err(AdapterError::timeout("budget"))]);
    let spent = Deadline::after(1).expect("deadline");
    std::thread::sleep(Duration::from_millis(5));
    let error = run(&world, true, spent).expect_err("timeout");
    assert_eq!(error.code, ErrorCode::Timeout);
}

#[test]
fn the_deadline_is_consulted_only_after_a_mismatch() {
    let world = Fake::new(vec![Ok(true)]);
    let spent = Deadline::after(1).expect("deadline");
    std::thread::sleep(Duration::from_millis(5));
    assert!(run(&world, true, spent).is_ok());
    assert_eq!(world.read_count.get(), 1);
}

fn facts(comm: &str, exe: Option<&str>) -> ProcessFacts {
    ProcessFacts {
        comm: comm.to_owned(),
        exe_basename: exe.map(str::to_owned),
        instance: "boot:1".to_owned(),
        web_family: None,
    }
}

#[test]
fn the_name_of_a_stopped_app_is_its_executable_then_its_comm_then_its_pid() {
    assert_eq!(
        app_label(Some(&facts("gnome-calc", Some("gnome-calculator"))), 7),
        "gnome-calculator"
    );
    assert_eq!(app_label(Some(&facts("gnome-calc", None)), 7), "gnome-calc");
    assert_eq!(app_label(Some(&facts("", None)), 7), "pid 7");
    assert_eq!(app_label(None, 7), "pid 7");
}

#[test]
fn a_stopped_target_error_names_the_app() {
    let world = Fake::new(vec![Ok(false)]).with_lives(vec![Liveness::Stopped]);
    let error = run(&world, true, deadline(5000)).expect_err("stopped");
    assert!(error.message.contains("calculator"), "{}", error.message);
}
