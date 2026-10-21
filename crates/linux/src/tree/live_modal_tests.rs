use std::io::Read;
use std::process::{Command, Stdio};

use super::*;
use crate::tree::ancestry;
use crate::tree::node_evidence::evidence_for;
use crate::tree::role_numbers::{FRAME, PUSH_BUTTON};
use crate::tree::state_bits::{MODAL, SHOWING};
use crate::tree::test_support::{read_of, words};
use crate::tree::{Field, ObservedRead};

struct Desktop {
    dialog_states: Field<Vec<u32>>,
    listed: bool,
    unavailable: Option<&'static str>,
}

fn element(path: &str) -> AtspiElement {
    AtspiElement::new(":1.42", path, 42)
}

impl NodeReader for Desktop {
    fn read(&mut self, element: &AtspiElement) -> Result<NodeEvidence, AdapterError> {
        crate::tree::node_evidence::require_ancestry(element)?;
        if self.unavailable == Some(element.path()) {
            return Err(AdapterError::timeout("window did not answer"));
        }
        let mut read: ObservedRead = match element.path() {
            "/dialog" => {
                let mut read = read_of(FRAME, "Preferences", &[]);
                read.node.traits.states = self.dialog_states.clone();
                read
            }
            "/parent" => read_of(FRAME, "Calculator", &[]),
            _ => read_of(PUSH_BUTTON, "7", &["click"]),
        };
        read.node.element = element.clone();
        Ok(NodeEvidence {
            evidence: evidence_for(&read, element),
            read,
            liveness: Liveness::Alive,
        })
    }

    fn top_levels(
        &mut self,
        target: &AtspiElement,
        _: Deadline,
    ) -> Result<Vec<AtspiElement>, AdapterError> {
        if self.unavailable == Some("listing") {
            return Err(AdapterError::timeout("window listing did not answer"));
        }
        Ok(if self.listed {
            ["/parent", "/dialog"]
                .map(|path| AtspiElement::new(target.bus_name(), path, target.pid()))
                .to_vec()
        } else {
            vec![]
        })
    }

    fn stopped_app(&mut self, _: u32) -> Option<String> {
        None
    }

    fn placement(&mut self, _: &AtspiElement, _: Deadline) -> Placement {
        Placement::default()
    }

    fn global_position(&mut self, _: &AtspiElement, _: Deadline) -> bool {
        true
    }
}

fn target(frame: &str) -> NativeHandle {
    let mut target = element("/button");
    target.set_ancestry(ancestry::of_root(&element(frame)));
    target.into_native_handle()
}

#[test]
fn a_renderer_target_checks_its_owning_frames_dialogs() {
    let mut renderer = AtspiElement::new(":1.99", "/button", 99);
    renderer.set_ancestry(ancestry::of_root(&element("/parent")));
    let handle = renderer.into_native_handle();
    let mut desktop = Desktop {
        dialog_states: Field::Present(words(&[])),
        listed: true,
        unavailable: None,
    };
    let deadline = Deadline::standard().unwrap();
    assert!(element_via(&mut desktop, &handle, deadline).is_ok());
    desktop.dialog_states = Field::Present(words(&[MODAL, SHOWING]));
    let error = element_via(&mut desktop, &handle, deadline).unwrap_err();
    assert_eq!(error.details.unwrap()["cause"], "blocked_by_modal_dialog");
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
}

