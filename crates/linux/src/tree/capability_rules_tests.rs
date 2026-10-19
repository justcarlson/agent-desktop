use agent_desktop_core::LocatorField;

use super::{Facts, rule_tokens, scroll_tokens};
use crate::tree::action_names::capabilities;
use crate::tree::role_numbers::{
    DIAL, ENTRY, LEVEL_BAR, LIST, PANEL, PASSWORD_TEXT, PROGRESS_BAR, PUSH_BUTTON, SCROLL_BAR,
    SCROLL_PANE, SLIDER, SPIN_BUTTON, TEXT,
};
use crate::tree::state_bits::{EDITABLE, EXPANDABLE, EXPANDED, HAS_POPUP, READ_ONLY, SENSITIVE};
use crate::tree::test_support::{read_of, words};
use crate::tree::{Field, RawNode};

const LIST_ITEM: u32 = 32;
const VALUE: &str = "org.a11y.atspi.Value";
const TEXT_IFACE: &str = "org.a11y.atspi.Text";
const EDITABLE_TEXT: &str = "org.a11y.atspi.EditableText";
const SELECTION: &str = "org.a11y.atspi.Selection";

struct Spec<'a> {
    role: u32,
    bits: &'a [u32],
    interfaces: &'a [&'a str],
    names: &'a [&'a str],
    children: i32,
}

fn node(spec: &Spec) -> RawNode {
    let mut node = read_of(spec.role, "n", spec.names).node;
    let mut bits = spec.bits.to_vec();
    bits.push(SENSITIVE);
    node.traits.states = Field::Present(words(&bits));
    node.traits.interfaces =
        Field::Present(spec.interfaces.iter().map(|i| (*i).to_owned()).collect());
    node.properties.child_count = Field::Present(spec.children);
    node
}

fn plain(role: u32) -> Spec<'static> {
    Spec {
        role,
        bits: &[],
        interfaces: &[],
        names: &[],
        children: 0,
    }
}

fn tokens_of(spec: &Spec) -> Vec<&'static str> {
    let node = node(spec);
    rule_tokens(&Facts::of(&node).expect("every read present"))
}

fn all_tokens(spec: &Spec) -> Vec<String> {
    match capabilities(&node(spec)) {
        LocatorField::Known(tokens) => tokens,
        other => panic!("expected known tokens, got {other:?}"),
    }
}

#[test]
fn a_stateful_node_with_a_click_class_verb_gets_both_disclosure_tokens() {
    let spec = Spec {
        bits: &[EXPANDED],
        names: &["Activate"],
        ..plain(LIST_ITEM)
    };
    assert_eq!(tokens_of(&spec), ["Expand", "Collapse"]);
}

#[test]
fn a_stateful_node_without_a_click_class_verb_gets_no_disclosure_token() {
    let spec = Spec {
        bits: &[EXPANDED],
        names: &["SetFocus"],
        ..plain(LIST_ITEM)
    };
    assert!(tokens_of(&spec).is_empty());
}

#[test]
fn a_node_with_neither_state_bit_gets_no_disclosure_token() {
    let spec = Spec {
        names: &["activate", "listitem.toggle-expand"],
        ..plain(LIST_ITEM)
    };
    assert!(tokens_of(&spec).is_empty());
}

#[test]
fn disclosure_verbs_give_the_tokens_they_name() {
    let both = Spec {
        bits: &[EXPANDABLE],
        names: &["listitem.toggle-expand"],
        ..plain(LIST_ITEM)
    };
    assert_eq!(tokens_of(&both), ["Expand", "Collapse"]);
    let expand = Spec {
        names: &["listitem.expand"],
        ..both
    };
    assert_eq!(tokens_of(&expand), ["Expand"]);
    let collapse = Spec {
        names: &["listitem.collapse"],
        ..both
    };
    assert_eq!(tokens_of(&collapse), ["Collapse"]);
}

#[test]
fn a_clipboard_verb_alone_gives_no_token() {
    let spec = Spec {
        bits: &[EXPANDABLE],
        names: &["clipboard.copy"],
        ..plain(LIST_ITEM)
    };
    assert!(all_tokens(&spec).is_empty());
}

#[test]
fn a_popup_button_with_one_child_and_no_verb_is_a_menu_button() {
    let spec = Spec {
        bits: &[HAS_POPUP],
        children: 1,
        ..plain(PUSH_BUTTON)
    };
    assert_eq!(tokens_of(&spec), ["Expand", "Collapse"]);
}

