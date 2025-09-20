use crossterm::event::KeyCode;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{FrameExt, Paragraph, StatefulWidgetRef, Widget};

use crate::{register_controller, Editing};
use crate::controllers::Controller;
use crate::utils::NumericInput;

const CONTROLLER_NAME: &str = "PID with Derivative Filter (backward Euler)";

/// PID controller implementation - discrete time version with D filtering. Backward Euler method is use for discrete time approximation.
/// Inpired by: https://www.scilab.org/discrete-time-pid-controller-implementation
#[allow(non_snake_case)]
#[derive(Clone)]
pub struct PIDController {
    // TODO - antiwindup
    // TODO - output saturation
    Kp: f64,             // proportional gain
    Ki: f64,             // integral gain
    Kd: f64,             // derivative gain
    e: (f64, f64, f64),  // (current, previous, before previous) error
    u: (f64, f64, f64),  // (current, previous, before previous) controller output/plant input
    y: f64,              // current output of the system
    r: f64,              // set point (reference input)
    N: f64,              // derivative filter coefficient
    Ts: f64,             // sampling time
    ku: (f64, f64), // difference equation controller output (plant input) u1 and u2 coefficients
    ke: (f64, f64, f64), // difference equation controller input (error) e0, e1 and e2 coefficients
    x: f64,         // current time
    edit: Option<PIDControllerEdit>,
}

#[derive(Clone)]
pub enum PIDControllerEdit {
    KP(NumericInput),
    KI(NumericInput),
    KD(NumericInput),
    N(NumericInput),
}

impl Default for PIDController {
    fn default() -> Self {
        PIDController::new(0.8, 2.0, 2.0, 5.0, 0.1)
    }
}

impl PIDController {
    #[allow(non_snake_case)]
    pub fn new(Kp: f64, Ki: f64, Kd: f64, N: f64, Ts: f64) -> Self {
        let mut pid = Self {
            Kp,
            Ki,
            Kd,
            e: (0.0, 0.0, 0.0),
            u: (0.0, 0.0, 0.0),
            y: 0.0,
            N,
            Ts,
            ku: (1.0, 1.0),
            ke: (1.0, 1.0, 1.0),
            x: 0.0,
            r: 0.0,
            edit: None,
        };
        pid.update_ku_ke_cooefficients();
        pid
    }
    fn update_ku_ke_cooefficients(&mut self) {
        let a0 = 1.0 + self.N * self.Ts;
        let a1 = -(2.0 + self.N * self.Ts);
        let a2 = 1.0;
        let b0 = self.Kp * (1.0 + self.N * self.Ts)
            + self.Ki * self.Ts * (1.0 + self.N * self.Ts)
            + self.Kd * self.N;
        let b1 = -(self.Kp * (2.0 + self.N * self.Ts) + self.Ki * self.Ts + 2.0 * self.Kd * self.N);
        let b2 = self.Kp + self.Kd * self.N;

        self.ku = (a1 / a0, a2 / a0);
        self.ke = (b0 / a0, b1 / a0, b2 / a0);
    }
    /// Reset the controller to the set point value which effectively disables the controller.
    pub fn reset_to_setpoint(&mut self, u: f64) {
        self.u.1 = 0.0;
        self.u.2 = 0.0;
        self.set_set_point(u);
        self.e.1 = 0.0;
        self.e.2 = 0.0;
        self.set_plant_output(u);
    }
}

impl Controller for PIDController {
    fn get_cursor_offsets(&self) -> (u16, u16) {
        let x_offset = match self.edit.as_ref().unwrap() {
            PIDControllerEdit::KP(e) => e.cursor as u16 + 6,
            PIDControllerEdit::KI(e) => e.cursor as u16 + 6,
            PIDControllerEdit::KD(e) => e.cursor as u16 + 6,
            PIDControllerEdit::N(e) => e.cursor as u16 + 5,
        };
        let y_offset = match self.edit.as_ref().unwrap() {
            PIDControllerEdit::KP(_) => 3,
            PIDControllerEdit::KI(_) => 4,
            PIDControllerEdit::KD(_) => 5,
            PIDControllerEdit::N(_) => 6,
        };
        (x_offset, y_offset)
    }

