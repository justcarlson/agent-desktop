use super::{RelationSet, read_label};
use crate::tree::{AtspiElement, LabelOutcome};
use zbus::connection::Builder;

const OWNER: &str = "org.agentdesktop.LabelFixture";

struct Accessible;

#[zbus::interface(name = "org.a11y.atspi.Accessible")]
impl Accessible {
    fn get_relation_set(&self) -> RelationSet {
        vec![(2, vec![(OWNER.to_owned(), "/label".try_into().unwrap())])]
    }

    #[zbus(property)]
    fn name(&self) -> &str {
        ""
    }
}

struct Text(Option<&'static str>);

#[zbus::interface(name = "org.a11y.atspi.Text")]
impl Text {
    fn get_text(&self, _start: i32, _end: i32) -> zbus::fdo::Result<String> {
        self.0
            .map(str::to_owned)
            .ok_or_else(|| zbus::fdo::Error::Failed("unreadable label".to_owned()))
    }
}

struct MalformedText;

#[zbus::interface(name = "org.a11y.atspi.Text")]
impl MalformedText {
    fn get_text(&self, _start: i32, _end: i32) -> u32 {
        7
    }
}

#[test]
fn label_text_transport_failures_preserve_name_uncertainty() {
    for (text, malformed, expected) in [
        (None, false, LabelOutcome::Failed),
        (None, true, LabelOutcome::Failed),
        (Some("  "), false, LabelOutcome::Unlabelled),
        (Some("Open"), false, LabelOutcome::Text("Open".to_owned())),
    ] {
        let (near, far) = std::os::unix::net::UnixStream::pair().unwrap();
        let server = Builder::async_io_unix_stream(far)
            .server(zbus::Guid::generate())
            .unwrap()
            .p2p()
            .serve_at("/button", Accessible)
            .unwrap()
            .serve_at("/label", Accessible)
            .unwrap();
        let server = if malformed {
            server.serve_at("/label", MalformedText).unwrap()
        } else {
            server.serve_at("/label", Text(text)).unwrap()
        };
        let (client, server) = zbus::block_on(futures_lite::future::zip(
            Builder::async_io_unix_stream(near).p2p().build(),
            server.build(),
        ));
        let _server = server.unwrap();
        let label = zbus::block_on(read_label(
            &client.unwrap(),
            &AtspiElement::new(OWNER, "/button", 1),
        ));
        assert_eq!(label, expected, "text={text:?} malformed={malformed}");
        let mut read = crate::tree::test_support::read_of(
            crate::tree::role_numbers::PUSH_BUTTON,
            "",
            &["click"],
        );
        read.label = label;
        let projected = crate::tree::projection::project_node(
            &read,
            false,
            &crate::tree::WalkLimits::default(),
        );
        assert_eq!(
            projected.name.is_unknown(),
            expected == LabelOutcome::Failed
        );
    }
}
