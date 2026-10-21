use super::*;
use crate::system::app_outcome::AppOutcome;
use crate::system::launch::choose_window;
use crate::system::process_facts::ProcessFacts;
use crate::system::registry_app::assemble;
use crate::system::signal_inventory::pass_over;
use crate::system::toolkit_id::ToolkitId;
use crate::system::x11::X11State;
use crate::tree::AtspiElement;

#[test]
fn an_old_frame_omitted_by_an_incomplete_baseline_cannot_be_a_launch_window() {
    let owner = Running {
        executable: None,
        pid: 5,
        instance: "boot:9".to_owned(),
        name: "tool".to_owned(),
    };
    let registry = Registry {
        apps: vec![assemble(
            AtspiElement::new(":1.5", "/root", owner.pid),
            ProcessFacts {
                executable: None,
                comm: "tool".to_owned(),
                exe_basename: Some("tool".to_owned()),
                instance: owner.instance.clone(),
                web_family: None,
            },
            Some(Some(owner.name.clone())),
            ToolkitId::default(),
        )],
        skipped: Vec::new(),
    };
    let mut pass = pass_over(
        Vec::new(),
        vec![AppOutcome {
            app: 0,
            read: RootRead::TimedOut,
        }],
        Vec::new(),
        &X11State::Unavailable,
    );
    let old = agent_desktop_core::WindowInfo {
        id: "w-old".to_owned(),
        title: "old".to_owned(),
        app: owner.name.clone(),
        pid: owner.pid.into(),
        process_instance: Some(owner.instance.clone()),
        bounds: None,
        state: Default::default(),
    };
    let selected =
        ids_of(&registry, &pass, &owner).map(|known| choose_window(vec![old.clone()], &known));
    assert_eq!(
        selected
            .expect_err("partial baseline must refuse launch")
            .code,
        agent_desktop_core::ErrorCode::Timeout
    );

    pass.outcomes[0].read = RootRead::Read;
    assert!(
        ids_of(&registry, &pass, &owner)
            .expect("complete empty baseline")
            .is_empty()
    );
    pass.outcomes.clear();
    assert!(
        ids_of(&registry, &pass, &owner).is_err(),
        "an unread owner is incomplete"
    );
    pass.recovered.push(0);
    pass.windows
        .push(crate::system::listed_window::ListedWindow {
            app: 0,
            info: agent_desktop_core::WindowInfo {
                id: "w-1.5-stopped-x42".to_owned(),
                state: agent_desktop_core::WindowState {
                    accessible: false,
                    ..Default::default()
                },
                ..old
            },
            active: false,
            modal: false,
            dialog: false,
            frame: crate::system::x11::FrameFacts {
                size_identity: false,
                pid: owner.pid,
                title: "old".to_owned(),
                extents: None,
                global_position: false,
            },
            sheet: false,
            unshown: false,
        });
    assert!(
        ids_of(&registry, &pass, &owner).is_err(),
        "X11 recovery cannot prove the old windows' AT-SPI identities"
    );
}
