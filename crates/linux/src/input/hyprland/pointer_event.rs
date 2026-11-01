use std::time::Duration;

use agent_desktop_core::{AdapterError, DisplayInfo};

use super::output_map::to_output;
use super::output_point::OutputPoint;
use super::pointer_sink::PointerSink;
use super::run_plans::{off_output, unsupported_step};
use super::step_event::{linux_button, wheel_of};
use super::wheel_axis::WheelAxis;
use crate::input::step::Step;

/// One plan step as the virtual pointer sends it, mapped before any event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PointerEvent {
    Motion(OutputPoint),
    Button { code: u32, down: bool },
    Wheel { axis: WheelAxis, discrete: i32 },
    Pause(Duration),
}

impl PointerEvent {
    /// The event for `step`; `None` for a wheel button release, which the
    /// wheel step already holds.
    pub(crate) fn of(step: &Step, displays: &[DisplayInfo]) -> Result<Option<Self>, AdapterError> {
        match step {
            Step::MoveTo { x, y } => to_output(*x, *y, displays)
                .map(|point| Some(Self::Motion(point)))
                .ok_or_else(|| off_output(*x, *y)),
            Step::Button { code, down } => match (wheel_of(*code), linux_button(*code)) {
                (Some(_), _) if !down => Ok(None),
                (Some((axis, discrete)), _) => Ok(Some(Self::Wheel { axis, discrete })),
                (None, Some(code)) => Ok(Some(Self::Button { code, down: *down })),
                (None, None) => Err(unsupported_step(step)),
            },
            Step::Pause(duration) => Ok(Some(Self::Pause(*duration))),
            Step::Key { .. } | Step::Keymap(_) | Step::CheckTextState => {
                Err(unsupported_step(step))
            }
        }
    }

    /// Whether the focus gate runs before this event.
    pub(crate) fn gated(&self) -> bool {
        matches!(self, Self::Button { down: true, .. } | Self::Wheel { .. })
    }

    /// How many delivered events this counts as.
    pub(crate) fn counts(&self) -> u8 {
        u8::from(!matches!(self, Self::Pause(_)))
    }

    pub(crate) fn send(&self, sink: &mut dyn PointerSink) -> Result<(), AdapterError> {
        match self {
            Self::Motion(point) => sink.motion(point),
            Self::Button { code, down } => sink.button(*code, *down),
            Self::Wheel { axis, discrete } => sink.wheel(*axis, *discrete),
            Self::Pause(duration) => {
                sink.pause(*duration);
                Ok(())
            }
        }
    }

    /// Records a sent press as held and a sent release as no longer held.
    pub(crate) fn track(&self, held: &mut Vec<u32>) {
        match self {
            Self::Button { code, down: true } => held.push(*code),
            Self::Button { code, down: false } => held.retain(|pressed| pressed != code),
            _ => {}
        }
    }
}
