use std::time::{Duration, Instant};

use color_eyre::Result;
use crossterm::event::{self, KeyCode};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Axis, Block, Borders, Chart, Clear, Dataset, List, ListItem, ListState,
};
use ratatui::{symbols, DefaultTerminal, Frame};
mod utils;
mod controllers;
mod inputs;
mod plants;
pub use controllers::pid_0::PIDController;
pub use inputs::step::StepSignal;
pub use plants::first_order::FirstOrderSystem;

use crate::controllers::{get_controller_by_index, Controller, CONTROLLER_REGISTRY};
use crate::inputs::{get_reference_by_index, Reference, REFERENCE_REGISTRY};
use crate::plants::second_order::SecondOrderSystem;
use crate::plants::{PLANT_REGISTRY, Plant, get_plant_by_index};

fn main() -> Result<()> {
    color_eyre::install()?;
    ratatui::run(|terminal| App::new().run(terminal))
}

struct App {
    reference: Box<dyn Reference>,
    reference_data: Vec<(f64, f64)>,
    plant: Box<dyn Plant>,
    plant_data: Vec<(f64, f64)>,
    window: [f64; 2],
    samples_per_window: usize,
    sampling: f64,
    controller: Box<dyn Controller>,
    controller_data: Vec<(f64, f64)>,
    simulation_on: bool,
    editing: Editing,
    is_controler_active: bool,
}

#[derive(Clone)]
pub enum Editing {
    None,
    Reference,
    ReferenceType(Option<usize>),
    Plant,
    PlantType(Option<usize>),
    Controller,
    ControllerType(Option<usize>),
}

