//! Inserts added keys into XKB keymap text without touching a line of it.
//! Key names go before the line that closes `xkb_keycodes`, key definitions
//! before the line that closes `xkb_symbols`.

use super::added_key::AddedKey;

const KEYCODES_SECTION: &str = "xkb_keycodes";
const SYMBOLS_SECTION: &str = "xkb_symbols";

/// The owner text with `added` inserted, or `None` when the text has no
/// `xkb_keycodes` or `xkb_symbols` section closed on a line of its own.
pub(crate) fn insert_keys(owner: &str, added: &[AddedKey]) -> Option<String> {
    let lines: Vec<&str> = owner.split_inclusive('\n').collect();
    let keycodes_end = section_end(&lines, KEYCODES_SECTION)?;
    let symbols_end = section_end(&lines, SYMBOLS_SECTION)?;
    let ending = if owner.contains("\r\n") { "\r\n" } else { "\n" };
    let mut text = String::with_capacity(owner.len() + added.len() * 96);
    for (index, line) in lines.iter().enumerate() {
        if index == keycodes_end {
            for name in added.iter().filter_map(AddedKey::keycode_line) {
                text.push_str(&name);
                text.push_str(ending);
            }
        }
        if index == symbols_end {
            for key in added {
                text.push_str(&key.symbols_line());
                text.push_str(ending);
            }
        }
        text.push_str(line);
    }
    Some(text)
}

fn section_end(lines: &[&str], section: &str) -> Option<usize> {
    let start = lines
        .iter()
        .position(|line| line.trim_start().starts_with(section))?;
    let mut depth = 0_i64;
    for (index, line) in lines.iter().enumerate().skip(start) {
        let (delta, opens) = braces(line);
        depth += delta;
        if depth < 0 || (index == start && depth == 0 && opens) {
            return None;
        }
        if depth == 0 && index > start {
            return line.trim_start().starts_with('}').then_some(index);
        }
    }
    None
}

fn braces(line: &str) -> (i64, bool) {
    let mut delta = 0;
    let mut opens = false;
    let mut quoted = false;
    let mut escaped = false;
    for character in line.chars() {
        match character {
            _ if escaped => escaped = false,
            '\\' if quoted => escaped = true,
            '"' => quoted = !quoted,
            '{' if !quoted => {
                delta += 1;
                opens = true;
            }
            '}' if !quoted => delta -= 1,
            _ => {}
        }
    }
    (delta, opens)
}

#[cfg(test)]
#[path = "keymap_text_tests.rs"]
mod tests;
