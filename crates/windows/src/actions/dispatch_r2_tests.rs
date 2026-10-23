use super::{ClickAvailability, click_chain_judged_for};
use crate::actions::chain::DeliveryOutcome;
use crate::system::test_time::deadline;
use crate::tree::actions::resolve_actions;
use crate::tree::properties::ElementProperties;
use crate::tree::property_ids::TreeProperty;
use crate::tree::property_outcome::{PropertyOutcome, PropertyValue};
use agent_desktop_core::{ActionStepOutcome, InteractionPolicy, capability};
use std::cell::Cell;

fn known_flag(value: bool) -> PropertyOutcome {
    PropertyOutcome::Known(PropertyValue::Flag(value))
}

fn known_text(value: &str) -> PropertyOutcome {
    PropertyOutcome::Known(PropertyValue::Text(value.to_string()))
}

fn inert_reads() -> Vec<(TreeProperty, PropertyOutcome)> {
    vec![
        (TreeProperty::InvokeAvailable, known_flag(false)),
        (TreeProperty::ToggleAvailable, known_flag(false)),
        (TreeProperty::ExpandCollapseAvailable, known_flag(false)),
        (TreeProperty::SelectionItemAvailable, known_flag(false)),
        (TreeProperty::ValueAvailable, known_flag(false)),
        (TreeProperty::RangeValueAvailable, known_flag(false)),
        (TreeProperty::ScrollAvailable, known_flag(false)),
        (TreeProperty::ScrollItemAvailable, known_flag(false)),
        (TreeProperty::IsKeyboardFocusable, known_flag(false)),
        (TreeProperty::LegacyDefaultAction, PropertyOutcome::Absent),
    ]
}

#[test]
fn r2_invoke_advertisement_reaches_click_rung() {
    let mut reads = inert_reads();
    reads.retain(|(property, _)| *property != TreeProperty::InvokeAvailable);
    reads.push((TreeProperty::InvokeAvailable, known_flag(true)));
    let actions = resolve_actions(&ElementProperties::from_reads(reads));
    let known = actions.known().expect("Known actions");
    assert!(known.iter().any(|action| action == capability::CLICK));

    let invoke = Cell::new(0u8);
    let legacy = Cell::new(0u8);
    let steps = click_chain_judged_for(
        deadline(5_000),
        InteractionPolicy::headless(),
        ClickAvailability {
            invoke_available: true,
            legacy_available: false,
        },
        || {
            invoke.set(invoke.get() + 1);
            Ok(DeliveryOutcome::DeliveredUnverified)
        },
        || {
            legacy.set(legacy.get() + 1);
            Ok(DeliveryOutcome::DeliveredUnverified)
        },
    )
    .expect("invoke rung");
    assert_eq!(invoke.get(), 1);
    assert_eq!(legacy.get(), 0);
    assert!(matches!(steps[0].outcome, ActionStepOutcome::Succeeded));
}

#[test]
fn r2_legacy_advertisement_reaches_legacy_rung() {
    let mut reads = inert_reads();
    reads.retain(|(property, _)| *property != TreeProperty::LegacyDefaultAction);
    reads.push((TreeProperty::LegacyDefaultAction, known_text("Press")));
    let actions = resolve_actions(&ElementProperties::from_reads(reads));
    let known = actions.known().expect("Known actions");
    assert!(known.iter().any(|action| action == capability::CLICK));

    let invoke = Cell::new(0u8);
    let legacy = Cell::new(0u8);
    let steps = click_chain_judged_for(
        deadline(5_000),
        InteractionPolicy::headless(),
        ClickAvailability {
            invoke_available: false,
            legacy_available: true,
        },
        || {
            invoke.set(invoke.get() + 1);
            Ok(DeliveryOutcome::DeliveredUnverified)
        },
        || {
            legacy.set(legacy.get() + 1);
            Ok(DeliveryOutcome::DeliveredUnverified)
        },
    )
    .expect("legacy rung");
    assert_eq!(invoke.get(), 0);
    assert_eq!(legacy.get(), 1);
    assert_eq!(steps[1].label(), "LegacyIAccessible.DoDefaultAction");
}
