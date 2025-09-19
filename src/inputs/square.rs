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

const REFERENCE_NAME: &str = "SquareSignal";

#[allow(non_snake_case)]
#[derive(Clone)]
pub struct SquareSignal {
    x: f64,
    Ts: f64,
    period: f64,
    amplitude: f64,
    duty: f64,
    phase: f64,
    pub edit: Option<SquareSignalEdit>,
}

#[derive(Clone)]
pub enum SquareSignalEdit {
    PERIOD(NumericInput),
    AMPLITUDE(NumericInput),
    DUTY(NumericInput),
}

impl PartialEq for SquareSignalEdit {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::PERIOD(_), Self::PERIOD(_)) => true,
            (Self::AMPLITUDE(_), Self::AMPLITUDE(_)) => true,
            (Self::DUTY(_), Self::DUTY(_)) => true,
            _ => false,
        }
    }
}

#[allow(non_snake_case)]
impl SquareSignal {
    pub fn new(Ts: f64, period: f64, amplitude: f64, duty: f64, phase: Option<f64>) -> Self {
        assert!(Ts > 0.0, "ts must be > 0");
        assert!(period > 0.0, "period must be > 0");
        assert!((0.0..=1.0).contains(&duty), "duty must be in [0,1]");
        let phase = phase.unwrap_or(0.0) % period;
        Self {
            x: 0.0,
            Ts,
            period,
            amplitude,
            duty,
            phase,
            edit: None,
        }
    }

    /// Return the instantaneous value at time `t` without advancing time.
    pub fn value_at(&self, t: f64) -> f64 {
        // compute local time inside period, considering phase
        // add a small epsilon to handle edge cases if you prefer inclusive behaviour
        let local = (t + self.phase) % self.period;
        if local < (self.duty * self.period) {
            self.amplitude
        } else {
            0.0
        }
    }
}

impl Default for SquareSignal {
    fn default() -> Self {
        Self::new(0.1, 10.0, 10.0, 0.5, None)
    }
}

impl Reference for SquareSignal {
    fn get_cursor_offsets(&self) -> (u16, u16) {
        let x_offset = match self.edit.as_ref().unwrap() {
            SquareSignalEdit::PERIOD(e) => e.cursor as u16 + 10,
            SquareSignalEdit::AMPLITUDE(e) => e.cursor as u16 + 13,
            SquareSignalEdit::DUTY(e) => e.cursor as u16 + 8,
        };
        let y_offset = match self.edit.as_ref().unwrap() {
            SquareSignalEdit::PERIOD(_) => 2,
            SquareSignalEdit::AMPLITUDE(_) => 3,
            SquareSignalEdit::DUTY(_) => 4,
        };
        (x_offset, y_offset)
    }

