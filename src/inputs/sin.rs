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

const REFERENCE_NAME: &str = "SinSignal";

#[allow(non_snake_case)]
#[derive(Clone)]
pub struct SinSignal {
    x: f64,
    Ts: f64,
    period: f64,
    amplitude: f64,
    pub edit: Option<SinSignalEdit>,
}

#[derive(Clone)]
pub enum SinSignalEdit {
    PERIOD(NumericInput),
    AMPLITUDE(NumericInput),
}
#[allow(non_snake_case)]
impl SinSignal {
    pub const fn new(Ts: f64, period: f64, amplitude: f64) -> Self {
        Self {
            x: 0.0,
            Ts,
            period,
            amplitude,
            edit: None
        }
    }
}

impl Default for SinSignal {
    fn default() -> Self {
        Self::new(0.1, 1.0, 10.0)
    }
}

impl Reference for SinSignal {
    fn get_cursor_offsets(&self) -> (u16, u16) {
        let x_offset = match self.edit.as_ref().unwrap() {
            SinSignalEdit::PERIOD(e) => e.cursor as u16 + 10,
            SinSignalEdit::AMPLITUDE(e) => e.cursor as u16 + 13,
        };
        let y_offset = match self.edit.as_ref().unwrap() {
            SinSignalEdit::PERIOD(_) => 2,
            SinSignalEdit::AMPLITUDE(_) => 3,
        };
        (x_offset, y_offset)
    }

    fn edit(&mut self, editing: &mut Editing, k: crossterm::event::KeyEvent) {
        // ensure one of the edits is initialized
                let edit = self
                    .edit
                    .get_or_insert(SinSignalEdit::PERIOD(NumericInput::from(self.period.to_string())));

                let input: &mut NumericInput = match edit {
                    SinSignalEdit::PERIOD(e) => e,
                    SinSignalEdit::AMPLITUDE(e) => e,
                };

                match k.code {
                    KeyCode::Esc => {
                        *editing = Editing::None;
                        self.edit = None;
                    }
                    KeyCode::Char(c) => {
                        input.insert(c);
                    }
                    KeyCode::Backspace => {
                        input.backspace();
                    }
                    KeyCode::Delete => {
                        input.delete();
                    }
                    KeyCode::Left => {
                        if input.cursor > 0 {
                            input.cursor -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if input.cursor < input.value.len() {
                            input.cursor += 1;
                        }
                    }
                    KeyCode::Down | KeyCode::Up | KeyCode::Enter => {
                        if let Some(num) = input.as_f64() {
                            match self.edit.as_ref().unwrap() {
                                SinSignalEdit::PERIOD(_) => {
                                    self.period = num;
                                    self.edit =
                                        Some(SinSignalEdit::AMPLITUDE(NumericInput::from(self.amplitude.to_string())));
                                }
                                SinSignalEdit::AMPLITUDE(_) => {
                                    self.amplitude = num;
                                    self.edit =
                                        Some(SinSignalEdit::PERIOD(NumericInput::from(self.period.to_string())));
                                }
                            }
                        }
                        if k.code == KeyCode::Enter {
                            *editing = Editing::None;
                            self.edit = None;
                        }
                    }
                    _ => {}
                }
    }

    fn set_edit(&mut self) {
        self.edit = Some(SinSignalEdit::PERIOD(NumericInput::from(self.period.to_string())));
    }

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

impl Iterator for SinSignal {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, (self.x * 1.0 / self.period).sin() * self.amplitude);
        self.x += self.Ts;
        Some(point)
    }
}

impl StatefulWidgetRef for SinSignal {
    type State = Editing;
    fn render_ref(&self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        let signal_line = Line::from(Span::styled(
            format!("Sin Signal"),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        let paragraph = if let Some(input) = self.edit.as_ref() {
            let (period_line, amplitude_line) = match input {
                SinSignalEdit::PERIOD(period) => (
                    Line::from(vec![
                        Span::raw("period = ").white(),
                        Span::styled(period.value.clone(), Style::default().cyan()),
                    ]),
                    Line::from(Span::styled(format!("amplitude = {}", self.amplitude), Style::default())).white(),
                ),
                SinSignalEdit::AMPLITUDE(amplitude) => (
                    Line::from(Span::styled(format!("period = {}", self.period), Style::default())).white(),
                    Line::from(vec![
                        Span::raw("amplitude = ").white(),
                        Span::styled(amplitude.value.clone(), Style::default().cyan()),
                    ]),
                ),
            };
            let lines = vec![
                signal_line,
                period_line.add_modifier(Modifier::BOLD),
                amplitude_line.add_modifier(Modifier::BOLD),
            ];
            Paragraph::new(lines).add_modifier(Modifier::BOLD)
        } else {
            let lines = vec![
                signal_line,
                Line::from(Span::styled(
                    format!("period = {}", self.period),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("amplitude = {}", self.amplitude),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            ];
            Paragraph::new(lines)
        };
        paragraph.render(area, buf);
    }
}

register_reference!(SinSignal, REFERENCE_NAME);
