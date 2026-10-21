use agent_desktop_core::NotificationInfo;

use super::world::{Node, Role};
use crate::tree::{AtspiElement, Extents};

const OFF_SCREEN_FLOOR: i32 = -1_000_000;

#[derive(Debug, Clone)]
pub(crate) struct Control {
    pub(crate) name: String,
    pub(crate) extents: Option<Extents>,
    pub(crate) element: AtspiElement,
}

#[derive(Debug, Clone)]
pub(crate) struct Entry {
    pub(crate) info: NotificationInfo,
    pub(crate) expand: Option<Control>,
    pub(crate) close: Option<Control>,
    pub(crate) actions: Vec<Control>,
}

/// Splits a container reading into the subtree of each notification node and
/// the nodes that sit outside every notification.
pub(crate) fn partition(nodes: &[Node]) -> (Vec<&[Node]>, Vec<&Node>) {
    let mut entries = Vec::new();
    let mut outside = Vec::new();
    let mut position = 0;
    while position < nodes.len() {
        let node = &nodes[position];
        if node.role == Role::Notification {
            let end = nodes[position + 1..]
                .iter()
                .position(|next| next.depth <= node.depth)
                .map_or(nodes.len(), |offset| position + 1 + offset);
            entries.push(&nodes[position..end]);
            position = end;
        } else {
            outside.push(node);
            position += 1;
        }
    }
    (entries, outside)
}

/// Whether the shell has laid the control out. Hidden controls report a
/// sentinel or empty rectangle until their entry is expanded.
pub(crate) fn is_laid_out(extents: Option<Extents>) -> bool {
    extents.is_some_and(|rect| {
        rect.width > 0 && rect.height > 0 && rect.x > OFF_SCREEN_FLOOR && rect.y > OFF_SCREEN_FLOOR
    })
}

/// Reads one entry from its subtree. The first node is the notification node.
/// Labels read app, optional time, title, optional body; push buttons with a
/// name are actions and the two unnamed ones are the expand and close buttons
/// of the header, in that order. Labels inside a push button belong to it.
pub(crate) fn extract(subtree: &[Node], index: usize) -> Option<Entry> {
    let mut labels = Vec::new();
    let mut unnamed = Vec::new();
    let mut actions = Vec::new();
    let mut button_depth: Option<usize> = None;
    for node in subtree.iter().skip(1) {
        if let Some(depth) = button_depth {
            if node.depth > depth {
                continue;
            }
            button_depth = None;
        }
        match node.role {
            Role::PushButton => {
                button_depth = Some(node.depth);
                let control = Control {
                    name: node.name.clone(),
                    extents: node.extents,
                    element: node.element.clone(),
                };
                if node.name.trim().is_empty() {
                    unnamed.push(control);
                } else {
                    actions.push(control);
                }
            }
            Role::Label if !node.name.trim().is_empty() => labels.push(node.name.clone()),
            _ => {}
        }
    }
    let mut fields = separate_time(labels).into_iter();
    let app_name = fields.next()?;
    let title = fields.next()?;
    let body: Vec<String> = fields.collect();
    let mut controls = unnamed.into_iter();
    Some(Entry {
        info: NotificationInfo {
            index,
            app_name,
            title,
            body: (!body.is_empty()).then(|| body.join("\n")),
            actions: actions.iter().map(|action| action.name.clone()).collect(),
        },
        expand: controls.next(),
        close: controls.next(),
        actions,
    })
}

/// Drops the time label of the open-list shape. The shell gives no structural
/// marker for it, so the rule is its position, second of three or more, plus
/// the shape of the text.
pub(crate) fn separate_time(mut labels: Vec<String>) -> Vec<String> {
    if labels.len() >= 3 && is_time_label(&labels[1]) {
        labels.remove(1);
    }
    labels
}

pub(crate) fn is_time_label(text: &str) -> bool {
    let lowered = text.trim().to_lowercase();
    matches!(lowered.as_str(), "just now" | "now" | "yesterday")
        || is_relative_age(&lowered)
        || is_clock_time(&lowered)
}

fn is_relative_age(lowered: &str) -> bool {
    let mut words = lowered.split_whitespace();
    let (Some(count), Some(unit), Some("ago"), None) =
        (words.next(), words.next(), words.next(), words.next())
    else {
        return false;
    };
    !count.is_empty()
        && count.chars().all(|digit| digit.is_ascii_digit())
        && matches!(
            unit.trim_end_matches('s'),
            "second" | "sec" | "minute" | "min" | "hour" | "hr" | "day" | "week" | "month" | "year"
        )
}

fn is_clock_time(lowered: &str) -> bool {
    let bare = lowered
        .trim_end_matches("am")
        .trim_end_matches("pm")
        .trim_end();
    let Some((hours, minutes)) = bare.split_once(':') else {
        return false;
    };
    (1..=2).contains(&hours.len())
        && minutes.len() == 2
        && hours
            .chars()
            .chain(minutes.chars())
            .all(|digit| digit.is_ascii_digit())
}

#[cfg(test)]
#[path = "extract_tests.rs"]
mod tests;
