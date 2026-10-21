/// For each panel node title, the index of the one watcher item it names. A
/// title pairs only when it is unique among the panel nodes and among the
/// items; a repeated title pairs with nothing, because the panel order is
/// not evidence of which item a node is.
pub(crate) fn pair(items: &[String], panel_titles: &[String]) -> Vec<Option<usize>> {
    panel_titles
        .iter()
        .map(|title| {
            let unique_in_panel = panel_titles.iter().filter(|other| *other == title).count() == 1;
            let mut matching = items
                .iter()
                .enumerate()
                .filter(|(_, item)| *item == title)
                .map(|(index, _)| index);
            let first = matching.next();
            match (unique_in_panel, first, matching.next()) {
                (true, Some(index), None) if !title.is_empty() => Some(index),
                _ => None,
            }
        })
        .collect()
}
