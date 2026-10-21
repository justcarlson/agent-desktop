use super::pointer_read::PointerRead;
use super::sink_error::SinkError;
use crate::input::keymap::Keymap;
use crate::input::keymap_edit::KeymapEdit;

pub(crate) trait X11Sink {
    fn key(&mut self, code: u8, down: bool) -> Result<(), SinkError>;
    fn button(&mut self, code: u8, down: bool) -> Result<(), SinkError>;
    fn move_to(&mut self, x: i16, y: i16) -> Result<(), SinkError>;
    fn round_trip(&mut self) -> Result<(), SinkError>;
    fn pointer(&mut self) -> Result<PointerRead, SinkError>;
    fn keys_down(&mut self) -> Result<[u8; 32], SinkError>;
    fn keymap(&mut self) -> Result<Keymap, SinkError>;
    fn edit_keymap(&mut self, edit: &KeymapEdit) -> Result<(), SinkError>;
    fn root_size(&mut self) -> Result<(u16, u16), SinkError>;
}
