use agent_desktop_core::{AdapterError, ErrorCode};

/// The chunks of one INCR transfer on the owner's side. The transfer ends
/// with a zero-length chunk, also when the data is empty or an exact multiple
/// of the chunk size.
#[derive(Debug)]
pub(crate) struct IncrSender {
    data: Vec<u8>,
    chunk: usize,
    sent: usize,
    finished: bool,
}

impl IncrSender {
    pub(crate) fn new(data: Vec<u8>, chunk: usize) -> Self {
        Self {
            data,
            chunk: chunk.max(1),
            sent: 0,
            finished: false,
        }
    }

    pub(crate) fn next_chunk(&mut self) -> Option<Vec<u8>> {
        if self.finished {
            return None;
        }
        if self.sent >= self.data.len() {
            self.finished = true;
            return Some(Vec::new());
        }
        let end = (self.sent + self.chunk).min(self.data.len());
        let chunk = self.data[self.sent..end].to_vec();
        self.sent = end;
        Some(chunk)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Push {
    More,
    Complete,
}

/// The reassembly of one INCR transfer on the reader's side.
#[derive(Debug)]
pub(crate) struct IncrReceiver {
    bytes: Vec<u8>,
    cap: usize,
    complete: bool,
}

impl IncrReceiver {
    pub(crate) fn new(cap: usize) -> Self {
        Self {
            bytes: Vec::new(),
            cap,
            complete: false,
        }
    }

    pub(crate) fn push(&mut self, chunk: &[u8]) -> Result<Push, AdapterError> {
        if chunk.is_empty() {
            self.complete = true;
            return Ok(Push::Complete);
        }
        if self.bytes.len() + chunk.len() > self.cap {
            return Err(AdapterError::new(
                ErrorCode::ActionFailed,
                "The clipboard holds more than the size cap",
            ));
        }
        self.bytes.extend_from_slice(chunk);
        Ok(Push::More)
    }

    pub(crate) fn into_bytes(self) -> Option<Vec<u8>> {
        self.complete.then_some(self.bytes)
    }
}

#[cfg(test)]
#[path = "incr_tests.rs"]
mod tests;