#[test]
fn a_menu_owner_spoofing_the_shell_comm_is_still_blocked_by_its_modal() {
    let mut owner = Command::new("sh")
        .args([
            "-c",
            "printf gnome-shell > /proc/self/comm; printf ready; read -r line",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut ready = [0; 5];
    owner.stdout.take().unwrap().read_exact(&mut ready).unwrap();
    let comm = std::fs::read_to_string(format!("/proc/{}/comm", owner.id())).unwrap();
    let frame = AtspiElement::new(":1.42", "/parent", owner.id());
    let mut menu = AtspiElement::new(":1.42", "/button", owner.id());
    menu.set_ancestry(ancestry::of_root(&AtspiElement::new(
        ":1.42",
        "/menu",
        owner.id(),
    )));
    menu.set_menu_owner(&frame);
    let handle = menu.into_native_handle();
    let mut desktop = Desktop {
        dialog_states: Field::Present(words(&[MODAL, SHOWING])),
        listed: true,
        unavailable: None,
    };
    let deadline = Deadline::standard().unwrap();
    let blocked = element_via(&mut desktop, &handle, deadline);
    desktop.dialog_states = Field::Present(words(&[]));
    let unblocked = element_via(&mut desktop, &handle, deadline);
    drop(owner.stdin.take());
    owner.wait().unwrap();

    assert_eq!(ready, *b"ready");
    assert_eq!(comm.trim(), "gnome-shell");
    let error = blocked.expect_err("a spoofed process name must not bypass the modal");
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert_eq!(error.details.unwrap()["cause"], "blocked_by_modal_dialog");
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
    assert!(unblocked.is_ok());
}

#[test]
fn live_preflight_refuses_enabled_parent_behind_modal_but_admits_the_dialog_and_closed_control() {
    let mut desktop = Desktop {
        dialog_states: Field::Present(words(&[MODAL, SHOWING])),
        listed: true,
        unavailable: None,
    };
    let deadline = Deadline::standard().unwrap();
    let error = element_via(&mut desktop, &target("/parent"), deadline).unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionFailed);
    assert!(!error.permits_retry_by_default());
    assert_eq!(
        error.disposition,
        agent_desktop_core::DeliverySemantics::not_delivered()
    );
    let details = error.details.unwrap();
    assert_eq!(details["cause"], "blocked_by_modal_dialog");
    assert_eq!(details["dialog"], "Preferences");
    assert_eq!(details["physical_delivery_started"], false);
    assert_eq!(details["semantic_delivery_started"], false);
    assert!(error.suggestion.unwrap().contains("Act in the dialog"));
    desktop.dialog_states = Field::Present(vec![(1 << MODAL) | (1 << SHOWING)]);
    let error = element_via(&mut desktop, &target("/parent"), deadline).unwrap_err();
    assert_eq!(error.details.unwrap()["cause"], "blocked_by_modal_dialog");
    assert!(element_via(&mut desktop, &target("/dialog"), deadline).is_ok());
    for bits in [vec![MODAL], vec![SHOWING], vec![]] {
        desktop.dialog_states = Field::Present(words(&bits));
        assert!(element_via(&mut desktop, &target("/parent"), deadline).is_ok());
    }
}

#[test]
fn live_preflight_refuses_when_modal_state_or_target_window_membership_is_unreadable() {
    let deadline = Deadline::standard().unwrap();
    for states in [
        Field::Absent,
        Field::Failed("unreadable".into()),
        Field::Present(vec![]),
    ] {
        let mut desktop = Desktop {
            dialog_states: states,
            listed: true,
            unavailable: None,
        };
        let error = element_via(&mut desktop, &target("/parent"), deadline).unwrap_err();
        assert!(!error.permits_retry_by_default());
        assert_eq!(error.details.unwrap()["cause"], "modal_state_unknown");
        assert_eq!(
            error.disposition,
            agent_desktop_core::DeliverySemantics::not_delivered()
        );
    }
    let mut desktop = Desktop {
        dialog_states: Field::Present(words(&[])),
        listed: false,
        unavailable: None,
    };
    let error = element_via(&mut desktop, &target("/parent"), deadline).unwrap_err();
    assert_eq!(error.details.unwrap()["cause"], "modal_state_unknown");
    for unavailable in ["listing", "/dialog"] {
        desktop.listed = true;
        desktop.unavailable = Some(unavailable);
        let error = element_via(&mut desktop, &target("/parent"), deadline).unwrap_err();
        assert!(!error.permits_retry_by_default());
        assert_eq!(error.details.unwrap()["cause"], "modal_state_unknown");
        assert!(
            error
                .suggestion
                .unwrap()
                .contains("act in the dialog or close it")
        );
        assert_eq!(
            error.disposition,
            agent_desktop_core::DeliverySemantics::not_delivered()
        );
    }
}
