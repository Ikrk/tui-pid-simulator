use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Widget, WidgetRef};

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
}

impl SecondOrderSystem {
    #[allow(non_snake_case)]
    pub fn new(zeta: f64, wn: f64, Ts: f64, prewarp: bool, y_0_1: Option<(f64, f64)>) -> Self {
        let wn_eff = if prewarp {
            let wts2 = 0.5 * wn * Ts;
            (2.0 / Ts) * (wts2.tan())
        } else {
            wn
        };

        let k = 2.0 / Ts;
        let a0 = k * k + 2.0 * zeta * wn_eff * k + wn_eff * wn_eff;
        let a1 = 2.0 * (wn_eff * wn_eff - k * k);
        let a2 = k * k - 2.0 * zeta * wn_eff * k + wn_eff * wn_eff;

        let b0 = (wn_eff * wn_eff) / a0;
        let b1 = 2.0 * (wn_eff * wn_eff) / a0;
        let b2 = (wn_eff * wn_eff) / a0;
        Self {
            x: 0.0,
            Ts,
            a: (a1 / a0, a2 / a0),
            b: (b0, b1, b2),
            u: (0.0, 0.0, 0.0),
            y_k: y_0_1.unwrap_or((0.0, 0.0)),
            zeta,
            wn,
        }
    }

    pub fn set_input(&mut self, u: f64) {
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

impl WidgetRef for &SecondOrderSystem {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
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
        let paragraph = Paragraph::new(lines).block(Block::bordered().title_top(Line::from(vec![
            " System - Second Order ".into(),
            "<m> ".blue().bold(),
        ])));
        paragraph.render(area, buf);
    }
}
