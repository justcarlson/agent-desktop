use crate::AppError;

pub(super) struct ReadBudget {
    bytes: usize,
    events: usize,
    entries: usize,
    pub(super) exceeded: bool,
}

impl Default for ReadBudget {
    fn default() -> Self {
        Self {
            bytes: 32 * 1024 * 1024,
            events: 100_000,
            entries: 4_096,
            exceeded: false,
        }
    }
}

impl ReadBudget {
    pub(super) fn consume_bytes(&mut self, count: usize) -> Result<(), AppError> {
        consume(
            &mut self.bytes,
            count,
            &mut self.exceeded,
            "32 MiB of input",
        )
    }

    pub(super) fn consume_event(&mut self) -> Result<(), AppError> {
        consume(&mut self.events, 1, &mut self.exceeded, "100,000 events")
    }

    pub(super) fn consume_entry(&mut self) -> Result<(), AppError> {
        consume(
            &mut self.entries,
            1,
            &mut self.exceeded,
            "4,096 directory entries",
        )
    }
}

fn consume(
    remaining: &mut usize,
    count: usize,
    exceeded: &mut bool,
    limit: &str,
) -> Result<(), AppError> {
    if let Some(next) = remaining.checked_sub(count) {
        *remaining = next;
        Ok(())
    } else {
        *exceeded = true;
        Err(AppError::invalid_input_with_suggestion(
            format!("Trace exceeds the safety limit of {limit}"),
            "Read a smaller trace directory; --limit only limits output, not input.",
        ))
    }
}
