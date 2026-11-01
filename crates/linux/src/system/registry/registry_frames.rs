use crate::system::registry::frame_read::FrameRead;
use std::future::Future;
use std::time::Duration;

use agent_desktop_core::{AdapterError, Deadline, ErrorCode};
use futures_util::stream::{self, StreamExt};

use crate::system::registry::app_patience::AppPatience;
use crate::system::registry::read_patience::ReadPatience;
use crate::system::{SharedBus, within};
use crate::tree::{AtspiElement, RawNode, ReadPurpose, list_children, read_node_for};

pub(crate) const CHILDREN_SLICE: Duration = Duration::from_millis(300);
pub(crate) const FRAME_SLICE: Duration = Duration::from_millis(300);
const APPS_IN_FLIGHT: usize = 16;
const FRAMES_IN_FLIGHT: usize = 8;

/// Whether an application's frames were read to the end.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RootRead {
    Read,
    TimedOut,
}

/// Whether a frame was read without the fields that decide if it is a window,
/// which one failed read leaves unanswered and not absent.
fn is_unclassified(node: &RawNode) -> bool {
    node.traits.role.is_failed()
        || node.traits.interfaces.is_failed()
        || node.traits.states.is_failed()
}

/// Whether a failed read says the object or its application left, which is an
/// absence and not a slow answer.
fn is_gone(error: &AdapterError) -> bool {
    matches!(
        error.code,
        ErrorCode::WindowNotFound | ErrorCode::AppNotFound | ErrorCode::ElementNotFound
    )
}

/// What one read of a frame or a child list came to.
enum Outcome<T> {
    Read(T),
    Gone,
    Slow,
}

/// A read that failed for another reason while its process is no longer
/// there is an absence: a call in flight when an application is killed fails
/// with a bus error for a moment before the name leaves the bus.
fn outcome<T>(answer: Option<Result<T, AdapterError>>) -> Outcome<T> {
    match answer {
        Some(Ok(value)) => Outcome::Read(value),
        Some(Err(error)) if is_gone(&error) => Outcome::Gone,
        Some(Err(_)) | None => Outcome::Slow,
    }
}

/// Reads every frame of one application, each within its own `patience`. A
/// frame that is gone is left out; one that does not answer in time or fails
/// otherwise is left out and marks the read `TimedOut`, so one stalled frame
/// costs one wait and never reads as a closed window.
async fn collect_frames<T, F, Fut>(
    children: Vec<(String, String)>,
    patience: ReadPatience,
    is_unclassified: impl Fn(&T) -> bool,
    read: F,
) -> (Vec<T>, RootRead)
where
    F: Fn((String, String)) -> Fut,
    Fut: Future<Output = Result<T, AdapterError>>,
{
    let outcomes = stream::iter(children.into_iter().map(|child| patience.wait(read(child))))
        .buffered(FRAMES_IN_FLIGHT)
        .map(outcome)
        .collect::<Vec<_>>()
        .await;
    let mut complete = RootRead::Read;
    let mut nodes = Vec::new();
    for outcome in outcomes {
        match outcome {
            Outcome::Read(node) => {
                if is_unclassified(&node) {
                    complete = RootRead::TimedOut;
                }
                nodes.push(node);
            }
            Outcome::Gone => {}
            Outcome::Slow => complete = RootRead::TimedOut,
        }
    }
    (nodes, complete)
}

/// Reads one application: its child list, then each frame. A child list that
/// is gone reads complete with no frames; one that does not answer within
/// its patience reads `TimedOut` with none.
async fn read_app<T, F, Fut>(
    patience: AppPatience,
    children: impl Future<Output = Result<Vec<(String, String)>, AdapterError>>,
    is_unclassified: impl Fn(&T) -> bool,
    read: F,
) -> (Vec<T>, RootRead)
where
    F: Fn((String, String)) -> Fut,
    Fut: Future<Output = Result<T, AdapterError>>,
{
    let children = match outcome(patience.children.wait(children).await) {
        Outcome::Read(children) => children,
        Outcome::Gone => return (Vec::new(), RootRead::Read),
        Outcome::Slow => return (Vec::new(), RootRead::TimedOut),
    };
    collect_frames(children, patience.frames, is_unclassified, read).await
}

/// Reads the children of each application root in full. An application whose
/// child list or frame is gone contributes no windows and reads complete; one
/// that does not answer in time contributes what it did answer and reads
/// `TimedOut`. Neither is an error.
pub(crate) fn read_frames(
    bus: &SharedBus,
    roots: &[AtspiElement],
    deadline: Deadline,
) -> Result<Vec<FrameRead>, AdapterError> {
    let shared = bus.get(deadline)?;
    within(deadline, async {
        let connection = shared.inner();
        let reads = roots.iter().map(|root| async move {
            let (nodes, read) = read_app(
                AppPatience::for_process(root.pid()),
                list_children(connection, root),
                is_unclassified,
                |(bus_name, path)| async move {
                    let frame = AtspiElement::new(bus_name, path, root.pid());
                    read_node_for(connection, &frame, ReadPurpose::Frame)
                        .await
                        .map(|(node, _)| node)
                },
            )
            .await;
            FrameRead { nodes, read }
        });
        Ok(stream::iter(reads)
            .buffered(APPS_IN_FLIGHT)
            .collect::<Vec<_>>()
            .await)
    })
}

#[cfg(test)]
#[path = "registry_frames_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "frame_payload_tests.rs"]
mod frame_payload_tests;
