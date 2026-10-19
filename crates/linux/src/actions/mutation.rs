#[expect(dead_code, reason = "no caller yet")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteKind {
    DoAction,
    SelectChild,
    ClearSelection,
    InsertText,
    DeleteText,
    SetTextContents,
    SetProperty,
}

#[expect(dead_code, reason = "no caller yet")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Corroboration {
    Alive,
    Dead,
    TimedOut,
    NotRead,
}

#[expect(dead_code, reason = "no caller yet")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WriteReply {
    Done,
    Absent,
}

#[cfg(test)]
#[path = "mutation_tests.rs"]
mod tests;
