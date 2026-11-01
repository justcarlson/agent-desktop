use agent_desktop_core::{CursorOverlayControl, CursorOverlayStyle, ErrorCode, Point};

use super::{AgentCursors, SOLE_CURSOR};

fn enable(agent: Option<&str>) -> CursorOverlayControl {
    CursorOverlayControl::enable("s".into(), CursorOverlayStyle::default())
        .with_agent_id(agent.map(str::to_owned))
}

#[test]
fn a_named_agent_keys_its_own_cursor_without_asking_the_session() {
    let key = AgentCursors::key_for(&enable(Some("agent-a")), || panic!("not asked"));
    assert_eq!(key.expect("named"), "agent-a");
}

#[test]
fn an_unnamed_control_is_the_sole_cursor_outside_a_multi_agent_session() {
    let key = AgentCursors::key_for(&enable(None), || false);
    assert_eq!(key.expect("sole"), SOLE_CURSOR);
}

#[test]
fn an_unnamed_control_in_a_multi_agent_session_is_refused() {
    let refused = AgentCursors::key_for(&enable(None), || true).expect_err("refused");
    assert_eq!(refused.code, ErrorCode::InvalidArgs);
    assert!(refused.suggestion.is_some());
}

#[test]
fn a_new_agent_cursor_starts_at_home_and_keeps_its_own_style() {
    let mut cursors = AgentCursors::new(Point { x: 960.0, y: 540.0 });
    let mut large = CursorOverlayStyle::default();
    large.set_size(2.0);
    let big = CursorOverlayControl::enable("s".into(), large.clone())
        .with_agent_id(Some("agent-b".into()));
    cursors.cursor("agent-a", &enable(Some("agent-a")));
    cursors.cursor("agent-b", &big);
    assert_eq!(cursors.len(), 2);
    assert_eq!(cursors.agents(), ["agent-a", "agent-b"]);
    let b = cursors.get_mut("agent-b").expect("b");
    assert_eq!(b.pose, Some(Point { x: 960.0, y: 540.0 }));
    assert_eq!(b.style, large);
    let a = cursors.get_mut("agent-a").expect("a");
    assert_eq!(a.style, CursorOverlayStyle::default());
    assert!(!cursors.travelling());
}
