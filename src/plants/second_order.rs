use crossterm::event::{KeyCode, KeyEvent};
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, StatefulWidgetRef, Widget};

use crate::plants::Plant;
use crate::Editing;
use crate::utils::NumericInput;

/// Discrete-time second-order system:
///
/// y"(t)+2ζωy'(t)+ω^2y(t)=ω^2u(t) in continuous time
///
/// y[k]=−a1y[k−1]−a2y[k−2]+b0u[k]+b1u[k−1]+b2u[k−2] in discrete time
///
/// Bilinear (Tustin) mapping
#[allow(non_snake_case)]
#[derive(Clone)]
pub struct SecondOrderSystem {
    x: f64,
    Ts: f64,
    a: (f64, f64),      // a1, a2
    b: (f64, f64, f64), // b0, b1, b2
    u: (f64, f64, f64), // u0, u1, u2
    y_k: (f64, f64),    // y1, y2
    zeta: f64,          // damping ratio
    wn: f64,            // natural frequency
    pub edit: Option<SecondOrderEdit>,
}

#[derive(Clone)]
pub enum SecondOrderEdit {
    Zeta(NumericInput),
    Wn(NumericInput),
}

impl SecondOrderSystem {
    #[allow(non_snake_case)]
    pub fn new(zeta: f64, wn: f64, Ts: f64, prewarp: bool, y_0_1: Option<(f64, f64)>) -> Self {
        let mut plant = Self {
            x: 0.0,
            Ts,
            a: (0.0, 0.0),
            b: (0.0, 0.0, 0.0),
            u: (0.0, 0.0, 0.0),
            y_k: y_0_1.unwrap_or((0.0, 0.0)),
            zeta,
            wn,
            edit: None,
        };
        plant.update_coefficients(prewarp);
        plant
    }

    fn update_coefficients(&mut self, prewarp: bool) {
        let wn_eff = if prewarp {
            let wts2 = 0.5 * self.wn * self.Ts;
            (2.0 / self.Ts) * (wts2.tan())
        } else {
            self.wn
        };

        let k = 2.0 / self.Ts;
        let a0 = k * k + 2.0 * self.zeta * wn_eff * k + wn_eff * wn_eff;
        let a1 = 2.0 * (wn_eff * wn_eff - k * k);
        let a2 = k * k - 2.0 * self.zeta * wn_eff * k + wn_eff * wn_eff;

        let b0 = (wn_eff * wn_eff) / a0;
        let b1 = 2.0 * (wn_eff * wn_eff) / a0;
        let b2 = (wn_eff * wn_eff) / a0;

        self.a = (a1 / a0, a2 / a0);
        self.b = (b0, b1, b2);
    }

    pub fn set_zeta(&mut self, zeta: f64) {
        self.zeta = zeta;
        self.update_coefficients(true);
    }
    pub fn get_zeta(&self) -> f64 {
        self.zeta
    }
    pub fn set_wn(&mut self, wn: f64) {
        self.wn = wn;
        self.update_coefficients(true);
    }
    pub fn get_wn(&self) -> f64 {
        self.wn
    }
}

impl Plant for SecondOrderSystem {

    fn get_cursor_offsets(&self) -> (u16, u16) {

            let x_offset = match self.edit.as_ref().unwrap() {
                SecondOrderEdit::Zeta(e) => e.cursor as u16 + 8,
                SecondOrderEdit::Wn(e) => e.cursor as u16 + 6,
            };
            let y_offset = match self.edit.as_ref().unwrap() {
                SecondOrderEdit::Zeta(_) => 1,
                SecondOrderEdit::Wn(_) => 2,
            };
            (x_offset, y_offset)
        }


        fn edit(&mut self, editing: &mut Editing, k: KeyEvent) {
            // ensure one of the edits is initialized
            let edit = self.edit.get_or_insert(SecondOrderEdit::Zeta(
                NumericInput::from(self.get_zeta().to_string()),
            ));

            let input: &mut NumericInput = match edit {
                SecondOrderEdit::Zeta(e) => e,
                SecondOrderEdit::Wn(e) => e,
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
                            SecondOrderEdit::Zeta(_) => {
                                self.set_zeta(num);
                                self.edit = Some(SecondOrderEdit::Wn(
                                    NumericInput::from(self.get_wn().to_string()),
                                ));
                            }
                            SecondOrderEdit::Wn(_) => {
                                self.set_wn(num);
                                self.edit =
                                    Some(SecondOrderEdit::Zeta(NumericInput::from(
                                        self.get_zeta().to_string(),
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
        fn set_input(&mut self, u: f64) {
        self.u.2 = self.u.1;
        self.u.1 = self.u.0;
        self.u.0 = u;
    }
}


impl Iterator for SecondOrderSystem {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let y = -self.a.0 * self.y_k.0 - self.a.1 * self.y_k.1
            + self.b.0 * self.u.0
            + self.b.1 * self.u.1
            + self.b.2 * self.u.2;
        let point = (self.x, y);
        self.x += self.Ts;
        self.y_k.1 = self.y_k.0;
        self.y_k.0 = y;
        Some(point)
    }
}

impl StatefulWidgetRef for SecondOrderSystem {
    type State = Editing;
    fn render_ref(&self, area: Rect, buf: &mut Buffer, _state: &mut Self::State) {
        let paragraph = if let Some(input) = self.edit.as_ref() {
            let (zeta_line, wn_line) = match input {
                SecondOrderEdit::Zeta(zeta) => {
                    (
                        Line::from(vec![
                            Span::raw("zeta = ").white(),
                            Span::styled(zeta.value.clone(), Style::default().cyan()),
                        ]),
                        Line::from(Span::styled(
                            format!("wn = {}", self.wn),
                            Style::default(),
                        )).white()
                    )
                },
                SecondOrderEdit::Wn(wn) => {
                    (
                        Line::from(Span::styled(
                            format!("zeta = {}", self.zeta),
                            Style::default(),
                        )).white(),

                        Line::from(vec![
                            Span::raw("wn = ").white(),
                            Span::styled(wn.value.clone(), Style::default().cyan()),
                        ])
                    )
                },
            };
            let lines = vec![
                zeta_line.add_modifier(Modifier::BOLD),
                wn_line.add_modifier(Modifier::BOLD),
                Line::from(Span::styled(
                    format!("y_k = ({:.2}, {:.2})", self.y_k.0, self.y_k.1),
                    Style::default().add_modifier(Modifier::BOLD).gray(),
                )),
            ];
            Paragraph::new(lines).add_modifier(Modifier::BOLD).block(
                Block::bordered().cyan().title_top(Line::from(vec![
                    " Plant - Second Order ".into(),
                    "<ESC> ".blue().bold(),
                ])),
            )
        } else {
            let lines = vec![
                Line::from(Span::styled(
                    format!("zeta = {}", self.zeta),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("wn = {}", self.wn),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("y_k = ({:.2}, {:.2})", self.y_k.0, self.y_k.1),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            ];
            Paragraph::new(lines).block(Block::bordered().title_top(Line::from(vec![
                " Plant - Second Order ".into(),
                "<p> ".blue().bold(),
            ])))
        };
        paragraph.render(area, buf);
    }
}
