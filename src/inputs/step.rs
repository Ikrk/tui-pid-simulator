use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{FrameExt, Paragraph, StatefulWidgetRef, Widget};

use crate::inputs::Reference;
use crate::register_reference;
use crate::{Editing, utils::NumericInput};

const REFERENCE_NAME: &str = "StepSignal";

#[allow(non_snake_case)]
#[derive(Clone)]
pub struct StepSignal {
    x: f64,
    pub Ts: f64,
    pub amplitude: f64,
    pub amplitude_edit: Option<NumericInput>,
}

#[allow(non_snake_case)]
impl StepSignal {
    pub const fn new(Ts: f64, amplitude: f64) -> Self {
        Self {
            x: 0.0,
            Ts,
            amplitude,
            amplitude_edit: None,
        }
    }
}

impl Default for StepSignal {
    fn default() -> Self {
        Self::new(0.1, 1.0)
    }
}

impl Reference for StepSignal {
    fn get_cursor_offsets(&self) -> (u16, u16) {
        let x_offset = self.amplitude_edit.as_ref().map_or_else(
            || self.amplitude.to_string().len() as u16,
            |a| a.cursor as u16,
        ) + 13;
        let y_offset = 1;
        (x_offset, y_offset)
    }

    fn edit(&mut self, editing: &mut Editing, k: crossterm::event::KeyEvent) {
        if self.amplitude_edit.is_none() {
            self.amplitude_edit = Some(NumericInput::from(self.amplitude.to_string()));
        }

        if let Some(edit) = self.amplitude_edit.as_mut() {
            match k.code {
                KeyCode::Esc => {
                    *editing = Editing::None;
                    self.amplitude_edit = None;
                }
                KeyCode::Char(c) => {
                    edit.insert(c);
                }
                KeyCode::Backspace => {
                    edit.backspace();
                }
                KeyCode::Delete => {
                    edit.delete();
                }
                KeyCode::Left => {
                    if edit.cursor > 0 {
                        edit.cursor -= 1;
                    }
                }
                KeyCode::Right => {
                    if edit.cursor < edit.value.len() {
                        edit.cursor += 1;
                    }
                }
                KeyCode::Enter => {
                    if let Some(num) = edit.as_f64() {
                        self.amplitude = num;
                        self.amplitude_edit = None;
                        *editing = Editing::None;
                    }
                }
                _ => {}
            }
        }
    }

    fn set_edit(&mut self) {}

    fn reset(&mut self) {
        self.x = 0.0;
    }

    fn render(&self, frame: &mut ratatui::Frame, area: Rect, state: &mut Editing) {
        frame.render_stateful_widget_ref(self.clone(), area, state);
    }

    fn name(&self) -> &'static str {
        REFERENCE_NAME
    }
}

impl Iterator for StepSignal {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, self.amplitude);
        self.x += self.Ts;
        Some(point)
    }
}

impl StatefulWidgetRef for StepSignal {
    type State = Editing;
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let paragraph = if let Editing::Reference = state {
            let amplitude = self
                .amplitude_edit
                .as_ref()
                .map_or(format!("{}", self.amplitude), |edit| edit.value.clone());
            Paragraph::new(
                Line::from(vec![
                    Span::raw("Set point = ").white(),
                    Span::styled(amplitude, Style::default().cyan()),
                ])
                .add_modifier(Modifier::BOLD),
            )
        } else {
            Paragraph::new(Span::styled(
                format!("Set point = {}", self.amplitude),
                Style::default().add_modifier(Modifier::BOLD),
            ))
        };
        paragraph.render(area, buf);
    }
}

register_reference!(StepSignal, REFERENCE_NAME);
