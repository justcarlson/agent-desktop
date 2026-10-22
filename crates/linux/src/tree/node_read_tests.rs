mod classification {
    use agent_desktop_core::ErrorCode;

    use super::super::{field, read_children};
    use crate::tree::Field;
    use crate::tree::absence::{UNKNOWN_INTERFACE, UNKNOWN_METHOD, UNKNOWN_OBJECT};
    use crate::tree::liveness::Liveness;
    use crate::tree::test_support::{client_timeout, method_error};

    #[test]
    fn a_suspect_name_on_a_node_that_answers_stays_on_its_field() {
        for name in [UNKNOWN_OBJECT, UNKNOWN_METHOD, UNKNOWN_INTERFACE] {
            let read: Result<u32, zbus::Error> = Err(method_error(name));
            let outcome = field(read, Liveness::Alive).expect("a live node's read does not fail");
            assert!(outcome.is_failed(), "{name}");
        }
    }

    #[test]
    fn a_suspect_name_on_a_node_whose_role_failed_the_same_way_ends_the_read() {
        let read: Result<u32, zbus::Error> = Err(method_error(UNKNOWN_OBJECT));
        let error = field(read, Liveness::ObjectGone).expect_err("a gone object fails the read");
        assert_eq!(error.code, ErrorCode::ElementNotFound);
    }

    #[test]
    fn an_application_that_left_the_bus_ends_the_read_with_the_walks_present_answer() {
        let read: Result<u32, zbus::Error> = Err(method_error(crate::system::SERVICE_UNKNOWN));
        let error = field(read, Liveness::Alive).expect_err("an exited app fails the read");
        assert_eq!(error.code, ErrorCode::AppNotFound);
    }

    #[test]
    fn a_client_timeout_is_a_failed_field_and_never_an_absence() {
        let read: Result<u32, zbus::Error> = Err(client_timeout());
        let outcome = field(read, Liveness::TimedOut).expect("a timeout keeps the node");
        assert!(matches!(outcome, Field::Failed(_)));
    }

    #[test]
    fn a_child_list_that_fails_on_a_live_node_is_a_failed_field() {
        let outcome = read_children(Err(method_error(UNKNOWN_METHOD)), Liveness::Alive)
            .expect("a live node's child list does not end the read");
        assert!(outcome.is_failed());
    }
}
