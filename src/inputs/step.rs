use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::Modifier;
use ratatui::style::{Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, StatefulWidgetRef, Widget};

use crate::{Editing, utils::NumericInput};

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

impl Iterator for StepSignal {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, self.amplitude);
        self.x += self.Ts;
        Some(point)
    }
}

impl StatefulWidgetRef for &StepSignal {
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
            .block(Block::bordered().cyan().title_top(Line::from(vec![
                " Input signal ".into(),
                "<ESC> ".blue().bold(),
            ])))
        } else {
            Paragraph::new(Span::styled(
                format!("Set point = {}", self.amplitude),
                Style::default().add_modifier(Modifier::BOLD),
            ))
            .block(Block::bordered().title_top(Line::from(vec![
                " Input signal ".into(),
                "<i> ".blue().bold(),
            ])))
        };
        paragraph.render(area, buf);
    }
}
