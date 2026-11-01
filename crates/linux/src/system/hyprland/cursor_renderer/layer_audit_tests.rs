use agent_desktop_core::Rect;

use super::audit_cursor_layers;
use crate::system::hyprland::cursor_renderer::hide_refusal::HideRefusal;
use crate::system::hyprland::receiver::layer_info::LayerInfo;

fn layer(namespace: &str, pid: Option<u32>) -> LayerInfo {
    LayerInfo {
        namespace: namespace.to_owned(),
        pid,
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 10.0,
            height: 10.0,
        },
        level: 3,
        output: "DP-1".into(),
    }
}

#[test]
fn acknowledged_cursor_layers_and_other_namespaces_pass() {
    let layers = [
        layer("agent-desktop-cursor", Some(10)),
        layer("waybar", Some(99)),
    ];
    let mut acked = vec![10];
    let mut late = |_| panic!("no late renderer");
    assert_eq!(audit_cursor_layers(&layers, &mut acked, &mut late), Ok(()));
}

#[test]
fn a_renderer_that_started_after_the_hide_is_hidden_now() {
    let layers = [layer("agent-desktop-cursor", Some(11))];
    let mut acked = Vec::new();
    let mut asked = Vec::new();
    let mut late = |pid| {
        asked.push(pid);
        Ok(true)
    };
    assert_eq!(audit_cursor_layers(&layers, &mut acked, &mut late), Ok(()));
    assert_eq!(asked, [11]);
    assert_eq!(acked, [11]);
}

#[test]
fn a_foreign_layer_with_the_cursor_namespace_refuses() {
    for pid in [Some(12), None] {
        let layers = [layer("agent-desktop-cursor", pid)];
        let refused = audit_cursor_layers(&layers, &mut Vec::new(), &mut |_| Ok(false));
        assert_eq!(
            refused,
            Err(HideRefusal::UnknownOverlay {
                pid,
                output: "DP-1".into()
            })
        );
    }
}