const WINDOW_SIZE: f64 = 20.0;
impl App {
    fn new() -> Self {
        let sampling = 0.1;
        let samples_per_window = (WINDOW_SIZE / sampling) as usize;
        let mut input = Box::new(StepSignal::default());
        let mut plant = Box::new(SecondOrderSystem::default());
        let mut controller = Box::new(PIDController::default());
        let input_data = input.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        let output_data = plant.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        let controller_data = controller.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        Self {
            reference: input,
            reference_data: input_data,
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
        self.reference.reset();
        self.plant.reset();
        self.controller.reset();
        self.reference_data = self.reference.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        self.plant_data = self.plant.by_ref().take(0).collect::<Vec<(f64, f64)>>();
        self.controller_data = self
            .controller
            .by_ref()
            .take(0)
            .collect::<Vec<(f64, f64)>>();
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
                    KeyCode::Char('i') => {
                        self.editing = Editing::Reference;
                        self.reference.set_edit();
                    }
                    KeyCode::Char('I') => {
                        self.editing = Editing::ReferenceType(None);
                    }
                    KeyCode::Char('p') => {
                        self.editing = Editing::Plant;
                        self.plant.set_edit();
                    }
                    KeyCode::Char('P') => {
                        self.editing = Editing::PlantType(None);
                    }
                    KeyCode::Char(' ') => {
                        self.is_controler_active = !self.is_controler_active;
                    }
                    KeyCode::Char('c') => {
                        self.editing = Editing::Controller;
                        self.controller.set_edit();
                    }
                    KeyCode::Char('C') => {
                        self.editing = Editing::ControllerType(None);
                    }
                    _ => (),
                },
                Editing::Reference => {
                    self.reference.edit(&mut self.editing, k);
                }
                Editing::Plant => {
                    self.plant.edit(&mut self.editing, k);
                }
                Editing::Controller => {
                    self.controller.edit(&mut self.editing, k);
                }
                Editing::ReferenceType(idx) => match k.code {
                    KeyCode::Esc => {
                        self.editing = Editing::None;
                    }
                    KeyCode::Down => {
                        if let Some(idx) = idx {
                            let refs_count = REFERENCE_REGISTRY.lock().unwrap().len();
                            if idx + 1 < refs_count {
                                self.editing = Editing::ReferenceType(Some(idx + 1));
                            } else {
                                self.editing = Editing::ReferenceType(Some(0));
                            }
                        } else {
                            self.editing = Editing::ReferenceType(Some(0));
                        }
                    }
                    KeyCode::Up => {
                        if let Some(idx) = idx {
                            let plants_count = REFERENCE_REGISTRY.lock().unwrap().len();
                            if idx == 0 {
                                self.editing = Editing::ReferenceType(Some(plants_count - 1));
                            } else {
                                self.editing = Editing::ReferenceType(Some(idx - 1));
                            }
                        } else {
                            self.editing = Editing::ReferenceType(Some(0));
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(selected_idx) = idx {
                            let current = self.reference.name();
                            let current_idx = REFERENCE_REGISTRY
                                .lock()
                                .unwrap()
                                .keys()
                                .position(|n| *n == current)
                                .unwrap_or(0);
                            if current_idx != selected_idx {
                                self.reference = get_reference_by_index(selected_idx).unwrap();
                                self.reset();
                            }
                        }
                        self.editing = Editing::None;
                    }
                    _ => {}
                },
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
                            let current_idx = PLANT_REGISTRY
                                .lock()
                                .unwrap()
                                .keys()
                                .position(|n| *n == current)
                                .unwrap_or(0);
                            if current_idx != selected_idx {
                                self.plant = get_plant_by_index(selected_idx).unwrap();
                                self.reset();
                            }
                        }
                        self.editing = Editing::None;
                    }
                    _ => {}
                },
                Editing::ControllerType(idx) => match k.code {
                    KeyCode::Esc => {
                        self.editing = Editing::None;
                    }
                    KeyCode::Down => {
                        if let Some(idx) = idx {
                            let controllers_count = CONTROLLER_REGISTRY.lock().unwrap().len();
                            if idx + 1 < controllers_count {
                                self.editing = Editing::ControllerType(Some(idx + 1));
                            } else {
                                self.editing = Editing::ControllerType(Some(0));
                            }
                        } else {
                            self.editing = Editing::ControllerType(Some(0));
                        }
                    }
                    KeyCode::Up => {
                        if let Some(idx) = idx {
                            let controllers_count = CONTROLLER_REGISTRY.lock().unwrap().len();
                            if idx == 0 {
                                self.editing = Editing::ControllerType(Some(controllers_count - 1));
                            } else {
                                self.editing = Editing::ControllerType(Some(idx - 1));
                            }
                        } else {
                            self.editing = Editing::ControllerType(Some(0));
                        }
                    }
                    KeyCode::Enter => {
                        if let Some(selected_idx) = idx {
                            let current = self.controller.name();
                            let current_idx = CONTROLLER_REGISTRY
                                .lock()
                                .unwrap()
                                .keys()
                                .position(|n| *n == current)
                                .unwrap_or(0);
                            if current_idx != selected_idx {
                                self.controller = get_controller_by_index(selected_idx).unwrap();
                                self.reset();
                            }
                        }
                        self.editing = Editing::None;
                    }
                    _ => {}
                },
            }
        }
        Ok(false)
    }

    fn on_tick(&mut self) {
        if self.reference_data.len() >= self.samples_per_window {
            self.reference_data.drain(0..1);
        }
        self.reference_data.extend(self.reference.by_ref().take(1));

        if self.is_controler_active {
            self.controller
                .set_set_point(self.reference_data.last().map_or(0.0, |(_, y)| *y));
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
            let set_point = self.reference_data.last().map_or(0.0, |(_, y)| *y);
            // self.controller.reset_to_setpoint(set_point);
            if self.controller_data.len() >= self.samples_per_window {
                self.controller_data.drain(0..1);
            }
            let last_controller_output = self.controller_data.last().map_or(0.0, |(_, y)| *y);
            let x = self
                .controller
                .by_ref()
                .take(1)
                .next()
                .unwrap_or((0.0, 0.0))
                .0;
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
        let horizontal = Layout::horizontal([Constraint::Length(29), Constraint::Fill(1)]);
        let [settings, charts] = frame.area().layout(&horizontal);
        let vertical = Layout::vertical([Constraint::Fill(3), Constraint::Fill(2)]);
        let [top, bottom] = charts.layout(&vertical);

        self.render_input_output_charts(frame, top);
        self.render_settings(frame, settings);
        self.render_controller_chart(frame, bottom);
        self.render_edit_popup(frame);
    }

    fn render_input_output_charts(&self, frame: &mut Frame, area: Rect) {
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
                .name("reference")
                .marker(symbols::Marker::Braille)
                .style(Style::default().fg(Color::Cyan))
                .data(&self.reference_data),
            Dataset::default()
                .name("plant output")
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
                    .labels(y_labels)
                    .bounds([-20.0, 20.0]),
            );

        frame.render_widget(chart, area);
    }

    fn render_settings(&mut self, frame: &mut Frame, settings: Rect) {
        let vertical = Layout::vertical([Constraint::Fill(1); 3]);
        let [reference, plant, controller] = settings.layout(&vertical);

        let outer_ref_block = if let Editing::Reference = self.editing {
            Block::bordered()
                .title_top(Line::from(vec![" Reference ".into(), "<ESC> ".blue().bold()]))
                .cyan()
        } else {
            Block::bordered().title_top(Line::from(vec![" Reference ".into(), "<i/I> ".blue().bold()]))
        };
        let inner_ref_area = outer_ref_block.inner(reference);
        frame.render_widget(outer_ref_block, reference);
        self.reference
            .render(frame, inner_ref_area, &mut self.editing);

        let outer_plant_block = if let Editing::Plant = self.editing {
            Block::bordered()
                .title_top(Line::from(vec![" Plant ".into(), "<ESC> ".blue().bold()]))
                .cyan()
        } else {
            Block::bordered().title_top(Line::from(vec![" Plant ".into(), "<p/P> ".blue().bold()]))
        };
        let inner_plant_area = outer_plant_block.inner(plant);
        frame.render_widget(outer_plant_block, plant);
        self.plant
            .render(frame, inner_plant_area, &mut self.editing);

        let controller_state = &mut (self.is_controler_active, self.editing.clone());
        let outer_controller_block = if let Editing::Controller = self.editing {
            Block::bordered()
                .title_top(Line::from(vec![" Controller ".into(), "<ESC> ".blue().bold()]))
                .cyan()
        } else {
            Block::bordered().title_top(Line::from(vec![" Controller ".into(), "<c/C> ".blue().bold()]))
        };
        let inner_controller_area = outer_controller_block.inner(controller);
        frame.render_widget(outer_controller_block, controller);
        self.controller
            .render(frame, inner_controller_area, controller_state);

        self.render_settings_cursor(frame, reference, plant, controller);
    }

    fn render_settings_cursor(
        &self,
        frame: &mut Frame,
        reference: Rect,
        plant: Rect,
        controller: Rect,
    ) {
        match self.editing {
            Editing::Reference => {
                let (x_offset, y_offset) = self.reference.get_cursor_offsets();
                frame.set_cursor_position((reference.x + x_offset, reference.y + y_offset))
            }
            Editing::Plant => {
                let (x_offset, y_offset) = self.plant.get_cursor_offsets();
                frame.set_cursor_position((plant.x + x_offset, plant.y + y_offset))
            }
            Editing::Controller => {
                let (x_offset, y_offset) = self.controller.get_cursor_offsets();
                frame.set_cursor_position((controller.x + x_offset, controller.y + y_offset))
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
                    .labels(["-30.0".bold(), "0.0".into(), "30.0".bold()])
                    .bounds([-30.0, 30.0]),
            );

        frame.render_widget(chart, area);
    }
    fn render_edit_popup(&mut self, frame: &mut Frame) {
        let (selected_idx, r#type, items) = match self.editing {
            Editing::ReferenceType(idx) => {
                let items: Vec<ListItem> = REFERENCE_REGISTRY
                    .lock()
                    .unwrap()
                    .keys()
                    .map(|name| ListItem::new(Span::raw(*name)))
                    .collect();

                let current = self.reference.name();
                // Figure out which index corresponds to `current`
                let selected_idx = idx.or_else(|| {
                    REFERENCE_REGISTRY
                        .lock()
                        .unwrap()
                        .keys()
                        .position(|n| *n == current)
                });
                self.editing = Editing::ReferenceType(selected_idx);
                (selected_idx, "reference", items)
            }
            Editing::PlantType(idx) => {
                let items: Vec<ListItem> = PLANT_REGISTRY
                    .lock()
                    .unwrap()
                    .keys()
                    .map(|name| ListItem::new(Span::raw(*name)))
                    .collect();

                let current = self.plant.name();
                // Figure out which index corresponds to `current`
                let selected_idx = idx.or_else(|| {
                    PLANT_REGISTRY
                        .lock()
                        .unwrap()
                        .keys()
                        .position(|n| *n == current)
                });
                self.editing = Editing::PlantType(selected_idx);
                (selected_idx, "plant", items)
            }
            Editing::ControllerType(idx) => {
                let items: Vec<ListItem> = CONTROLLER_REGISTRY
                    .lock()
                    .unwrap()
                    .keys()
                    .map(|name| ListItem::new(Span::raw(*name)))
                    .collect();

                let current = self.controller.name();
                // Figure out which index corresponds to `current`
                let selected_idx = idx.or_else(|| {
                    CONTROLLER_REGISTRY
                        .lock()
                        .unwrap()
                        .keys()
                        .position(|n| *n == current)
                });
                self.editing = Editing::ControllerType(selected_idx);
                (selected_idx, "controller", items)
            }
            _ => return,
        };
        let title = format!("Choose a {} type (ESC to close)", r#type);
        let popup_block = Block::default()
            .title(title.clone())
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black).fg(Color::White));

        let area = centered_rect(25, 25, frame.area());
        frame.render_widget(Clear, area);
        frame.render_widget(popup_block, area);
        // Build list items from registry
        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(title))
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol(">> ");

        // Setup list state with highlighted element
        let mut state = ListState::default();
        if let Some(idx) = selected_idx {
            state.select(Some(idx));
        }

        frame.render_stateful_widget(list, area, &mut state);
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
