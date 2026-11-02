//! The renderer process: it records itself, asks the bound compositor not
//! to animate cursor layers, connects to it, shows the first control's
//! cursor at its initial point before it serves the first request, listens
//! on its socket, and serves requests until it is disabled, its session
//! ends or turns the cursor off, its compositor goes, or the compositor
//! closes a surface. While a cursor travels it polls without waiting and
//! advances every travel each frame. A live hide holder clears every
//! cursor: travels then go on unseen, and each idle tick rereads the
//! holders to clear or draw again. When an output comes, goes, moves or
//! rescales, every cursor is drawn again where it is. On the way out it
//! answers every travel still in flight, and removes the socket and record
//! only while the record still names its own generation.

use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use agent_desktop_core::{AdapterError, CursorOverlayControl, Deadline, Point};
use rustix::event::{PollFd, PollFlags, Timespec};

use super::agent_cursors::AgentCursors;
use super::cursor_animation::{draw_at, draw_within, step_travels};
use super::held_travel::{UNSEEN_STEP_MS, step_unseen};
use super::holder_reply::refresh_holders;
use super::holder_view::any_live;
use super::holder_watch::HolderWatch;
use super::initial_pose::initial_point;
use super::layer_animation::suppress_layer_animation;
use super::layer_client::LayerClient;
use super::record_read::{RecordRead, read_record, remove_if_generation};
use super::renderer_owner::RendererOwner;
use super::renderer_paths::RendererPaths;
use super::renderer_record::{RendererRecord, now_secs};
use super::renderer_reply::{Flow, answer_travels, arrived, serve_one};
use super::renderer_spawn::RENDERER_PROTOCOL;
use super::renderer_target::RendererTarget;
use super::session_rules::SessionRules;
use crate::system::hyprland::ipc::bound_request::bound_request;
use crate::system::hyprland::toplevel::bound_socket::connect_bound;

const CONNECT_MS: u64 = 4_000;
const FIRST_DRAW_MS: u64 = 1_000;
const MONITORS_MS: u64 = 1_000;
const IDLE_POLL: Duration = Duration::from_millis(250);
const LIVENESS_EVERY: Duration = Duration::from_secs(1);

/// A running renderer and what it serves.
pub(crate) struct RendererProcess {
    pub(crate) paths: RendererPaths,
    pub(crate) record: RendererRecord,
    pub(crate) client: LayerClient,
    pub(crate) cursors: AgentCursors,
    listener: UnixListener,
    compositor_dir: PathBuf,
    pub(crate) rules: SessionRules,
}

impl RendererProcess {
    /// Records this process, connects to the compositor and binds the socket.
    pub(crate) fn start(
        paths: RendererPaths,
        target: &RendererTarget,
        first: &CursorOverlayControl,
    ) -> Result<Self, AdapterError> {
        let stream = connect_bound(&target.binding)?;
        Self::start_over(stream, paths, target, first)
    }

    /// Like `start`, over a compositor connection the caller opened.
    pub(crate) fn start_over(
        stream: UnixStream,
        paths: RendererPaths,
        target: &RendererTarget,
        first: &CursorOverlayControl,
    ) -> Result<Self, AdapterError> {
        let rules = SessionRules::live();
        let key = AgentCursors::key_for(first, || (rules.multi_agent)(first.session_id()))?;
        let owner = RendererOwner::current().ok_or_else(|| {
            AdapterError::internal("The cursor renderer could not read its own process")
        })?;
        let record = RendererRecord {
            owner,
            protocol: RENDERER_PROTOCOL.to_owned(),
            session_id: first.session_id().to_owned(),
            hypr_signature: target.binding.signature.clone(),
            wayland_socket: target.binding.wayland_socket.clone(),
            created_at: now_secs(),
        };
        paths.ensure_dirs()?;
        record.write_atomic(&paths.record)?;
        suppress_layer_animation(&target.binding);
        let opened = Self::open(stream, &paths, &target.binding.wayland_socket);
        let (client, listener) = match opened {
            Ok(opened) => opened,
            Err(error) => {
                remove_if_generation(&paths, &record.owner.generation);
                return Err(error);
            }
        };
        let mut process = Self {
            paths,
            record,
            client,
            cursors: AgentCursors::new(first_point(target)),
            listener,
            compositor_dir: target
                .runtime_dir
                .join("hypr")
                .join(&target.binding.signature),
            rules,
        };
        process.cursors.cursor(&key, first).concealed.hidden =
            matches!(first, CursorOverlayControl::Hide { .. });
        if let Err(error) = process.show_first(&key) {
            process.client.release();
            remove_if_generation(&process.paths, &process.record.owner.generation);
            return Err(error);
        }
        Ok(process)
    }

