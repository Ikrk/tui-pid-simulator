use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, KeyCode};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::symbols::{self, Marker};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, Borders, Chart, Clear, Dataset, FrameExt, GraphType, List, ListItem, ListState
};
use ratatui::{DefaultTerminal, Frame};
mod utils;
use utils::NumericInput;
mod controllers;
mod inputs;
mod plants;
pub use controllers::pid_0::PIDController;
pub use inputs::step::StepSignal;
pub use plants::first_order::FirstOrderSystem;

use crate::plants::{get_plant_by_index, Plant, PLANT_REGISTRY};
use crate::plants::second_order::SecondOrderSystem;

fn main() -> Result<()> {
    color_eyre::install()?;
    ratatui::run(|terminal| App::new().run(terminal))
}

struct App {
    referrence: StepSignal,
    referrence_data: Vec<(f64, f64)>,
    plant: Box<dyn Plant>,
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
pub enum Editing {
    None,
    Reference,
    Plant,
    PlantType(Option<usize>),
    Controller,
}

const WINDOW_SIZE: f64 = 20.0;
impl App {
    fn new() -> Self {
        let sampling = 0.1;
        let samples_per_window = (WINDOW_SIZE / sampling) as usize;
        let mut input = StepSignal::new(sampling, 15.0);
        let mut plant = Box::new(SecondOrderSystem::default());
        let mut controller = PIDController::new(0.8, 2.0, 2.0, 5.0, sampling);
        let input_data = input.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        let output_data = plant.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        let controller_data = controller.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        Self {
            referrence: input,
            referrence_data: input_data,
            plant,
            plant_data: output_data,
            window: [0.0, WINDOW_SIZE],
            samples_per_window,
            sampling,
            controller,
            simulation_on: false,
            editing: Editing::None,
            controller_data: controller_data,
            is_controler_active: true,
        }
    }

    fn reset(&mut self) {
        self.referrence.reset();
        self.controller.reset();
        self.referrence_data = self.referrence.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        self.plant_data = self.plant.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        self.controller_data = self.controller.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        self.window = [0.0, WINDOW_SIZE];
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
            // Terminate the program if the user presses 'q' or 'Q' and true is returned
            if self.event_handler()? {
                return Ok(());
            }
        }
    }

