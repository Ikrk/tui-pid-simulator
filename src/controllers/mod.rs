use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, widgets::StatefulWidgetRef, Frame};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use crate::Editing;

pub mod pid_0;

#[macro_export]
macro_rules! register_controller {
    ($type:ty, $name:expr) => {
        #[ctor::ctor] // runs at program startup
        fn register() {
            crate::controllers::register_controller($name, || Box::new(<$type>::default()));
        }
    };
}

pub trait Controller: StatefulWidgetRef<State = (bool, Editing)> + Iterator<Item = (f64, f64)> {

    fn get_cursor_offsets(&self) -> (u16, u16);
    fn edit(&mut self, editing: &mut Editing, k: KeyEvent);

    fn set_edit(&mut self);
    fn reset(&mut self);

    fn render(&self, frame: &mut Frame, area: Rect, state: &mut (bool, Editing));
    fn name(&self) -> &'static str;
}


pub static CONTROLLER_REGISTRY: Lazy<Mutex<HashMap<&'static str, fn() -> Box<dyn Controller + Send>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn register_controller(name: &'static str, factory: fn() -> Box<dyn Controller + Send>) {
    CONTROLLER_REGISTRY.lock().unwrap().insert(name, factory);
}

#[allow(dead_code)]
pub fn get_controller_by_name(name: &str) -> Option<Box<dyn Controller + Send>> {
    CONTROLLER_REGISTRY.lock().unwrap().get(name).map(|f| f())
}
pub fn get_controller_by_index(idx: usize) -> Option<Box<dyn Controller + Send>> {
    CONTROLLER_REGISTRY.lock().unwrap().values().nth(idx).map(|f| f())
}
