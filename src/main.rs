/// A Ratatui example that demonstrates how to handle charts.
///
/// This example demonstrates how to draw various types of charts such as line, bar, and
/// scatter charts.
///
/// This example runs with the Ratatui library code in the branch that you are currently
/// reading. See the [`latest`] branch for the code which works with the most recent Ratatui
/// release.
///
/// [`latest`]: https://github.com/ratatui/ratatui/tree/latest
use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, KeyCode};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::symbols::{self, Marker};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, Chart, Dataset, FrameExt, GraphType, LegendPosition, Paragraph, StatefulWidgetRef,
    Widget, WidgetRef,
};
use ratatui::{DefaultTerminal, Frame};
mod utils;
use utils::NumericInput;

fn main() -> Result<()> {
    color_eyre::install()?;
    ratatui::run(|terminal| App::new().run(terminal))
}

struct App {
    input: StepSignal,
    input_data: Vec<(f64, f64)>,
    output: FirstOrderSystem,
    output_data: Vec<(f64, f64)>,
    window: [f64; 2],
    samples_per_window: usize,
    sampling: f64,
    controller: PIDController,
    simulation_on: bool,
    editing: Editing,
}

enum Editing {
    None,
    Input,
    Output,
    Controller,
}

#[derive(Clone)]
struct SinSignal {
    x: f64,
    interval: f64,
    period: f64,
    amplitude: f64,
}

#[derive(Clone)]
struct StepSignal {
    x: f64,
    interval: f64,
    amplitude: f64,
    amplitude_edit: Option<NumericInput>,
}

/// Discrete-time first-order system:
///   y_{k+1} = a * y_k + b * u_k
#[derive(Clone)]
struct FirstOrderSystem {
    x: f64,
    interval: f64,
    a: f64,
    b: f64,
    u: f64,
    y_k: f64,
}

#[derive(Clone, Default)]
struct PIDController {
    p: f64,
    i: f64,
    d: f64,
}

impl WidgetRef for &PIDController {
    fn render_ref(&self, area: Rect, buf: &mut Buffer) {
        let lines = vec![
            Line::from(Span::styled(
                format!("P = {}", self.p),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("I = {}", self.i),
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(Span::styled(
                format!("D = {}", self.d),
                Style::default().add_modifier(Modifier::BOLD),
            )),
        ];
        let paragraph = Paragraph::new(lines).block(Block::bordered().title_top(Line::from(vec![
            " Controller ".into(),
            "<c> ".blue().bold(),
        ])));
        paragraph.render(area, buf);
    }
}

impl SinSignal {
    const fn new(interval: f64, period: f64, amplitude: f64) -> Self {
        Self {
            x: 0.0,
            interval,
            period,
            amplitude,
        }
    }
}

impl Iterator for SinSignal {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, (self.x * 1.0 / self.period).sin() * self.amplitude);
        self.x += self.interval;
        Some(point)
    }
}

impl StepSignal {
    const fn new(interval: f64, amplitude: f64) -> Self {
        Self {
            x: 0.0,
            interval,
            amplitude,
            amplitude_edit: None,
        }
    }
}

impl Iterator for StepSignal {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, self.amplitude);
        self.x += self.interval;
        Some(point)
    }
}

