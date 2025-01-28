
use std::time::{Instant, Duration};

pub struct Timer {
    pub start: Instant,
    pub duration: Duration,
}

impl Timer {
    pub fn start(duration: Duration) -> Timer {
        Timer {
            start: Instant::now(),
            duration,
        }
    }

    pub fn time_out(&self) -> bool {
        self.start.elapsed() >= self.duration
    }

    pub fn reset(&mut self) {
        self.start = Instant::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test() {

        let mut timer = Timer::start(Duration::from_secs(5));
        loop {
            if timer.time_out() {
                assert_eq!(timer.time_out(), true);
            }

        }
    }
}