    fn edit(&mut self, editing: &mut Editing, k: crossterm::event::KeyEvent) {
        // ensure one of the edits is initialized
        let edit = self
            .edit
            .get_or_insert(PIDControllerEdit::KP(NumericInput::from(
                self.Kp.to_string(),
            )));

        let input: &mut NumericInput = match edit {
            PIDControllerEdit::KP(e) => e,
            PIDControllerEdit::KI(e) => e,
            PIDControllerEdit::KD(e) => e,
            PIDControllerEdit::N(e) => e,
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
                        PIDControllerEdit::KP(_) => self.Kp = num,
                        PIDControllerEdit::KI(_) => self.Ki = num,
                        PIDControllerEdit::KD(_) => self.Kd = num,
                        PIDControllerEdit::N(_) => self.N = num,
                    }
                }
                if k.code == KeyCode::Down {
                    match self.edit.as_ref().unwrap() {
                        PIDControllerEdit::KP(_) => {
                            self.edit = Some(PIDControllerEdit::KI(NumericInput::from(
                                self.Ki.to_string(),
                            )));
                        }
                        PIDControllerEdit::KI(_) => {
                            self.edit = Some(PIDControllerEdit::KD(NumericInput::from(
                                self.Kd.to_string(),
                            )));
                        }
                        PIDControllerEdit::KD(_) => {
                            self.edit =
                                Some(PIDControllerEdit::N(NumericInput::from(self.N.to_string())));
                        }
                        PIDControllerEdit::N(_) => {
                            self.edit = Some(PIDControllerEdit::KP(NumericInput::from(
                                self.Kp.to_string(),
                            )));
                        }
                    }
                }
                if k.code == KeyCode::Up {
                    match self.edit.as_ref().unwrap() {
                        PIDControllerEdit::KP(_) => {
                            self.edit =
                                Some(PIDControllerEdit::N(NumericInput::from(self.N.to_string())));
                        }
                        PIDControllerEdit::KI(_) => {
                            self.edit = Some(PIDControllerEdit::KP(NumericInput::from(
                                self.Kp.to_string(),
                            )));
                        }
                        PIDControllerEdit::KD(_) => {
                            self.edit = Some(PIDControllerEdit::KI(NumericInput::from(
                                self.Ki.to_string(),
                            )));
                        }
                        PIDControllerEdit::N(_) => {
                            self.edit = Some(PIDControllerEdit::KD(NumericInput::from(
                                self.Kd.to_string(),
                            )));
                        }
                    }
                }
                if k.code == KeyCode::Enter {
                    self.update_ku_ke_cooefficients();
                    *editing = Editing::None;
                    self.edit = None;
                }
            }
            _ => {}
        }
    }

    fn set_plant_output(&mut self, y: f64) {
        self.y = y;
    }
    fn set_set_point(&mut self, r: f64) {
        self.r = r;
    }

    fn set_edit(&mut self) {
        self.edit = Some(PIDControllerEdit::KP(NumericInput::from(
            self.Kp.to_string(),
        )));
    }

    fn reset(&mut self) {
        self.x = 0.0;
        self.reset_to_setpoint(0.0);
        self.set_plant_output(0.0);
    }

    fn render(&self, frame: &mut ratatui::Frame, area: Rect, state: &mut (bool, Editing)) {
        frame.render_stateful_widget_ref(self.clone(), area, state);
    }

    fn name(&self) -> &'static str {
        CONTROLLER_NAME
    }
}

impl Iterator for PIDController {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        self.e.2 = self.e.1;
        self.e.1 = self.e.0;
        self.u.2 = self.u.1;
        self.u.1 = self.u.0;
        self.e.0 = self.r - self.y; // error = set point - plant_output
        let ku1 = self.ku.0;
        let ku2 = self.ku.1;
        let controller_output = -ku1 * self.u.1 - ku2 * self.u.2
            + self.ke.0 * self.e.0
            + self.ke.1 * self.e.1
            + self.ke.2 * self.e.2;
        let point = (self.x, controller_output);
        self.x += self.Ts;
        self.u.0 = controller_output;
        Some(point)
    }
}