impl StatefulWidgetRef for &StepSignal {
    type State = Editing;
    fn render_ref(&self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let paragraph = if let Editing::Input = state {
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

impl FirstOrderSystem {
    fn new(interval: f64, a: f64, b: f64, y_0: Option<f64>) -> Self {
        Self {
            x: 0.0,
            interval,
            a,
            b,
            u: 0.0,
            y_k: y_0.unwrap_or(0.0),
        }
    }

    fn set_input(&mut self, u: f64) {
        self.u = u;
    }
}

impl Iterator for FirstOrderSystem {
    type Item = (f64, f64);
    fn next(&mut self) -> Option<Self::Item> {
        let point = (self.x, self.a * self.y_k + self.b * self.u);
        self.x += self.interval;
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

impl App {
    fn new() -> Self {
        let sampling = 0.1;
        let window_size = 20.0;
        let samples_per_window = (window_size / sampling) as usize;
        let mut input = StepSignal::new(sampling, 15.0);
        let mut output = FirstOrderSystem::new(sampling, 0.95, 0.05, None);
        let input_data = input.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        let output_data = output.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        Self {
            input,
            input_data,
            output,
            output_data,
            window: [0.0, window_size],
            samples_per_window,
            sampling,
            controller: Default::default(),
            simulation_on: false,
            editing: Editing::None,
        }
    }

    fn run(mut self, terminal: &mut DefaultTerminal) -> Result<()> {
        let tick_rate = Duration::from_millis(50);
        let mut last_tick = Instant::now();
        loop {
            terminal.draw(|frame| self.render(frame))?;

            let timeout = tick_rate.saturating_sub(last_tick.elapsed());
            if !event::poll(timeout)? {
                if self.simulation_on {
                    self.on_tick();
                }
                last_tick = Instant::now();
                continue;
            }
            if let Some(k) = event::read()?.as_key_press_event() {
                match self.editing {
                    Editing::None => match k.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(()),
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            self.simulation_on = !self.simulation_on;
                        }
                        KeyCode::Char('i') | KeyCode::Char('I') => {
                            self.editing = Editing::Input;
                        }
                        _ => (),
                    },
                    Editing::Input => {
                        if self.input.amplitude_edit.is_none() {
                            self.input.amplitude_edit =
                                Some(NumericInput::from(self.input.amplitude.to_string()));
                        }

                        if let Some(edit) = self.input.amplitude_edit.as_mut() {
                            match k.code {
                                KeyCode::Esc => {
                                    self.editing = Editing::None;
                                    self.input.amplitude_edit = None;
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
                                        self.input.amplitude = num;
                                        self.input.amplitude_edit = None;
                                        self.editing = Editing::None;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => (), // Editing::Output => todo!(),
                             // Editing::Controller => todo!(),
                }
            }
        }
    }

    fn on_tick(&mut self) {
        if self.input_data.len() >= self.samples_per_window {
            self.input_data.drain(0..1);
        }
        self.input_data.extend(self.input.by_ref().take(1));

        self.output
            .set_input(self.input_data.last().map_or(0.0, |(_, y)| *y));
        if self.output_data.len() >= self.samples_per_window {
            self.output_data.drain(0..1);
        }
        self.output_data.extend(self.output.by_ref().take(1));

        if self.output_data.len() >= self.samples_per_window {
            self.window[0] += self.sampling;
            self.window[1] += self.sampling;
        }
    }

    fn render(&mut self, frame: &mut Frame) {
        let vertical = Layout::vertical([Constraint::Fill(3), Constraint::Fill(1)]);
        let [top, bottom] = frame.area().layout(&vertical);
        let horizontal = Layout::horizontal([Constraint::Length(29), Constraint::Fill(1)]);
        let [bar_chart, animated_chart] = top.layout(&horizontal);
        let [line_chart, scatter] = bottom.layout(&Layout::horizontal([Constraint::Fill(1); 2]));

        self.render_animated_chart(frame, animated_chart);
        self.render_settings(frame, bar_chart);
        render_line_chart(frame, line_chart);
        render_scatter(frame, scatter);
    }

    fn render_animated_chart(&self, frame: &mut Frame, area: Rect) {
        let x_labels = vec![
            Span::styled(
                format!("{:.1}", self.window[0]),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!(
                "{:.1}",
                f64::midpoint(self.window[0], self.window[1])
            )),
            Span::styled(
                format!("{:.1}", self.window[1]),
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ];
        let datasets = vec![
            Dataset::default()
                .name("input")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::Cyan))
                .data(&self.input_data),
            Dataset::default()
                .name("output")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::Yellow))
                .data(&self.output_data),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::bordered().title_top(
                    Line::from(vec![
                        " Start/stop the simulation ".into(),
                        "<s>".blue().bold(),
                        " Quit ".into(),
                        "<q> ".blue().bold(),
                    ])
                    .centered(),
                ),
            )
            .x_axis(
                Axis::default()
                    .title("X Axis")
                    .style(Style::default().fg(Color::Gray))
                    .labels(x_labels)
                    .bounds(self.window),
            )
            .y_axis(
                Axis::default()
                    .title("Y Axis")
                    .style(Style::default().fg(Color::Gray))
                    .labels(["-20".bold(), "0".into(), "20".bold()])
                    .bounds([-20.0, 20.0]),
            );

        frame.render_widget(chart, area);
    }

    fn render_settings(&mut self, frame: &mut Frame, settings: Rect) {
        let vertical = Layout::vertical([Constraint::Fill(1); 3]);
        let [input, output, controller] = settings.layout(&vertical);
        frame.render_stateful_widget_ref(&self.input, input, &mut self.editing);
        frame.render_widget_ref(&self.output, output);
        frame.render_widget_ref(&self.controller, controller);
        if let Editing::Input = self.editing {
            frame.set_cursor_position((
                settings.x
                    + self.input.amplitude_edit.as_ref().map_or_else(
                        || self.input.amplitude.to_string().len() as u16,
                        |a| a.cursor as u16,
                    )
                    + 13,
                settings.y + 1,
            ));
        }
    }
}

fn render_line_chart(frame: &mut Frame, area: Rect) {
    let datasets = vec![
        Dataset::default()
            .name("Line from only 2 points".italic())
            .marker(symbols::Marker::Braille)
            .style(Style::default().fg(Color::Yellow))
            .graph_type(GraphType::Line)
            .data(&[(1., 1.), (4., 4.)]),
    ];

    let chart = Chart::new(datasets)
        .block(Block::bordered().title(Line::from("Line chart").cyan().bold().centered()))
        .x_axis(
            Axis::default()
                .title("X Axis")
                .style(Style::default().gray())
                .bounds([0.0, 5.0])
                .labels(["0".bold(), "2.5".into(), "5.0".bold()]),
        )
        .y_axis(
            Axis::default()
                .title("Y Axis")
                .style(Style::default().gray())
                .bounds([0.0, 5.0])
                .labels(["0".bold(), "2.5".into(), "5.0".bold()]),
        )
        .legend_position(Some(LegendPosition::TopLeft))
        .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)));

    frame.render_widget(chart, area);
}

fn render_scatter(frame: &mut Frame, area: Rect) {
    let datasets = vec![
        Dataset::default()
            .name("Heavy")
            .marker(Marker::Dot)
            .graph_type(GraphType::Scatter)
            .style(Style::new().yellow())
            .data(&HEAVY_PAYLOAD_DATA),
        Dataset::default()
            .name("Medium".underlined())
            .marker(Marker::Braille)
            .graph_type(GraphType::Scatter)
            .style(Style::new().magenta())
            .data(&MEDIUM_PAYLOAD_DATA),
        Dataset::default()
            .name("Small")
            .marker(Marker::Dot)
            .graph_type(GraphType::Scatter)
            .style(Style::new().cyan())
            .data(&SMALL_PAYLOAD_DATA),
    ];

    let chart = Chart::new(datasets)
        .block(Block::bordered().title(Line::from("Scatter chart").cyan().bold().centered()))
        .x_axis(
            Axis::default()
                .title("Year")
                .bounds([1960., 2020.])
                .style(Style::default().fg(Color::Gray))
                .labels(["1960", "1990", "2020"]),
        )
        .y_axis(
            Axis::default()
                .title("Cost")
                .bounds([0., 75000.])
                .style(Style::default().fg(Color::Gray))
                .labels(["0", "37 500", "75 000"]),
        )
        .hidden_legend_constraints((Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)));

