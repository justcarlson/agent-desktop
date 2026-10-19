use super::TrayTarget;

const PREFIX: &str = "tray:v1:";
const ITEM: &str = "item";
const ENTRY: &str = "entry";

fn escape(text: &str, slash: bool) -> String {
    let mut out = String::with_capacity(text.len());
    for ch in text.chars() {
        match ch {
            '%' => out.push_str("%25"),
            '|' => out.push_str("%7C"),
            '/' if slash => out.push_str("%2F"),
            other => out.push(other),
        }
    }
    out
}

fn unescape(text: &str) -> Option<String> {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            out.push(ch);
            continue;
        }
        let code: String = chars.by_ref().take(2).collect();
        out.push(match code.as_str() {
            "25" => '%',
            "7C" => '|',
            "2F" => '/',
            _ => return None,
        });
    }
    Some(out)
}

pub(super) fn is_tray_id(value: &str) -> bool {
    value.starts_with(PREFIX)
}

pub(super) fn encode(target: &TrayTarget) -> String {
    let owner = escape(&target.owner, false);
    let path = escape(&target.object_path, false);
    let (Some(menu), Some(id)) = (&target.menu_path, target.entry_id) else {
        return format!("{PREFIX}{ITEM}|{owner}|{path}");
    };
    let labels = target
        .label_path
        .iter()
        .map(|label| escape(label, true))
        .collect::<Vec<_>>()
        .join("/");
    format!(
        "{PREFIX}{ENTRY}|{owner}|{path}|{}|{id}|{labels}",
        escape(menu, false)
    )
}

fn well_formed(target: &TrayTarget) -> bool {
    !target.owner.is_empty() && target.object_path.starts_with('/')
}

pub(super) fn decode(value: &str) -> Option<TrayTarget> {
    let body = value.strip_prefix(PREFIX)?;
    let parts: Vec<&str> = body.split('|').collect();
    let target = match parts.as_slice() {
        [ITEM, owner, path] => TrayTarget {
            owner: unescape(owner)?,
            object_path: unescape(path)?,
            menu_path: None,
            entry_id: None,
            revision: None,
            label_path: Vec::new(),
        },
        [ENTRY, owner, path, menu, id, labels] => TrayTarget {
            owner: unescape(owner)?,
            object_path: unescape(path)?,
            menu_path: Some(unescape(menu)?),
            entry_id: Some(id.parse().ok()?),
            revision: None,
            label_path: labels
                .split('/')
                .map(unescape)
                .collect::<Option<Vec<_>>>()?,
        },
        _ => return None,
    };
    well_formed(&target).then_some(target)
}

#[cfg(test)]
#[path = "identity_tests.rs"]
mod tests;
