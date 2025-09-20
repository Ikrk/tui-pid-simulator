# PID Control Loop Simulator
This is a toy project to simulate a PID control loop in a terminal-based user interface by using [Ratatui](https://github.com/ratatui-org/ratatui).

The simulator provides an interactive TUI where you can:

- Experiment with reference signals (step, sine, square, pulse, etc.).
- Choose between different plant models (e.g., first-order, second-order systems).
- Tune PID controller parameters (proportional, integral, derivative gains).
- Observe real-time plots of the reference, plant output, and individual PID term contributions.

The goal is not accuracy or production-grade control, but to visualize how PID controllers behave in different scenarios — directly from your terminal.

## ToDos (that may never be completed)
- Show the contribution of each PID term
- Allow setting sample time
- Add support for soft-switching when enabling/disabling the controller
- Allow setting all input signal and plant parameters
- Allow modifying charts axis settings
- Add support for dynamic axis settings
- Add basic stability analysis tools
- Add support for multiple controllers
