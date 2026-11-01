/// A key the extension adds on an unbound keycode: its keycode, the key name
/// it is written under, whether the owner keymap already names that keycode,
/// and the XKB name of the one keysym it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AddedKey {
    pub(crate) keycode: u32,
    pub(crate) name: String,
    pub(crate) named_by_owner: bool,
    pub(crate) symbol: String,
}

impl AddedKey {
    pub(crate) fn keycode_line(&self) -> Option<String> {
        if self.named_by_owner {
            return None;
        }
        Some(format!(
            "\t{:<20} = {};",
            format!("<{}>", self.name),
            self.keycode
        ))
    }

    pub(crate) fn symbols_line(&self) -> String {
        format!(
            "\tkey <{}> {{ type= \"ONE_LEVEL\", symbols[Group1]= [ {} ] }};",
            self.name, self.symbol
        )
    }
}
