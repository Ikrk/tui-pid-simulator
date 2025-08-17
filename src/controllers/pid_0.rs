use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, StatefulWidgetRef, Widget};

use crate::Editing;

/// PID controller implementation - discrete time version with D filtering. Backward Euler method is use for discrete time approximation.
/// Inpired by: https://www.scilab.org/discrete-time-pid-controller-implementation
#[allow(non_snake_case)]
#[derive(Clone, Default)]
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
    pub fn set_plant_output(&mut self, y: f64) {
        self.y = y;
    }
    pub fn set_set_point(&mut self, r: f64) {
        self.r = r;
        // self.x = 0.0; // reset time when set point changes
    }
    /// Reset the controller to the set point value which effectively disables the controller.
    pub fn reset_to_setpoint(&mut self, u: f64) {
        self.u.1 = u;
        self.u.2 = u;
        self.set_set_point(u);
        self.e.1 = 0.0;
        self.e.2 = 0.0;
        self.set_plant_output(u);
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

impl StatefulWidgetRef for &PIDController {
    type State = (bool, Editing);
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let mut lines = if state.0 {
            vec![Line::from(
                Span::raw("ENABLED").green().add_modifier(Modifier::BOLD),
            )]
        } else {
            vec![Line::from(
                Span::raw("DISABLED").red().add_modifier(Modifier::BOLD),
            )]
        };
        let lines_pid = vec![
            Line::from(Span::styled(
                format!("P = {}", self.Kp),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("I = {}", self.Ki),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("D = {}", self.Kd),
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
        let paragraph = Paragraph::new(lines).block(Block::bordered().title_top(Line::from(vec![
            " Controller ".into(),
            "<c> ".blue().bold(),
        ])));
        paragraph.render(area, buf);
    }
}
