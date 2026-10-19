#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextUnitError {
    TooLong,
}

#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn utf8_len(_text: &str) -> Result<i32, TextUnitError> {
    Err(TextUnitError::TooLong)
}

#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn code_point_to_utf16(_text: &str, _offset: usize) -> Option<usize> {
    None
}

#[expect(dead_code, reason = "no caller yet")]
pub(crate) fn utf16_to_code_point(_text: &str, _offset: usize) -> Option<usize> {
    None
}

#[cfg(test)]
#[path = "text_units_tests.rs"]
mod tests;
