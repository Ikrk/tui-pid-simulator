use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Widget, WidgetRef};

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
        }
    }

    pub fn set_input(&mut self, u: f64) {
        self.u = u;
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

impl WidgetRef for &FirstOrderSystem {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let lines = vec![
            Line::from(Span::styled(
                format!("a = {}", self.a),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("b = {}", self.b),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("y_k = {}", self.y_k),
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ];
        let paragraph = Paragraph::new(lines).block(Block::bordered().title_top(Line::from(vec![
            " System - First Order ".into(),
            "<m> ".blue().bold(),
        ])));
        paragraph.render(area, buf);
    }
}
