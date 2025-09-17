use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{FrameExt, Paragraph, StatefulWidgetRef, Widget};

use crate::{register_plant,Editing};
use crate::plants::Plant;
use crate::utils::NumericInput;

const PLANT_NAME: &str = "FirstOrderSystem";

/// Discrete-time first-order system:
///   y_{k+1} = a * y_k + b * u_k
#[allow(non_snake_case)]
#[derive(Clone)]
pub struct FirstOrderSystem {
    x: f64,
    Ts: f64,
    a: f64,
    b: f64,
    u: f64,
    y_k: f64,
    pub edit: Option<FirstOrderEdit>,
}

#[derive(Clone)]
pub enum FirstOrderEdit {
    A(NumericInput),
    B(NumericInput),
}

impl FirstOrderSystem {
    #[allow(non_snake_case)]
    pub fn new(Ts: f64, a: f64, b: f64, y_0: Option<f64>) -> Self {
        Self {
            x: 0.0,
            Ts,
            a,
            b,
            u: 0.0,
            y_k: y_0.unwrap_or(0.0),
            edit: None,
        }
    }
    pub fn set_ts(&mut self, ts: f64) {
        self.Ts = ts;
    }

}

impl Default for FirstOrderSystem {
    fn default() -> Self {
        Self::new(0.1, 0.95, 0.05, None)
    }
}

impl Plant for FirstOrderSystem {
    fn get_cursor_offsets(&self) -> (u16, u16) {
        let x_offset = match self.edit.as_ref().unwrap() {
            FirstOrderEdit::A(e) => e.cursor as u16 + 5,
            FirstOrderEdit::B(e) => e.cursor as u16 + 5,
        };
        let y_offset = match self.edit.as_ref().unwrap() {
            FirstOrderEdit::A(_) => 2,
            FirstOrderEdit::B(_) => 3,
        };
        (x_offset, y_offset)
    }

    fn edit(&mut self, editing: &mut crate::Editing, k: crossterm::event::KeyEvent) {
        // ensure one of the edits is initialized
        let edit = self
            .edit
            .get_or_insert(FirstOrderEdit::A(NumericInput::from(self.a.to_string())));

        let input: &mut NumericInput = match edit {
            FirstOrderEdit::A(e) => e,
            FirstOrderEdit::B(e) => e,
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
                        FirstOrderEdit::A(_) => {
                            self.a = num;
                            self.edit =
                                Some(FirstOrderEdit::B(NumericInput::from(self.b.to_string())));
                        }
                        FirstOrderEdit::B(_) => {
                            self.b = num;
                            self.edit =
                                Some(FirstOrderEdit::A(NumericInput::from(self.a.to_string())));
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

    fn set_input(&mut self, u: f64) {
        self.u = u;
    }

    fn set_edit(&mut self) {
        self.edit = Some(FirstOrderEdit::A(NumericInput::from(self.a.to_string())));
    }

    fn render(&self, frame: &mut ratatui::Frame, area: Rect, state: &mut crate::Editing) {
        frame.render_stateful_widget_ref(self.clone(), area, state);
    }
    fn name(&self) -> &'static str {
        PLANT_NAME
    }
}

impl Iterator for FirstOrderSystem {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, self.a * self.y_k + self.b * self.u);
        self.x += self.Ts;
        self.y_k = point.1;
        Some(point)
    }
}

impl StatefulWidgetRef for FirstOrderSystem {
    type State = Editing;
    fn render_ref(&self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        let format_num = |val: f64| {
            if val.abs() < 0.01 && val != 0.0 {
                format!("{:.2e}", val) // scientific notation
            } else {
                format!("{:.2}", val) // normal fixed 2 decimals
            }
        };
        let plant_line = Line::from(Span::styled(
            format!("First Order"),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        let paragraph = if let Some(input) = self.edit.as_ref() {
            let (zeta_line, wn_line) = match input {
                FirstOrderEdit::A(a) => (
                    Line::from(vec![
                        Span::raw("a = ").white(),
                        Span::styled(a.value.clone(), Style::default().cyan()),
                    ]),
                    Line::from(Span::styled(format!("b = {}", self.b), Style::default())).white(),
                ),
                FirstOrderEdit::B(b) => (
                    Line::from(Span::styled(format!("a = {}", self.a), Style::default())).white(),
                    Line::from(vec![
                        Span::raw("b = ").white(),
                        Span::styled(b.value.clone(), Style::default().cyan()),
                    ]),
                ),
            };
            let lines = vec![
                plant_line,
                zeta_line.add_modifier(Modifier::BOLD),
                wn_line.add_modifier(Modifier::BOLD),
                Line::from(Span::styled(
                    format!("y_k = {}", format_num(self.y_k),),
                    Style::default().add_modifier(Modifier::BOLD).gray(),
                )),
            ];
            Paragraph::new(lines).add_modifier(Modifier::BOLD)
        } else {
            let lines = vec![
                plant_line,
                Line::from(Span::styled(
                    format!("a = {}", self.a),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("b = {}", self.b),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("y_k = {}", format_num(self.y_k),),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            ];
            Paragraph::new(lines)
        };
        paragraph.render(area, buf);
    }
}

register_plant!(FirstOrderSystem,PLANT_NAME);
