use super::*;

#[test]
fn the_portal_is_reached_on_the_session_bus_connection() {
    let session: BusGetter = SessionBus::get;
    assert!(std::ptr::fn_addr_eq(PORTAL_BUS, session));
}
