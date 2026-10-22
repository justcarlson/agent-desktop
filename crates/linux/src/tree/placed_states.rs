/// Fresh state tokens and the placement facts used to derive them.
pub(super) struct PlacedStates {
    pub(super) states: Vec<String>,
    pub(super) hidden: Option<bool>,
    pub(super) offscreen: Option<bool>,
}