#[test]
fn an_expanded_popup_button_keeps_its_pair_while_the_popover_is_its_second_child() {
    let open = Spec {
        bits: &[HAS_POPUP, EXPANDED],
        children: 2,
        ..plain(PUSH_BUTTON)
    };
    assert_eq!(tokens_of(&open), ["Expand", "Collapse"]);
    let crowded = Spec {
        bits: &[HAS_POPUP, EXPANDED],
        children: 3,
        ..plain(PUSH_BUTTON)
    };
    assert!(tokens_of(&crowded).is_empty());
}

#[test]
fn a_button_that_is_not_a_menu_button_gets_no_disclosure_token() {
    let two_children = Spec {
        bits: &[HAS_POPUP],
        children: 2,
        ..plain(PUSH_BUTTON)
    };
    assert!(tokens_of(&two_children).is_empty());
    let no_popup = Spec {
        children: 1,
        ..plain(PUSH_BUTTON)
    };
    assert!(tokens_of(&no_popup).is_empty());
    let not_a_button = Spec {
        bits: &[HAS_POPUP],
        children: 1,
        ..plain(PANEL)
    };
    assert!(tokens_of(&not_a_button).is_empty());
}

#[test]
fn a_slider_with_value_gets_set_value() {
    let with_value = Spec {
        interfaces: &[VALUE],
        ..plain(SLIDER)
    };
    assert_eq!(tokens_of(&with_value), ["SetValue"]);
    assert_eq!(
        tokens_of(&Spec {
            role: DIAL,
            ..with_value
        }),
        ["SetValue"]
    );
}

#[test]
fn a_meter_and_a_spin_button_without_value_get_no_set_value() {
    for role in [LEVEL_BAR, PROGRESS_BAR] {
        let spec = Spec {
            interfaces: &[VALUE],
            ..plain(role)
        };
        assert!(tokens_of(&spec).is_empty(), "role {role}");
    }
    assert!(tokens_of(&plain(SPIN_BUTTON)).is_empty());
}

#[test]
fn a_scroll_bar_with_value_gets_set_value_whatever_the_toolkit() {
    let spec = Spec {
        interfaces: &[VALUE],
        ..plain(SCROLL_BAR)
    };
    assert_eq!(tokens_of(&spec), ["SetValue"]);
}

#[test]
fn an_editable_entry_with_editable_text_gets_set_value_and_type_text() {
    let spec = Spec {
        bits: &[EDITABLE],
        interfaces: &[EDITABLE_TEXT, TEXT_IFACE],
        ..plain(ENTRY)
    };
    assert_eq!(tokens_of(&spec), ["SetValue", "TypeText"]);
    let password = Spec {
        role: PASSWORD_TEXT,
        ..spec
    };
    assert_eq!(tokens_of(&password), ["SetValue", "TypeText"]);
}

#[test]
fn an_entry_without_editable_text_gets_neither_token() {
    let spec = Spec {
        bits: &[EDITABLE],
        interfaces: &[TEXT_IFACE],
        ..plain(ENTRY)
    };
    assert!(tokens_of(&spec).is_empty());
}

#[test]
fn a_read_only_entry_gets_neither_token() {
    let spec = Spec {
        interfaces: &[EDITABLE_TEXT, TEXT_IFACE],
        ..plain(ENTRY)
    };
    assert!(tokens_of(&spec).is_empty());
}

#[test]
fn an_editable_entry_marked_read_only_gets_no_token() {
    let spec = Spec {
        bits: &[EDITABLE, READ_ONLY],
        interfaces: &[EDITABLE_TEXT, TEXT_IFACE],
        ..plain(ENTRY)
    };
    assert!(tokens_of(&spec).is_empty());
}

#[test]
fn type_text_needs_the_text_interface_and_set_value_does_not() {
    let spec = Spec {
        bits: &[EDITABLE],
        interfaces: &[EDITABLE_TEXT],
        ..plain(TEXT)
    };
    assert_eq!(tokens_of(&spec), ["SetValue"]);
}

#[test]
fn a_node_that_is_not_a_text_role_gets_no_text_token() {
    let spec = Spec {
        bits: &[EDITABLE],
        interfaces: &[EDITABLE_TEXT, TEXT_IFACE],
        ..plain(PUSH_BUTTON)
    };
    assert!(tokens_of(&spec).is_empty());
}

