use std::collections::BTreeMap;

use super::Field;

pub(super) fn attributes(pairs: &[(&str, &str)]) -> Field<BTreeMap<String, String>> {
    Field::Present(
        pairs
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect(),
    )
}

pub(super) fn words(bits: &[u32]) -> Vec<u32> {
    let mut words = vec![0u32; 2];
    for bit in bits {
        words[(*bit / 32) as usize] |= 1 << (*bit % 32);
    }
    words
}
