/// Fields needed by one fresh observation; omitted fields remain unknown.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReadPurpose {
    Snapshot,
    Sampling,
    Identity,
    Element,
    State,
    Bounds,
    Value,
    Actions,
    Frame,
    Structure,
}

impl ReadPurpose {
    pub(super) fn identity(self) -> bool {
        matches!(
            self,
            Self::Snapshot | Self::Sampling | Self::Identity | Self::Element
        )
    }

    pub(super) fn vocabulary(self) -> bool {
        self.identity() || matches!(self, Self::State | Self::Actions | Self::Value)
    }

    pub(super) fn value(self) -> bool {
        self.identity() || matches!(self, Self::State | Self::Value)
    }

    pub(super) fn bounds(self) -> bool {
        (self.identity() && self != Self::Sampling)
            || matches!(self, Self::State | Self::Bounds | Self::Frame)
    }

    pub(super) fn actions(self) -> bool {
        matches!(
            self,
            Self::Snapshot | Self::Sampling | Self::Element | Self::Actions
        )
    }

    pub(super) fn children(self) -> bool {
        matches!(self, Self::Snapshot | Self::Sampling | Self::Structure)
    }
}
