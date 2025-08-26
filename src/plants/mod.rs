use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, widgets::StatefulWidgetRef, Frame};

use crate::Editing;

pub mod first_order;
pub mod second_order;

pub trait Plant: StatefulWidgetRef<State = Editing> + Iterator<Item = (f64, f64)> {

    fn get_cursor_offsets(&self) -> (u16, u16);
    fn edit(&mut self, editing: &mut Editing, k: KeyEvent);

    fn set_input(&mut self, u: f64);
    fn set_edit(&mut self);

    fn render(&self, frame: &mut Frame, area: Rect, state: &mut Editing);
}
