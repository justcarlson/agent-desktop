use super::*;

#[test]
fn normalized_typed_text_is_reported_as_delivered_unverified() {
    for (before, requested, normalized, selection) in [
        ("", "(7+8)*9", "(7+8)×9", None),
        ("", "200*10%", "200×10%", None),
        ("×", "*", "×", Some(0..1)),
    ] {
        let mut adapter = adapter(
            element(Some(before), &["focused"]),
            element(Some(normalized), &[]),
        );
        adapter.selection = selection;
        adapter.execution = Ok(ActionResult::delivered_unverified("type").with_steps(vec![
            ActionStep::succeeded("XTEST.type_text")
                .with_mechanism(crate::StepMechanism::PhysicalSynthetic)
                .with_verified(false),
        ]));
        let result = execute_verified_action(
            &adapter,
            &NativeHandle::null(),
            ActionRequest::headed(Action::TypeText(requested.into())),
            &InteractionLease::guarded(Deadline::after(1000).unwrap(), ()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            result.disposition(),
            DeliverySemantics::delivered_unverified()
        );
        assert_eq!(
            result.details.as_ref().unwrap()["verification_reason"],
            "postcondition_mismatch"
        );
        assert_eq!(
            result.post_state.unwrap().value.as_deref(),
            Some(normalized)
        );
        assert_eq!(adapter.calls.load(Ordering::SeqCst), 1);
    }
}
