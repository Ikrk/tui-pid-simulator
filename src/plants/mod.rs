use crossterm::event::KeyEvent;
use ratatui::widgets::StatefulWidgetRef;

use crate::Editing;

pub mod first_order;
pub mod second_order;

pub trait Plant: StatefulWidgetRef {

    fn get_cursor_offsets(&self) -> (u16, u16);
    fn edit(&mut self, editing: &mut Editing, k: KeyEvent);

    fn set_input(&mut self, u: f64);

}
