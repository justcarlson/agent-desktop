use agent_desktop_core::{ErrorCode, InteractionPolicy, Point};

use super::{ClickSpec, click_steps};
use crate::actions::physical_target::tests::fake::{
    FakeWorld, check_of, deadline, element, not_delivered,
};

#[test]
fn every_click_revalidates_and_a_late_refusal_preserves_delivery() {
    use crate::input::x11::fake_sink::{FakeSink, Recorded};
    use agent_desktop_core::{AdapterError, DeliverySemantics};
    use serde_json::json;

    for count in [2, 3] {
        for details in [
            json!({"check": "receives_events", "interceptor": {"role": "button", "name": "Cover", "bounds": null}}),
            json!({"kind": "pointer_hit_unproven", "hit_test": "unknown"}),
        ] {
            for allowed in (1..=count).chain(std::iter::once(0)) {
                let mut world = FakeWorld::x11();
                let sink = FakeSink::new();
                world.sink = Some(sink.clone());
                world
                    .pointer_reads
                    .borrow_mut()
                    .extend((0..allowed).map(|_| Ok(())));
                world
                    .pointer_reads
                    .borrow_mut()
                    .push_back(Err(AdapterError::new(
                        ErrorCode::ActionFailed,
                        "receiver changed",
                    )
                    .with_details(details.clone())
                    .with_disposition(DeliverySemantics::not_delivered())));
                let result = click_steps(
                    &world,
                    &element(),
                    ClickSpec::left(count, None),
                    InteractionPolicy::headed(),
                    deadline(),
                );
                let sent = sink
                    .recorded()
                    .iter()
                    .filter(|event| matches!(event, Recorded::Button(1, true)))
                    .count();
                assert_eq!(
                    sent, allowed as usize,
                    "requested {count}, allowed {allowed}"
                );
                if allowed == count {
                    result.expect("every click receiver was proven");
                } else {
                    let error = result.expect_err("receiver refusal stops the gesture");
                    assert_eq!(
                        error.disposition,
                        if allowed == 0 {
                            DeliverySemantics::not_delivered()
                        } else {
                            DeliverySemantics::delivered_unverified()
                        }
                    );
                    let observed = error.details.expect("refusal evidence");
                    assert_eq!(observed["delivered_clicks"], allowed);
                    assert_eq!(observed["physical_delivery_started"], allowed > 0);
                    for (key, value) in details.as_object().unwrap() {
                        assert_eq!(&observed[key], value);
                    }
                }
            }
        }
    }
}

#[test]
fn a_click_without_a_verified_point_is_aimed_at_the_centre_of_the_bounds() {
    let world = FakeWorld::x11();
    click_steps(
        &world,
        &element(),
        ClickSpec::left(1, None),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap();
    assert!(
        world
            .entries()
            .contains(&"plan_click 120,210 Left 1".to_owned())
    );
    assert_eq!(world.count("sent"), 1);
}

#[test]
fn a_verified_point_is_used_and_the_bounds_are_not_read() {
    let world = FakeWorld::x11();
    let point = Point { x: 7.0, y: 9.0 };
    click_steps(
        &world,
        &element(),
        ClickSpec::right(Some(point)),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap();
    assert!(
        world
            .entries()
            .contains(&"plan_click 7,9 Right 1".to_owned())
    );
    assert_eq!(world.count("bounds"), 0);
}

#[test]
fn bounds_that_cannot_be_established_refuse_with_no_event() {
    for bounds in [
        None,
        Some(agent_desktop_core::Rect {
            x: 5.0,
            y: 5.0,
            width: 0.0,
            height: 10.0,
        }),
    ] {
        let world = FakeWorld::x11().with_bounds(bounds);
        let error = click_steps(
            &world,
            &element(),
            ClickSpec::left(1, None),
            InteractionPolicy::headed(),
            deadline(),
        )
        .unwrap_err();
        assert_eq!(check_of(&error).as_deref(), Some("geometry"));
        assert!(not_delivered(&error));
        assert!(world.sent_nothing());
    }
}

#[test]
fn focus_only_policy_denies_a_click() {
    let world = FakeWorld::x11();
    let error = click_steps(
        &world,
        &element(),
        ClickSpec::left(1, None),
        InteractionPolicy::focus_fallback(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::PolicyDenied);
    assert!(world.sent_nothing());
}

#[test]
fn a_click_does_not_wait_for_the_frame_to_become_active() {
    let world = FakeWorld::x11().active_script(&[Some(false)]);
    click_steps(
        &world,
        &element(),
        ClickSpec::left(1, None),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap();
    assert_eq!(world.count("read_active"), 0);
    assert_eq!(world.count("sent"), 1);
}

#[test]
fn on_wayland_a_click_is_refused_before_anything_is_computed() {
    let world = FakeWorld::wayland();
    let error = click_steps(
        &world,
        &element(),
        ClickSpec::left(1, None),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::ActionNotSupported);
    assert_eq!(error.message, "pointer_refusal click");
    assert_eq!(world.count("bounds"), 0);
    assert_eq!(world.count("plan_click"), 0);
    assert!(world.sent_nothing());
}

#[test]
fn the_step_is_a_physical_unverified_click() {
    let world = FakeWorld::x11();
    let steps = click_steps(
        &world,
        &element(),
        ClickSpec::left(1, None),
        InteractionPolicy::headed(),
        deadline(),
    )
    .unwrap();
    assert_eq!(steps[0].label(), "XTEST.click");
    assert_eq!(steps[0].verified(), Some(false));
}

#[test]
fn a_clipped_sidebar_row_never_receives_a_click_outside_its_viewport() {
    use crate::tree::{Extents, ancestry::Ancestry, placement::Placement};

    let mut target = element();
    target.set_ancestry(Ancestry::of(Some(element()), Some(element()), false));
    for viewport_height in [105, 99] {
        for point in [None, Some(Point { x: 120.0, y: 210.0 })] {
            let mut world = FakeWorld::x11();
            world.clip = Some(Placement {
                x11: Some(crate::system::x11::window_x11_state::WindowX11State {
                    client_origin: (100, 100),
                    client_id: 7,
                    shadow: (0, 0, 0, 0),
                    hidden: false,
                }),
                node: Some(Extents {
                    x: 0,
                    y: 100,
                    width: 40,
                    height: 20,
                }),
                frame: Some(Extents {
                    x: 0,
                    y: 0,
                    width: 500,
                    height: 500,
                }),
                container: Some(Extents {
                    x: 0,
                    y: 0,
                    width: 200,
                    height: viewport_height,
                }),
                ..Placement::default()
            });
            let error = click_steps(
                &world,
                &target,
                ClickSpec::left(1, point),
                InteractionPolicy::headed(),
                deadline(),
            )
            .unwrap_err();
            assert_eq!(error.code, ErrorCode::StaleRef);
            assert_eq!(error.details.unwrap()["kind"], "pointer_target_not_visible");
            assert!(world.sent_nothing());

            if viewport_height == 105 {
                click_steps(
                    &world,
                    &target,
                    ClickSpec::left(1, Some(Point { x: 120.0, y: 202.0 })),
                    InteractionPolicy::headed(),
                    deadline(),
                )
                .unwrap();
                assert_eq!(world.count("sent"), 1);
            }
        }
    }
}
