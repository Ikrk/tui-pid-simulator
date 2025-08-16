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
    referrence: StepSignal,
    referrence_data: Vec<(f64, f64)>,
    plant: FirstOrderSystem,
    plant_data: Vec<(f64, f64)>,
    window: [f64; 2],
    samples_per_window: usize,
    sampling: f64,
    controller: PIDController,
    controller_data: Vec<(f64, f64)>,
    simulation_on: bool,
    editing: Editing,
    is_controler_active: bool,
}

#[derive(Clone)]
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

#[allow(non_snake_case)]
#[derive(Clone)]
struct StepSignal {
    x: f64,
    Ts: f64,
    amplitude: f64,
    amplitude_edit: Option<NumericInput>,
}

/// Discrete-time first-order system:
///   y_{k+1} = a * y_k + b * u_k
#[allow(non_snake_case)]
#[derive(Clone)]
struct FirstOrderSystem {
    x: f64,
    Ts: f64,
    a: f64,
    b: f64,
    u: f64,
    y_k: f64,
}

// Inpired by: https://www.scilab.org/discrete-time-pid-controller-implementation
#[allow(non_snake_case)]
#[derive(Clone, Default)]
struct PIDController {
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

#[allow(non_snake_case)]
impl StepSignal {
    const fn new(Ts: f64, amplitude: f64) -> Self {
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
    #[allow(non_snake_case)]
    fn new(Ts: f64, a: f64, b: f64, y_0: Option<f64>) -> Self {
        Self {
            x: 0.0,
            Ts,
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

impl App {
    fn new() -> Self {
        let sampling = 0.1;
        let window_size = 20.0;
        let samples_per_window = (window_size / sampling) as usize;
        let mut input = StepSignal::new(sampling, 15.0);
        let mut output = FirstOrderSystem::new(sampling, 0.95, 0.05, None);
        let mut controller = PIDController::new(3.0, 1.9, 0.0, 10.0, sampling);
        let input_data = input.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        let output_data = output.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        let controller_data = controller.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        Self {
            referrence: input,
            referrence_data: input_data,
            plant: output,
            plant_data: output_data,
            window: [0.0, window_size],
            samples_per_window,
            sampling,
            controller,
            simulation_on: false,
            editing: Editing::None,
            controller_data: controller_data,
            is_controler_active: true,
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
                        KeyCode::Char('c') | KeyCode::Char('C') => {
                            self.is_controler_active = !self.is_controler_active;
                        }
                        _ => (),
                    },
                    Editing::Input => {
                        if self.referrence.amplitude_edit.is_none() {
                            self.referrence.amplitude_edit =
                                Some(NumericInput::from(self.referrence.amplitude.to_string()));
                        }

                        if let Some(edit) = self.referrence.amplitude_edit.as_mut() {
                            match k.code {
                                KeyCode::Esc => {
                                    self.editing = Editing::None;
                                    self.referrence.amplitude_edit = None;
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
                                        self.referrence.amplitude = num;
                                        self.referrence.amplitude_edit = None;
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
        if self.referrence_data.len() >= self.samples_per_window {
            self.referrence_data.drain(0..1);
        }
        self.referrence_data
            .extend(self.referrence.by_ref().take(1));

        if self.is_controler_active {
            self.controller
                .set_set_point(self.referrence_data.last().map_or(0.0, |(_, y)| *y));
            if self.controller_data.len() >= self.samples_per_window {
                self.controller_data.drain(0..1);
            }
            self.controller_data
                .extend(self.controller.by_ref().take(1));

            self.plant
                .set_input(self.controller_data.last().map_or(0.0, |(_, y)| *y));
            if self.plant_data.len() >= self.samples_per_window {
                self.plant_data.drain(0..1);
            }
            self.plant_data.extend(self.plant.by_ref().take(1));

            self.controller
                .set_plant_output(self.plant_data.last().map_or(0.0, |(_, y)| *y));
        } else {
            let set_point = self.referrence_data.last().map_or(0.0, |(_, y)| *y);
            self.controller.reset_to_setpoint(set_point);
            self.plant.set_input(set_point);
            if self.plant_data.len() >= self.samples_per_window {
                self.plant_data.drain(0..1);
            }
            self.plant_data.extend(self.plant.by_ref().take(1));
        };
        if self.plant_data.len() >= self.samples_per_window {
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
                .data(&self.referrence_data),
            Dataset::default()
                .name("output")
                .marker(symbols::Marker::Dot)
                .style(Style::default().fg(Color::Yellow))
                .data(&self.plant_data),
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
        frame.render_stateful_widget_ref(&self.referrence, input, &mut self.editing);
        frame.render_widget_ref(&self.plant, output);
        let controller_state = &mut (self.is_controler_active, self.editing.clone());
        frame.render_stateful_widget_ref(&self.controller, controller, controller_state);
        if let Editing::Input = self.editing {
            frame.set_cursor_position((
                settings.x
                    + self.referrence.amplitude_edit.as_ref().map_or_else(
                        || self.referrence.amplitude.to_string().len() as u16,
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