    fn show_first(&mut self, key: &str) -> Result<(), AdapterError> {
        let held = any_live(&self.paths.holders);
        let Some(cursor) = self.cursors.get_mut(key) else {
            return Ok(());
        };
        let Some(pose) = cursor
            .pose
            .clone()
            .filter(|_| !cursor.concealed.any() && !held)
        else {
            return Ok(());
        };
        draw_within(&mut self.client, (key, cursor), pose, FIRST_DRAW_MS)
    }

    /// Draws the cursor under `key` where it already is, unless it is
    /// hidden, parked, held or travelling.
    pub(crate) fn redraw(&mut self, key: &str) -> Result<(), AdapterError> {
        let held = any_live(&self.paths.holders);
        let Some(cursor) = self.cursors.get_mut(key) else {
            return Ok(());
        };
        if cursor.concealed.any() || held || cursor.travel.is_some() {
            return Ok(());
        }
        match cursor.pose.clone() {
            Some(pose) => draw_at(&mut self.client, key, cursor, pose),
            None => Ok(()),
        }
    }

    fn redraw_all(&mut self) -> Result<(), AdapterError> {
        let keys: Vec<String> = self
            .cursors
            .iter_mut()
            .map(|(key, cursor)| {
                if let Some(travel) = cursor.travel.as_mut() {
                    travel.recommit_arrival();
                }
                key.clone()
            })
            .collect();
        keys.iter().try_for_each(|key| self.redraw(key))
    }

    /// Whether this renderer's session is a multi-agent session.
    pub(crate) fn multi_agent(&self) -> bool {
        (self.rules.multi_agent)(&self.record.session_id)
    }

    fn open(
        stream: UnixStream,
        paths: &RendererPaths,
        wayland_socket: &Path,
    ) -> Result<(LayerClient, UnixListener), AdapterError> {
        let deadline = Deadline::detached_after(CONNECT_MS)?;
        let client = LayerClient::over(stream, wayland_socket, deadline)?;
        let listener = UnixListener::bind(&paths.socket)
            .and_then(|listener| listener.set_nonblocking(true).map(|()| listener))
            .map_err(|error| {
                AdapterError::internal("The cursor renderer could not listen on its socket")
                    .with_platform_detail(format!("{}: {error}", paths.socket.display()))
            })?;
        Ok((client, listener))
    }

    /// Serves until a stop condition, then cleans up its own files.
    pub(crate) fn serve(mut self) -> Result<(), AdapterError> {
        let mut checked = Instant::now();
        let mut refreshed = Instant::now();
        let mut watch = HolderWatch::loaded(&self.paths.holders);
        let outcome = loop {
            if let Err(error) = self.wait_for_work(watch) {
                break Err(error);
            }
            if let Err(error) = self.client.pump() {
                break Err(error);
            }
            if self.client.take_output_change()
                && let Err(error) = self.redraw_all()
            {
                break Err(error);
            }
            if self.client.is_closed() {
                break Ok(());
            }
            if checked.elapsed() >= LIVENESS_EVERY {
                checked = Instant::now();
                if !self.still_wanted() {
                    break Ok(());
                }
            }
            if self.accept_all(&mut watch) == Flow::Exit {
                break Ok(());
            }
            if refreshed.elapsed() >= IDLE_POLL {
                refreshed = Instant::now();
                if let Err(error) = refresh_holders(&mut self, &mut watch) {
                    break Err(error);
                }
            }
            if self.step(watch) == Flow::Exit {
                break Ok(());
            }
        };
        answer_travels(&mut self.cursors, "cursor_renderer_stopped");
        self.client.release();
        remove_if_generation(&self.paths, &self.record.owner.generation);
        outcome
    }