    /// Handles user input events.
    ///
    /// @returns Ok(true) if the user wants to quit the program.
    fn event_handler(&mut self) -> Result<bool> {
        if let Some(k) = event::read()?.as_key_press_event() {
            match self.editing {
                Editing::None => match k.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => return Ok(true),
                    KeyCode::Char('s') | KeyCode::Char('S') => {
                        self.simulation_on = !self.simulation_on;
                    }
                    KeyCode::Char('i') | KeyCode::Char('I') => {
                        self.editing = Editing::Reference;
                    }
                    KeyCode::Char('p') => {
                        self.editing = Editing::Plant;
                        self.plant.set_edit();
                    }
                    KeyCode::Char('P') => {
                        self.editing = Editing::PlantType(None);
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        self.is_controler_active = !self.is_controler_active;
                    }
                    _ => (),
                },
                Editing::Reference => {
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
                Editing::Plant => {
                    self.plant.edit(&mut self.editing, k);
                }
                Editing::PlantType(idx) => match k.code {
                    KeyCode::Esc => {
                        self.editing = Editing::None;
                    }
                    KeyCode::Down => {
                        if let Some(idx) = idx {
                            let plants_count = PLANT_REGISTRY.lock().unwrap().len();
                            if idx + 1 < plants_count {
                                self.editing = Editing::PlantType(Some(idx + 1));
                            } else {
                                self.editing = Editing::PlantType(Some(0));
                            }
                        } else {
                            self.editing = Editing::PlantType(Some(0));
                        }
                    }
                    KeyCode::Up => {
                        if let Some(idx) = idx {
                            let plants_count = PLANT_REGISTRY.lock().unwrap().len();
                            if idx == 0 {
                                self.editing = Editing::PlantType(Some(plants_count - 1));
                            } else {
                                self.editing = Editing::PlantType(Some(idx - 1));
                            }
                        } else {
                            self.editing = Editing::PlantType(Some(0));
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(selected_idx) = idx {
                            let current = self.plant.name();
                            let current_idx = PLANT_REGISTRY.lock().unwrap().keys().position(|n| *n == current).unwrap_or(0);
                            if current_idx != selected_idx {
                                self.plant = get_plant_by_index(selected_idx).unwrap();
                                self.reset();
                            }
                        }
                        self.editing = Editing::None;
                    }
                    _ => {}
                },
                _ => (),
                // Editing::Controller => todo!(),
            }
        }
        Ok(false)
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
            // self.controller.reset_to_setpoint(set_point);
            if self.controller_data.len() >= self.samples_per_window {
                self.controller_data.drain(0..1);
            }
            let last_controller_output = self.controller_data.last().map_or(0.0, |(_, y)| *y);
            let x = self.controller.by_ref().take(1).next().unwrap_or((0.0, 0.0)).0;
            self.controller_data.push((x, last_controller_output));

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
        let vertical = Layout::vertical([Constraint::Fill(3), Constraint::Fill(2)]);
        let [top, bottom] = frame.area().layout(&vertical);
        let horizontal = Layout::horizontal([Constraint::Length(29), Constraint::Fill(1)]);
        let [bar_chart, animated_chart] = top.layout(&horizontal);
        let [controller_chart, scatter] =
            bottom.layout(&Layout::horizontal([Constraint::Fill(1); 2]));

        self.render_animated_chart(frame, animated_chart);
        self.render_settings(frame, bar_chart);
        self.render_controller_chart(frame, controller_chart);
        render_scatter(frame, scatter);
        self.render_edit_popup(frame);
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
        let y_labels = vec![
            Span::styled(
                format!("{:.1}", -20.0),
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(format!("{:.1}", 0.0)),
            Span::styled(
                format!("{:.1}", 20.0),
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
                .marker(symbols::Marker::Braille)
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
                    // .labels(["-20".bold(), "0".into(), "20".bold()])
                    .labels(y_labels)
                    .bounds([-20.0, 20.0]),
            );

        frame.render_widget(chart, area);
    }

    fn render_settings(&mut self, frame: &mut Frame, settings: Rect) {
        let vertical = Layout::vertical([Constraint::Fill(1); 3]);
        let [reference, plant, controller] = settings.layout(&vertical);
        frame.render_stateful_widget_ref(&self.referrence, reference, &mut self.editing);

        let outer_plant_block = if let Editing::Plant = self.editing {
            Block::bordered().title_top(Line::from(vec![" Plant ".into(), "<ESC> ".blue().bold()])).cyan()
        } else {
            Block::bordered().title_top(Line::from(vec![" Plant ".into(), "<p/P> ".blue().bold()]))
        };
        let inner_plant_area = outer_plant_block.inner(plant);
        frame.render_widget(outer_plant_block, plant);

        self.plant
            .render(frame, inner_plant_area, &mut self.editing);
        let controller_state = &mut (self.is_controler_active, self.editing.clone());
        frame.render_stateful_widget_ref(&self.controller, controller, controller_state);
        self.render_settings_cursor(frame, reference, plant, controller);
    }

    fn render_settings_cursor(
        &self,
        frame: &mut Frame,
        reference: Rect,
        plant: Rect,
        _controller: Rect,
    ) {
        match self.editing {
            Editing::Reference => frame.set_cursor_position((
                reference.x
                    + self.referrence.amplitude_edit.as_ref().map_or_else(
                        || self.referrence.amplitude.to_string().len() as u16,
                        |a| a.cursor as u16,
                    )
                    + 13,
                reference.y + 1,
            )),
            Editing::Plant => {
                let (x_offset, y_offset) = self.plant.get_cursor_offsets();
                frame.set_cursor_position((plant.x + x_offset, plant.y + y_offset))
            }
            _ => (),
        }
    }
    fn render_controller_chart(&self, frame: &mut Frame, area: Rect) {
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
                .name("controller output")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Yellow))
                .data(&self.controller_data),
        ];

        let chart = Chart::new(datasets)
            .block(
                Block::bordered()
                    .title_top(Line::from(vec![" Controller output ".into()]).centered()),
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
                    .labels(["-30".bold(), "0".into(), "30".bold()])
                    .bounds([-30.0, 30.0]),
            );

        frame.render_widget(chart, area);
    }
    fn render_edit_popup(&mut self, frame: &mut Frame) {
        if let Editing::PlantType(idx) = self.editing {
            let popup_block = Block::default()
                .title("Choose a plant type (ESC to close)")
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black).fg(Color::White));

            let area = centered_rect(25, 25, frame.area());
            frame.render_widget(Clear, area);
            frame.render_widget(popup_block, area);
            let current = self.plant.name();
            // Build list items from registry
                let items: Vec<ListItem> = PLANT_REGISTRY.lock().unwrap()
                    .keys()
                    .map(|name| ListItem::new(Span::raw(*name)))
                    .collect();

                let list = List::new(items)
                    .block(Block::default().borders(Borders::ALL).title("Choose a plant type (ESC to close)"))
                    .highlight_style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol(">> ");

                // Figure out which index corresponds to `current`
                let selected_idx = idx.or_else(||PLANT_REGISTRY.lock().unwrap().keys().position(|n| *n == current) );
                self.editing = Editing::PlantType(selected_idx);

                // Setup list state with highlighted element
                let mut state = ListState::default();
                if let Some(idx) = selected_idx {
                    state.select(Some(idx));
                }

                frame.render_stateful_widget(list, area, &mut state);
        }
    }
}
/// helper function to create a centered rect using up certain percentage of the available rect `r`
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    // Cut the given rectangle into three vertical pieces
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    // Then cut the middle vertical piece into three width-wise pieces
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1] // Return the middle chunk
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
