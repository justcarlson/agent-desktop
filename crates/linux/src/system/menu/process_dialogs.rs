/// The dialogs of one process as its accessibility tree shows them, read for
/// a close request that must not answer them.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProcessDialogs {
    /// Titles of the dialogs drawn inside its windows, which no list of
    /// top-level windows holds. A blank title is an empty string.
    pub(crate) nested: Vec<String>,
    /// Titles of its showing top-level frames with a dialog role. A
    /// compositor list marks a dialog only by the parent it reports, and a
    /// dialog outside the app's own window group may have none.
    pub(crate) framed: Vec<String>,
}
