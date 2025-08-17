#[derive(Clone)]
pub struct SinSignal {
    x: f64,
    interval: f64,
    period: f64,
    amplitude: f64,
}

impl SinSignal {
    pub const fn new(interval: f64, period: f64, amplitude: f64) -> Self {
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
