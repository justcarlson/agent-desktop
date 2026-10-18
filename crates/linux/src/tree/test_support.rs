use std::collections::BTreeMap;

use super::{AtspiElement, Extents, Field, NodeProperties, NodeTraits, ObservedRead, RawNode};

pub(crate) fn attributes(pairs: &[(&str, &str)]) -> Field<BTreeMap<String, String>> {
    Field::Present(
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
    )
}

pub(crate) fn words(bits: &[u32]) -> Vec<u32> {
    let mut words = vec![0u32; 2];
    for bit in bits {
        words[(*bit / 32) as usize] |= 1 << (*bit % 32);
    }
    words
}

/// A fully read node: every field present, so a test changes only what it
/// is about.
pub(crate) fn read_of(role: u32, name: &str, actions: &[&str]) -> ObservedRead {
    let node = RawNode {
        element: AtspiElement::new(":1.1", "/p", 1),
        properties: NodeProperties {
            name: Field::Present(name.to_owned()),
            description: Field::Present(String::new()),
            child_count: Field::Present(0),
            accessible_id: Field::Present(String::new()),
        },
        traits: NodeTraits {
            role: Field::Present(role),
            role_name: Field::Present("role".to_owned()),
            states: Field::Present(words(&[8, 25])),
            interfaces: Field::Present(Vec::new()),
        },
        attributes: attributes(&[]),
        extents: Field::Present(Extents {
            x: 0,
            y: 0,
            width: 10,
            height: 10,
        }),
        actions: Field::Present(actions.iter().map(|name| (*name).to_owned()).collect()),
        children: Field::Present(Vec::new()),
    };
    ObservedRead::plain(node)
}

pub(crate) fn method_error(name: &str) -> zbus::Error {
    let call = zbus::Message::method_call("/probe", "Probe")
        .and_then(|builder| builder.build(&()))
        .expect("a method call message must build");
    let reply = zbus::Message::error(&call.header(), name)
        .and_then(|builder| builder.build(&()))
        .expect("an error reply must build");
    zbus::Error::MethodError(name.try_into().expect("a valid error name"), None, reply)
}

pub(crate) fn client_timeout() -> zbus::Error {
    zbus::Error::InputOutput(std::sync::Arc::new(std::io::Error::from(
        std::io::ErrorKind::TimedOut,
    )))
}
