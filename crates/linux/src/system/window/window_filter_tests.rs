use super::*;
use crate::tree::test_support::words;

fn component() -> Field<Vec<String>> {
    Field::Present(vec![COMPONENT_INTERFACE.to_owned()])
}

fn no_states() -> Field<Vec<u32>> {
    Field::Present(words(&[]))
}

fn modal() -> Field<Vec<u32>> {
    Field::Present(words(&[16]))
}

#[test]
fn a_modal_panel_is_a_window_and_a_plain_panel_is_not() {
    let panel = Field::Present(39);
    assert!(is_window_shape(&panel, &component(), &modal()));
    assert!(!is_window_shape(&panel, &component(), &no_states()));
    assert!(!is_window_shape(
        &panel,
        &component(),
        &Field::Failed("x".into())
    ));
    assert!(!is_window_shape(&panel, &Field::Present(vec![]), &modal()));
}

#[test]
fn other_roles_and_frames_without_a_component_are_not() {
    let none = no_states();
    assert!(!is_window_shape(
        &Field::Present(75),
        &component(),
        &modal()
    ));
    assert!(!is_window_shape(
        &Field::Present(23),
        &Field::Present(vec![]),
        &none
    ));
    assert!(!is_window_shape(
        &Field::Present(23),
        &Field::Failed("x".into()),
        &none
    ));
    assert!(!is_window_shape(
        &Field::Failed("x".into()),
        &component(),
        &none
    ));
}

#[test]
fn state_bits_become_visible_minimized_and_active() {
    let shown = frame_state(&Field::Present(words(&[25, 1])));
    assert_eq!(
        shown,
        FrameState {
            visible: Some(true),
            minimized: Some(false),
            active: Some(true),
            modal: Some(false)
        }
    );
    let hidden = frame_state(&Field::Present(words(&[15])));
    assert_eq!(hidden.visible, Some(false));
    assert_eq!(hidden.minimized, Some(true));
}

#[test]
fn a_showing_visible_filler_is_a_window_and_a_hidden_one_is_not() {
    let filler = Field::Present(20);
    let shown = Field::Present(words(&[25, 30]));
    assert!(is_window_shape(&filler, &component(), &shown));
    assert!(!is_window_shape(&filler, &component(), &no_states()));
    assert!(!is_window_shape(
        &filler,
        &component(),
        &Field::Present(words(&[25]))
    ));
    assert!(!is_window_shape(&filler, &Field::Present(vec![]), &shown));
}

#[test]
fn native_window_types_require_complete_uniform_process_or_geometry_groups() {
    use crate::system::x11::{FrameFacts, X11Client};
    use crate::tree::Extents;
    let bounds = Extents {
        x: 31,
        y: 47,
        width: 300,
        height: 200,
    };
    let frame = FrameFacts {
        pid: 7,
        title: "arbitrary title".into(),
        extents: Some(bounds),
        global_position: true,
        size_identity: false,
    };
    let client = X11Client {
        id: 20,
        pid: Some(7),
        title: "unrelated title".into(),
        bounds: Some(bounds),
        extents: None,
        hidden: false,
        shadow: (0, 0, 0, 0),
    };
    let users = std::collections::HashSet::from([20, 21]);
    let helpers = std::collections::HashSet::new();
    let classify = |frames: &[FrameFacts], clients: &[X11Client]| {
        native_group(&frame, frames, clients, &users, &helpers, true)
    };
    let one = std::slice::from_ref(&frame);
    assert!(classify(one, std::slice::from_ref(&client)).unwrap());
    assert!(classify(one, &[]).is_err());
    let wrong = X11Client {
        bounds: Some(Extents { x: 99, ..bounds }),
        ..client.clone()
    };
    assert!(classify(one, std::slice::from_ref(&wrong)).unwrap());
    assert!(
        native_group(
            &frame,
            one,
            std::slice::from_ref(&wrong),
            &users,
            &helpers,
            false
        )
        .is_err()
    );
    assert!(
        classify(
            one,
            &[X11Client {
                pid: Some(8),
                ..wrong
            }]
        )
        .is_err()
    );
    let twins = [frame.clone(), frame.clone()];
    let natives = [client.clone(), X11Client { id: 21, ..client }];
    assert!(classify(&twins, &natives).unwrap());
    assert!(native_group(&frame, &twins, &natives, &users, &helpers, false).is_err());
    assert!(classify(one, &natives).is_err());
    let separated = [
        natives[0].clone(),
        X11Client {
            bounds: Some(Extents {
                x: 999,
                width: 100,
                ..bounds
            }),
            ..natives[1].clone()
        },
    ];
    assert!(
        native_group(
            &frame,
            one,
            &separated,
            &std::collections::HashSet::from([20]),
            &std::collections::HashSet::from([21]),
            true
        )
        .is_err()
    );
    assert!(classify(&twins, &natives[..1]).is_err());
    assert!(
        native_group(
            &frame,
            &twins,
            &natives,
            &std::collections::HashSet::from([20]),
            &std::collections::HashSet::from([21]),
            true,
        )
        .is_err()
    );
    assert!(!native_group(&frame, &twins, &natives, &helpers, &users, true).unwrap());
}
