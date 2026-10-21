use zbus::connection::Builder;
use zbus::zvariant::OwnedObjectPath;

use super::focused;
use crate::tree::AtspiElement;
use crate::tree::state_bits::{EDITABLE, FOCUSED};

const OWNER: &str = "org.agentdesktop.FocusFixture";

struct Accessible {
    states: u32,
    interfaces: Vec<&'static str>,
    parent: &'static str,
}

#[zbus::interface(name = "org.a11y.atspi.Accessible")]
impl Accessible {
    fn get_state(&self) -> Vec<u32> {
        vec![self.states, 0]
    }

    fn get_interfaces(&self) -> Vec<&str> {
        self.interfaces.clone()
    }

    #[zbus(property)]
    fn parent(&self) -> (String, OwnedObjectPath) {
        (
            OWNER.to_owned(),
            self.parent.try_into().expect("parent path"),
        )
    }
}

struct Component {
    hit: &'static str,
    owner: &'static str,
}

#[zbus::interface(name = "org.a11y.atspi.Component")]
impl Component {
    fn get_extents(&self, _coordinates: u32) -> (i32, i32, i32, i32) {
        (9, 0, 355, 34)
    }

    fn get_accessible_at_point(
        &self,
        _x: i32,
        _y: i32,
        _coordinates: u32,
    ) -> (String, OwnedObjectPath) {
        (
            self.owner.to_owned(),
            self.hit.try_into().expect("hit path"),
        )
    }
}

#[test]
fn focused_internal_editor_is_proven_only_under_its_entry() {
    for (entry_focused, editor_focused, editable, parent, hit_owner, hit, expected) in [
        (false, true, true, "/entry", OWNER, "/placeholder", true),
        (false, true, true, "/entry", OWNER, "/editor", true),
        (false, false, true, "/entry", OWNER, "/placeholder", false),
        (false, true, true, "/other", OWNER, "/placeholder", false),
        (false, true, false, "/entry", OWNER, "/editor", false),
        (
            false,
            true,
            true,
            "/entry",
            "org.agentdesktop.Other",
            "/editor",
            false,
        ),
        (false, true, true, "/entry", OWNER, "/missing", false),
        (true, false, false, "/entry", OWNER, "/missing", true),
    ] {
        let (near, far) = std::os::unix::net::UnixStream::pair().expect("socket pair");
        let (client, server) = zbus::block_on(async {
            futures_lite::future::zip(
                Builder::async_io_unix_stream(near).p2p().build(),
                Builder::async_io_unix_stream(far)
                    .server(zbus::Guid::generate())
                    .expect("guid")
                    .p2p()
                    .serve_at(
                        "/entry",
                        Accessible {
                            states: (1 << EDITABLE) | if entry_focused { 1 << FOCUSED } else { 0 },
                            interfaces: vec![],
                            parent: "/frame",
                        },
                    )
                    .expect("entry")
                    .serve_at(
                        "/entry",
                        Component {
                            hit,
                            owner: hit_owner,
                        },
                    )
                    .expect("component")
                    .serve_at(
                        "/placeholder",
                        Accessible {
                            states: 0,
                            interfaces: vec!["org.a11y.atspi.Text"],
                            parent: "/editor",
                        },
                    )
                    .expect("placeholder")
                    .serve_at(
                        "/editor",
                        Accessible {
                            states: if editor_focused { 1 << FOCUSED } else { 0 },
                            interfaces: if editable {
                                vec!["org.a11y.atspi.Text", "org.a11y.atspi.EditableText"]
                            } else {
                                vec!["org.a11y.atspi.Text"]
                            },
                            parent,
                        },
                    )
                    .expect("editor")
                    .build(),
            )
            .await
        });
        let _server = server.expect("server");
        let client = client.expect("client");
        let entry = AtspiElement::new(OWNER, "/entry", 1);
        let actual = zbus::block_on(focused(&client, &entry))
            .expect("a readable entry retains the ordinary focus-acquisition path");
        assert_eq!(
            actual, expected,
            "entry_focused={entry_focused}, editor_focused={editor_focused}, editable={editable}, parent={parent}, owner={hit_owner}"
        );
    }
}