    frame.render_widget(chart, area);
}

// Data from https://ourworldindata.org/space-exploration-satellites
const HEAVY_PAYLOAD_DATA: [(f64, f64); 9] = [
    (1965., 8200.),
    (1967., 5400.),
    (1981., 65400.),
    (1989., 30800.),
    (1997., 10200.),
    (2004., 11600.),
    (2014., 4500.),
    (2016., 7900.),
    (2018., 1500.),
];

const MEDIUM_PAYLOAD_DATA: [(f64, f64); 29] = [
    (1963., 29500.),
    (1964., 30600.),
    (1965., 177_900.),
    (1965., 21000.),
    (1966., 17900.),
    (1966., 8400.),
    (1975., 17500.),
    (1982., 8300.),
    (1985., 5100.),
    (1988., 18300.),
    (1990., 38800.),
    (1990., 9900.),
    (1991., 18700.),
    (1992., 9100.),
    (1994., 10500.),
    (1994., 8500.),
    (1994., 8700.),
    (1997., 6200.),
    (1999., 18000.),
    (1999., 7600.),
    (1999., 8900.),
    (1999., 9600.),
    (2000., 16000.),
    (2001., 10000.),
    (2002., 10400.),
    (2002., 8100.),
    (2010., 2600.),
    (2013., 13600.),
    (2017., 8000.),
];

const SMALL_PAYLOAD_DATA: [(f64, f64); 23] = [
    (1961., 118_500.),
    (1962., 14900.),
    (1975., 21400.),
    (1980., 32800.),
    (1988., 31100.),
    (1990., 41100.),
    (1993., 23600.),
    (1994., 20600.),
    (1994., 34600.),
    (1996., 50600.),
    (1997., 19200.),
    (1997., 45800.),
    (1998., 19100.),
    (2000., 73100.),
    (2003., 11200.),
    (2008., 12600.),
    (2010., 30500.),
    (2012., 20000.),
    (2013., 10600.),
    (2013., 34500.),
    (2015., 10600.),
    (2018., 23100.),
    (2019., 17300.),
];
