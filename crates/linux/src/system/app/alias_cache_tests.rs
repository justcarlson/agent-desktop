use super::AliasCache;
use std::path::PathBuf;

#[test]
fn cache_is_constructible_without_an_index() {
    let cache = AliasCache::new();
    let _ = cache.index(&[PathBuf::from("missing-alias-cache-test-directory")]);
}