#[test]
fn a_list_with_selection_and_children_gets_select() {
    let spec = Spec {
        interfaces: &[SELECTION],
        children: 3,
        ..plain(LIST)
    };
    assert_eq!(tokens_of(&spec), ["Select"]);
}

#[test]
fn a_list_without_children_or_without_selection_gets_no_select() {
    let empty = Spec {
        interfaces: &[SELECTION],
        ..plain(LIST)
    };
    assert!(tokens_of(&empty).is_empty());
    let unselectable = Spec {
        children: 3,
        ..plain(LIST)
    };
    assert!(tokens_of(&unselectable).is_empty());
}

#[test]
fn a_failed_interface_or_action_read_makes_the_node_unknown() {
    let mut failed_interfaces = node(&plain(SLIDER));
    failed_interfaces.traits.interfaces = Field::Failed("x".to_owned());
    assert_eq!(capabilities(&failed_interfaces), LocatorField::Unknown);
    let mut failed_actions = node(&plain(SLIDER));
    failed_actions.actions = Field::Failed("x".to_owned());
    assert_eq!(capabilities(&failed_actions), LocatorField::Unknown);
}

#[test]
fn a_failed_interface_read_on_a_layout_node_keeps_the_tokens_its_names_give() {
    let mut panel = node(&Spec {
        names: &["click"],
        ..plain(PANEL)
    });
    panel.traits.interfaces = Field::Failed("x".to_owned());
    assert_eq!(
        capabilities(&panel),
        LocatorField::Known(vec!["Click".to_owned()])
    );
}

#[test]
fn a_failed_child_count_matters_only_to_a_selection_node() {
    let mut selection = node(&Spec {
        interfaces: &[SELECTION],
        children: 3,
        ..plain(LIST)
    });
    selection.properties.child_count = Field::Failed("x".to_owned());
    assert_eq!(capabilities(&selection), LocatorField::Unknown);
    let mut button = node(&plain(PUSH_BUTTON));
    button.properties.child_count = Field::Failed("x".to_owned());
    assert_eq!(capabilities(&button), LocatorField::Known(Vec::new()));
}

#[test]
fn no_node_gets_scroll_from_this_table() {
    let specs = [
        Spec {
            interfaces: &[VALUE],
            ..plain(SCROLL_BAR)
        },
        Spec {
            interfaces: &[SELECTION],
            children: 5,
            names: &["click"],
            ..plain(SCROLL_PANE)
        },
    ];
    for spec in &specs {
        assert!(scroll_tokens(&node(spec)).is_empty());
        assert!(!all_tokens(spec).iter().any(|token| token == "Scroll"));
    }
}

#[test]
fn a_switch_with_only_a_toggle_verb_also_takes_a_click() {
    use crate::tree::role_numbers::{CHECK_BOX, TOGGLE_BUTTON};
    for role in [CHECK_BOX, TOGGLE_BUTTON] {
        for verb in ["toggle", "switch", "uncheck"] {
            let spec = Spec {
                names: &[verb],
                ..plain(role)
            };
            assert!(tokens_of(&spec).contains(&"Click"), "{role} {verb}");
        }
    }
}

#[test]
fn a_toggle_verb_on_a_button_or_a_plain_check_verb_adds_no_click_rule() {
    use crate::tree::role_numbers::PUSH_BUTTON;
    let button = Spec {
        names: &["toggle"],
        ..plain(PUSH_BUTTON)
    };
    assert!(!tokens_of(&button).contains(&"Click"));
}

fn tokens_with_states(role: u32, names: &[&str], bits: &[u32]) -> Vec<&'static str> {
    let mut raw = node(&Spec {
        names,
        ..plain(role)
    });
    raw.traits.states = Field::Present(words(bits));
    rule_tokens(&Facts::of(&raw).expect("every read present"))
}

#[test]
fn a_disabled_button_keeps_the_click_a_live_one_would_carry() {
    use crate::tree::role_numbers::CHECK_BOX;
    for role in [PUSH_BUTTON, CHECK_BOX] {
        let disabled = tokens_with_states(role, &["doDefault"], &[]);
        assert!(disabled.contains(&"Click"), "{role}");
        assert!(!tokens_with_states(role, &["doDefault"], &[SENSITIVE]).contains(&"Click"));
    }
    assert!(!tokens_with_states(PANEL, &["doDefault"], &[]).contains(&"Click"));
}

#[test]
fn a_read_only_node_is_never_given_the_disabled_click() {
    assert!(!tokens_with_states(PUSH_BUTTON, &[], &[READ_ONLY]).contains(&"Click"));
}
