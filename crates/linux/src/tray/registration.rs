#![expect(dead_code, reason = "stub the owning lane replaces")]

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Registration {
    pub(crate) bus_name: String,
    pub(crate) object_path: String,
}

pub(crate) fn parse(_text: &str) -> Option<Registration> {
    None
}

#[cfg(test)]
#[path = "registration_tests.rs"]
mod tests;
