#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextUnitError {
    TooLong,
}

fn length_as_i32(length: usize) -> Result<i32, TextUnitError> {
    i32::try_from(length).map_err(|_| TextUnitError::TooLong)
}

/// The UTF-8 byte length of `text`, the length `InsertText` takes. A length
/// above `i32::MAX` cannot be sent and is refused.
pub(crate) fn utf8_len(text: &str) -> Result<i32, TextUnitError> {
    length_as_i32(text.len())
}

/// The UTF-16 offset of the code point at `offset`, or `None` past the end.
/// An offset equal to the code-point count is the end of the text.
pub(crate) fn code_point_to_utf16(text: &str, offset: usize) -> Option<usize> {
    let mut units = 0;
    let mut points = text.chars();
    for _ in 0..offset {
        units += points.next()?.len_utf16();
    }
    Some(units)
}

/// The code-point offset of the UTF-16 unit at `offset`, or `None` past the
/// end or between the two halves of a surrogate pair.
#[cfg_attr(not(test), expect(dead_code, reason = "no caller yet"))]
pub(crate) fn utf16_to_code_point(text: &str, offset: usize) -> Option<usize> {
    let mut units = 0;
    for (index, point) in text.chars().enumerate() {
        if units == offset {
            return Some(index);
        }
        if units > offset {
            return None;
        }
        units += point.len_utf16();
    }
    (units == offset).then(|| text.chars().count())
}

#[cfg(test)]
#[path = "text_units_tests.rs"]
mod tests;
