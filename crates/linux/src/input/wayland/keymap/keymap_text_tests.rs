use super::super::added_key::AddedKey;
use super::insert_keys;

const SMALL: &str = "xkb_keymap {\n\
xkb_keycodes \"odd{name\" {\n\
\t<ESC> = 9;\n\
};\n\
xkb_symbols \"x\" {\n\
\tname[Group1]=\"}\";\n\
\tkey <ESC> {\n\
\t\tsymbols[1]= [ Escape ]\n\
\t};\n\
};\n\
};\n";

fn added(named_by_owner: bool) -> AddedKey {
    AddedKey {
        keycode: 97,
        name: "I97".to_owned(),
        named_by_owner,
        symbol: "eacute".to_owned(),
    }
}

#[test]
fn keys_go_before_the_lines_that_close_their_sections() {
    let text = insert_keys(SMALL, &[added(false)]).expect("both sections are found");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines[3], "\t<I97>                = 97;");
    assert_eq!(lines[4], "};");
    assert_eq!(
        lines[10],
        "\tkey <I97> { type= \"ONE_LEVEL\", symbols[Group1]= [ eacute ] };"
    );
    assert_eq!(lines[11], "};");
    assert_eq!(lines.len(), SMALL.lines().count() + 2);
}

#[test]
fn a_keycode_the_owner_names_gets_no_second_name() {
    let text = insert_keys(SMALL, &[added(true)]).expect("both sections are found");
    assert_eq!(text.lines().count(), SMALL.lines().count() + 1);
    assert!(!text.contains("= 97;"));
}

#[test]
fn text_without_both_sections_is_unrecognized() {
    assert_eq!(insert_keys("xkb_keymap {\n};\n", &[added(false)]), None);
    let one_line = "xkb_keycodes \"x\" { <ESC> = 9; };\nxkb_symbols \"x\" {\n};\n";
    assert_eq!(insert_keys(one_line, &[added(false)]), None);
}