impl StatefulWidgetRef for PIDController {
    type State = (bool, Editing);
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let controller_line = Line::from(Span::styled(
            format!("PID with derivative filter"),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        let paragraph = if let Some(input) = self.edit.as_ref() {
            let mut lines = if state.0 {
                vec![Line::from(
                    Span::raw("ENABLED").green().add_modifier(Modifier::BOLD),
                )]
            } else {
                vec![Line::from(
                    Span::raw("DISABLED").red().add_modifier(Modifier::BOLD),
                )]
            };
            let (kp_line, ki_line, kd_line, n_line) = match input {
                PIDControllerEdit::KP(kp) => (
                    Line::from(vec![
                        Span::raw("Kp = ").white(),
                        Span::styled(kp.value.clone(), Style::default().cyan()),
                    ]),
                    Line::from(Span::styled(format!("Ki = {}", self.Ki), Style::default())).white(),
                    Line::from(Span::styled(format!("Kd = {}", self.Kd), Style::default())).white(),
                    Line::from(Span::styled(format!("N = {}", self.N), Style::default())).white(),
                ),
                PIDControllerEdit::KI(ki) => (
                    Line::from(Span::styled(format!("Kp = {}", self.Kp), Style::default())).white(),
                    Line::from(vec![
                        Span::raw("Ki = ").white(),
                        Span::styled(ki.value.clone(), Style::default().cyan()),
                    ]),
                    Line::from(Span::styled(format!("Kd = {}", self.Kd), Style::default())).white(),
                    Line::from(Span::styled(format!("N = {}", self.N), Style::default())).white(),
                ),
                PIDControllerEdit::KD(kd) => (
                    Line::from(Span::styled(format!("Kp = {}", self.Kp), Style::default())).white(),
                    Line::from(Span::styled(format!("Ki = {}", self.Ki), Style::default())).white(),
                    Line::from(vec![
                        Span::raw("Kd = ").white(),
                        Span::styled(kd.value.clone(), Style::default().cyan()),
                    ]),
                    Line::from(Span::styled(format!("N = {}", self.N), Style::default())).white(),
                ),
                PIDControllerEdit::N(n) => (
                    Line::from(Span::styled(format!("Kp = {}", self.Kp), Style::default())).white(),
                    Line::from(Span::styled(format!("Ki = {}", self.Ki), Style::default())).white(),
                    Line::from(Span::styled(format!("Kd = {}", self.Kd), Style::default())).white(),
                    Line::from(vec![
                        Span::raw("N = ").white(),
                        Span::styled(n.value.clone(), Style::default().cyan()),
                    ]),
                ),
            };
            let lines_pid = vec![
                controller_line,
                kp_line.add_modifier(Modifier::BOLD),
                ki_line.add_modifier(Modifier::BOLD),
                kd_line.add_modifier(Modifier::BOLD),
                n_line.add_modifier(Modifier::BOLD),
                Line::from(Span::styled(
                    format!("Ts = {}", self.Ts),
                    Style::default().gray().add_modifier(Modifier::BOLD),
                )),
            ];
            lines.extend(lines_pid);
            Paragraph::new(lines).add_modifier(Modifier::BOLD)
        } else {
            let mut lines = if state.0 {
                vec![Line::from(vec![
                    Span::raw("ENABLED").green().add_modifier(Modifier::BOLD),
                    Span::raw(" <space>").white(),
                ])]
            } else {
                vec![Line::from(vec![
                    Span::raw("DISABLED").red().add_modifier(Modifier::BOLD),
                    Span::raw(" <space>").white(),
                ])]
            };
            let lines_pid = vec![
                controller_line,
                Line::from(Span::styled(
                    format!("Kp = {}", self.Kp),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("Ki = {}", self.Ki),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("Kd = {}", self.Kd),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("N = {}", self.N),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
                Line::from(Span::styled(
                    format!("Ts = {}", self.Ts),
                    Style::default().add_modifier(Modifier::BOLD),
                )),
            ];
            lines.extend(lines_pid);
            Paragraph::new(lines)
        };
        paragraph.render(area, buf);
    }
}

register_controller!(PIDController, CONTROLLER_NAME);
