/// One visited node with the children the walk descended into. `truncated`
/// is set when children exist that the walk did not read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WalkedNode<P> {
    pub(crate) payload: P,
    pub(crate) children: Vec<WalkedNode<P>>,
    pub(crate) truncated: bool,
}
