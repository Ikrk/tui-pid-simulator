use crossterm::event::KeyEvent;
use ratatui::{layout::Rect, widgets::StatefulWidgetRef, Frame};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use crate::Editing;

pub mod first_order;
pub mod second_order;

#[macro_export]
macro_rules! register_plant {
    ($type:ty, $name:expr) => {
        #[ctor::ctor] // runs at program startup
        fn register() {
            crate::plants::register_plant($name, || Box::new(<$type>::default()));
        }
    };
}

pub trait Plant: StatefulWidgetRef<State = Editing> + Iterator<Item = (f64, f64)> {

    fn get_cursor_offsets(&self) -> (u16, u16);
    fn edit(&mut self, editing: &mut Editing, k: KeyEvent);

    fn set_input(&mut self, u: f64);
    fn set_edit(&mut self);

    fn render(&self, frame: &mut Frame, area: Rect, state: &mut Editing);
    fn name(&self) -> &'static str;
}


pub static PLANT_REGISTRY: Lazy<Mutex<HashMap<&'static str, fn() -> Box<dyn Plant + Send>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub fn register_plant(name: &'static str, factory: fn() -> Box<dyn Plant + Send>) {
    PLANT_REGISTRY.lock().unwrap().insert(name, factory);
}

#[allow(dead_code)]
pub fn get_plant_by_name(name: &str) -> Option<Box<dyn Plant + Send>> {
    PLANT_REGISTRY.lock().unwrap().get(name).map(|f| f())
}
pub fn get_plant_by_index(idx: usize) -> Option<Box<dyn Plant + Send>> {
    PLANT_REGISTRY.lock().unwrap().values().nth(idx).map(|f| f())
}