    fn step(&mut self, watch: HolderWatch) -> Flow {
        if !self.cursors.travelling() {
            return Flow::Continue;
        }
        if watch.is_suppressed() {
            step_unseen(&mut self.cursors).iter().for_each(arrived);
            return Flow::Continue;
        }
        match step_travels(&mut self.client, &mut self.cursors) {
            Ok(streams) => {
                streams.iter().for_each(arrived);
                Flow::Continue
            }
            Err(_) => {
                answer_travels(&mut self.cursors, "cursor_renderer_draw_failed");
                Flow::Exit
            }
        }
    }

    fn wait_for_work(&self, watch: HolderWatch) -> Result<(), AdapterError> {
        let pause = match (self.cursors.travelling(), watch.is_suppressed()) {
            (true, false) => Duration::ZERO,
            (true, true) => Duration::from_millis(UNSEEN_STEP_MS),
            (false, _) => IDLE_POLL,
        };
        let timeout = Timespec::try_from(pause)
            .map_err(|_| AdapterError::internal("The renderer poll interval does not fit"))?;
        let mut fds = [
            PollFd::new(&self.listener, PollFlags::IN),
            PollFd::from_borrowed_fd(self.client.poll_fd(), PollFlags::IN),
        ];
        match rustix::event::poll(&mut fds, Some(&timeout)) {
            Ok(_) | Err(rustix::io::Errno::INTR) => Ok(()),
            Err(error) => Err(AdapterError::internal("The cursor renderer could not poll")
                .with_platform_detail(error.to_string())),
        }
    }

    fn accept_all(&mut self, watch: &mut HolderWatch) -> Flow {
        while let Ok((stream, _)) = self.listener.accept() {
            if stream.set_nonblocking(false).is_err() {
                continue;
            }
            if serve_one(self, watch, stream) == Flow::Exit {
                return Flow::Exit;
            }
        }
        Flow::Continue
    }

    fn still_wanted(&self) -> bool {
        self.compositor_dir.is_dir()
            && owns_record(&self.paths, &self.record.owner.generation)
            && (self.rules.wanted)(&self.record.session_id)
    }
}

#[cfg(test)]
impl RendererProcess {
    /// Replaces the session check, for tests that run without a session store.
    pub(crate) fn with_wanted(mut self, wanted: fn(&str) -> bool) -> Self {
        self.rules.wanted = wanted;
        self
    }

    /// Replaces the multi-agent check, for tests that run without a session store.
    pub(crate) fn with_multi_agent(mut self, multi_agent: fn(&str) -> bool) -> Self {
        self.rules.multi_agent = multi_agent;
        self
    }
}

fn first_point(target: &RendererTarget) -> Point {
    Deadline::detached_after(MONITORS_MS)
        .and_then(|deadline| bound_request(&target.binding, "j/monitors", deadline))
        .map(|body| initial_point(&body))
        .unwrap_or(Point { x: 0.0, y: 0.0 })
}

fn owns_record(paths: &RendererPaths, generation: &str) -> bool {
    matches!(read_record(&paths.record), RecordRead::Present(record) if record.owner.generation == generation)
}

#[cfg(test)]
#[path = "renderer_process_tests.rs"]
mod tests;
