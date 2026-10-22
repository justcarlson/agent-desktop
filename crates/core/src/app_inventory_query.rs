/// Presentation matching includes display-name substrings and exact aliases;
/// mutation lookup requires an exact identifier and optional bundle identity.
#[derive(Debug, Clone, Copy)]
pub enum AppInventoryQuery<'a> {
    All,
    NameOrAlias(&'a str),
    Exact {
        name: &'a str,
        bundle_id: Option<&'a str>,
    },
}

#[cfg(test)]
#[path = "app_inventory_query_tests.rs"]
mod tests;
