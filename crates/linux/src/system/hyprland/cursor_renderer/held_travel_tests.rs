use std::os::unix::net::UnixStream;

use agent_desktop_core::{CursorOverlayControl, CursorOverlayStyle, Deadline, Point};

use super::step_unseen;
use crate::system::hyprland::cursor_renderer::agent_cursors::AgentCursors;
use crate::system::hyprland::cursor_renderer::cursor_travel::CursorTravel;

fn point(x: f64, y: f64) -> Point {
    Point { x, y }
}

fn travelling(cursors: &mut AgentCursors, key: &str, from: Option<Point>, to: Point) -> UnixStream {
    let enable = CursorOverlayControl::enable("s".into(), CursorOverlayStyle::default());
    let cursor = cursors.cursor(key, &enable);
    cursor.pose = from;
    let (reply, kept) = UnixStream::pair().expect("pair");
    let budget = Deadline::detached_after(5_000).expect("budget");
    cursor.travel = Some(CursorTravel::begin(cursor, to, false, budget, reply));
    kept
}

#[test]
fn an_unseen_travel_lands_and_is_acknowledged_at_its_arrival_time() {
    let mut cursors = AgentCursors::new(point(0.0, 0.0));
    let _waiting = travelling(&mut cursors, "a", None, point(40.0, 50.0));
    let arrived = step_unseen(&mut cursors);
    assert_eq!(arrived.len(), 1);
    let cursor = cursors.get_mut("a").expect("cursor");
    assert!(cursor.travel.is_none());
    assert_eq!(cursor.pose, Some(point(40.0, 50.0)));
}

#[test]
fn an_unseen_travel_in_flight_keeps_its_schedule_and_its_connection() {
    let mut cursors = AgentCursors::new(point(0.0, 0.0));
    let _waiting = travelling(
        &mut cursors,
        "b",
        Some(point(0.0, 0.0)),
        point(900.0, 700.0),
    );
    assert!(step_unseen(&mut cursors).is_empty());
    let cursor = cursors.get_mut("b").expect("cursor");
    assert!(cursor.travel.is_some());
    assert_ne!(cursor.pose, Some(point(900.0, 700.0)));
}
