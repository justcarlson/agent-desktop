use agent_desktop_core::{OperationReport, OperationTarget, WindowOp};

pub(crate) fn window_report(target: OperationTarget, op: &WindowOp) -> OperationReport {
    OperationReport::window_operation(target, op, !matches!(op, WindowOp::Maximize))
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_desktop_core::{DeliverySemantics, ProcessIdentity};

    fn target() -> OperationTarget {
        OperationTarget::Window {
            id: "w-1".into(),
            title: "Doc".into(),
            process: ProcessIdentity::new(7u32, "boot"),
        }
    }

    #[test]
    fn maximize_stays_unverified_because_position_and_size_are_read_separately() {
        let maximize = window_report(target(), &WindowOp::Maximize);

        assert_eq!(
            maximize.disposition(),
            DeliverySemantics::delivered_unverified()
        );
    }

    #[test]
    fn operations_with_a_single_readback_are_verified() {
        for op in [
            WindowOp::Move { x: 1.0, y: 2.0 },
            WindowOp::Resize {
                width: 3.0,
                height: 4.0,
            },
            WindowOp::Minimize,
            WindowOp::Restore,
        ] {
            assert_eq!(
                window_report(target(), &op).disposition(),
                DeliverySemantics::delivered_verified()
            );
        }
    }
}