    fn edit(&mut self, editing: &mut Editing, k: crossterm::event::KeyEvent) {
        // ensure one of the edits is initialized
        let edit = self
            .edit
            .get_or_insert(SquareSignalEdit::PERIOD(NumericInput::from(
                self.period.to_string(),
            )));

        let input: &mut NumericInput = match edit {
            SquareSignalEdit::PERIOD(e) => e,
            SquareSignalEdit::AMPLITUDE(e) => e,
            SquareSignalEdit::DUTY(e) => e,
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
                        SquareSignalEdit::PERIOD(_) => self.period = num,
                        SquareSignalEdit::AMPLITUDE(_) => self.amplitude = num,
                        SquareSignalEdit::DUTY(_) => {
                            if (0.0..=1.0).contains(&num) {
                                self.duty = num
                            }
                        }
                    }
                }
                if k.code == KeyCode::Down {
                    match self.edit.as_ref().unwrap() {
                        SquareSignalEdit::PERIOD(_) => {
                            self.edit = Some(SquareSignalEdit::AMPLITUDE(NumericInput::from(
                                self.amplitude.to_string(),
                            )));
                        }
                        SquareSignalEdit::AMPLITUDE(_) => {
                            self.edit = Some(SquareSignalEdit::DUTY(NumericInput::from(
                                self.duty.to_string(),
                            )));
                        }
                        SquareSignalEdit::DUTY(_) => {
                            self.edit = Some(SquareSignalEdit::PERIOD(NumericInput::from(
                                self.period.to_string(),
                            )));
                        }
                    }
                }
                if k.code == KeyCode::Up {
                    match self.edit.as_ref().unwrap() {
                        SquareSignalEdit::PERIOD(_) => {
                            self.edit = Some(SquareSignalEdit::DUTY(NumericInput::from(
                                self.duty.to_string(),
                            )));
                        }
                        SquareSignalEdit::AMPLITUDE(_) => {
                            self.edit = Some(SquareSignalEdit::PERIOD(NumericInput::from(
                                self.period.to_string(),
                            )));
                        }
                        SquareSignalEdit::DUTY(_) => {
                            self.edit = Some(SquareSignalEdit::AMPLITUDE(NumericInput::from(
                                self.amplitude.to_string(),
                            )));
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
        self.edit = Some(SquareSignalEdit::PERIOD(NumericInput::from(
            self.period.to_string(),
        )));
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

impl Iterator for SquareSignal {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        // current sample
        let val = self.value_at(self.x);
        let out = (self.x, val);
        // advance time
        self.x += self.Ts;
        Some(out)
    }
}

impl StatefulWidgetRef for SquareSignal {
    type State = Editing;
    fn render_ref(&self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        let signal_line = Line::from(Span::styled(
            format!("Square Signal"),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        let paragraph = if let Some(input) = self.edit.as_ref() {
            let (period_line, amplitude_line, duty_line, phase_line) = match input {
                SquareSignalEdit::PERIOD(period) => (
                    Line::from(vec![
                        Span::raw("period = ").white(),
                        Span::styled(period.value.clone(), Style::default().cyan()),
                    ]),
                    Line::from(Span::styled(
                        format!("amplitude = {}", self.amplitude),
                        Style::default(),
                    ))
                    .white(),
                    Line::from(Span::styled(
                        format!("duty = {}", self.duty),
                        Style::default(),
                    ))
                    .white(),
                    Line::from(Span::styled(
                        format!("phase = {}", self.phase),
                        Style::default(),
                    ))
                    .white(),
                ),
                SquareSignalEdit::AMPLITUDE(amplitude) => (
                    Line::from(Span::styled(
                        format!("period = {}", self.period),
                        Style::default(),
                    ))
                    .white(),
                    Line::from(vec![
                        Span::raw("amplitude = ").white(),
                        Span::styled(amplitude.value.clone(), Style::default().cyan()),
                    ]),
                    Line::from(Span::styled(
                        format!("duty = {}", self.duty),
                        Style::default(),
                    ))
                    .white(),
                    Line::from(Span::styled(
                        format!("phase = {}", self.phase),
                        Style::default(),
                    ))
                    .white(),
                ),
                SquareSignalEdit::DUTY(duty) => (
                    Line::from(Span::styled(
                        format!("period = {}", self.period),
                        Style::default(),
                    ))
                    .white(),
                    Line::from(Span::styled(
                        format!("amplitude = {}", self.amplitude),
                        Style::default(),
                    ))
                    .white(),
                    Line::from(vec![
                        Span::raw("duty = ").white(),
                        Span::styled(duty.value.clone(), Style::default().cyan()),
                    ]),
                    Line::from(Span::styled(
                        format!("phase = {}", self.phase),
                        Style::default(),
                    ))
                    .white(),
                ),
            };
            let lines = vec![
                signal_line,
                period_line.add_modifier(Modifier::BOLD),
                amplitude_line.add_modifier(Modifier::BOLD),
                duty_line.add_modifier(Modifier::BOLD),
                phase_line.add_modifier(Modifier::BOLD),
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
                Line::from(Span::styled(
                    format!("duty = {}", self.duty),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("phase = {}", self.phase),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            ];
            Paragraph::new(lines)
        };
        paragraph.render(area, buf);
    }
}

register_reference!(SquareSignal, REFERENCE_NAME);
