/// What a restore reads beside a client's place and state: the id of the
/// monitor it is on and the tags it carries.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ClientMarks {
    pub(crate) monitor: Option<i64>,
    pub(crate) tags: Vec<String>,
}
