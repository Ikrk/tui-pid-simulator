use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, widgets::StatefulWidgetRef, Frame};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use crate::Editing;

pub mod step;
pub mod sin;

#[macro_export]
macro_rules! register_reference {
    ($type:ty, $name:expr) => {
        #[ctor::ctor] // runs at program startup
        fn register() {
            crate::inputs::register_reference($name, || Box::new(<$type>::default()));
        }
    };
}

pub trait Reference: StatefulWidgetRef<State = Editing> + Iterator<Item = (f64, f64)> {

    fn get_cursor_offsets(&self) -> (u16, u16);
    fn edit(&mut self, editing: &mut Editing, k: KeyEvent);

    fn set_edit(&mut self);
    fn reset(&mut self);

    fn render(&self, frame: &mut Frame, area: Rect, state: &mut Editing);
    fn name(&self) -> &'static str;
}


pub static REFERENCE_REGISTRY: Lazy<Mutex<HashMap<&'static str, fn() -> Box<dyn Reference + Send>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn register_reference(name: &'static str, factory: fn() -> Box<dyn Reference + Send>) {
    REFERENCE_REGISTRY.lock().unwrap().insert(name, factory);
}

#[allow(dead_code)]
pub fn get_reference_by_name(name: &str) -> Option<Box<dyn Reference + Send>> {
    REFERENCE_REGISTRY.lock().unwrap().get(name).map(|f| f())
}
pub fn get_reference_by_index(idx: usize) -> Option<Box<dyn Reference + Send>> {
    REFERENCE_REGISTRY.lock().unwrap().values().nth(idx).map(|f| f())
}
