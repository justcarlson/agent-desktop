//! The wire format the fake compositor speaks: messages built from words
//! and strings, and the reads that take them apart.

pub(super) enum Arg<'a> {
    Uint(u32),
    Text(&'a str),
}

pub(super) fn message(object: u32, opcode: u16, args: &[Arg<'_>]) -> Vec<u8> {
    let mut body = Vec::new();
    for arg in args {
        match arg {
            Arg::Uint(value) => body.extend_from_slice(&value.to_ne_bytes()),
            Arg::Text(text) => {
                let length = u32::try_from(text.len() + 1).expect("short text");
                body.extend_from_slice(&length.to_ne_bytes());
                body.extend_from_slice(text.as_bytes());
                body.push(0);
                while body.len() % 4 != 0 {
                    body.push(0);
                }
            }
        }
    }
    let size = u32::try_from(body.len() + 8).expect("short message");
    let mut out = object.to_ne_bytes().to_vec();
    out.extend_from_slice(&((size << 16) | u32::from(opcode)).to_ne_bytes());
    out.extend_from_slice(&body);
    out
}

pub(super) fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_ne_bytes(bytes[at..at + 4].try_into().expect("four bytes"))
}

pub(super) fn text_at(bytes: &[u8], at: usize) -> (String, usize) {
    let length = word(bytes, at) as usize;
    let text = String::from_utf8_lossy(&bytes[at + 4..at + 4 + length.saturating_sub(1)]);
    (text.into_owned(), at + 4 + length.div_ceil(4) * 4)
}

pub(super) fn done(callback: u32) -> Vec<u8> {
    let mut out = message(callback, 0, &[Arg::Uint(0)]);
    out.extend(message(1, 1, &[Arg::Uint(callback)]));
    out
}
