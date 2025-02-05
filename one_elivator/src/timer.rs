
use std::time::{Instant, Duration};
use crossbeam_channel as cbc;
use std::thread::{spawn, sleep};


pub fn start_timer (chanel: &cbc::Sender<()>, duration: Duration) {
    let tx = chanel.clone();
    spawn(move || {
        sleep(duration);
        let _ =  tx.send(()).unwrap();
    });
}




/* #[derive(Debug, Clone)]
pub struct Timer {
    pub start: Instant,
    duration: Duration,
    pub channel: (cbc::Sender<bool>, cbc::Receiver<bool>),
    pub reset_channel: (cbc::Sender<bool>, cbc::Receiver<bool>),
}
 */


/* impl Timer {
    pub fn init() -> Timer {
        println!("Timer initialized");
        Timer {
            start: Instant::now(),
            duration: Duration::from_secs(3),
            channel: cbc::unbounded::<bool>(),
            reset_channel: cbc::unbounded::<bool>(),
        }
    }

    pub fn start(&mut self, duration: Duration) {
        println!("Timer started");
        self.duration = duration;
        let mut timer = self.clone();
        spawn(move || {
            loop {
                if timer.start.elapsed() >= timer.duration {
                    timer.channel.0.send(true).unwrap();
                    break;
                }
                if let Ok(_) = timer.reset_channel.1.try_recv() {
                    timer.start = Instant::now();
                }
            }
        });
    }

    pub fn reset(&mut self) {
        self.reset_channel.0.send(true).unwrap();
    }
}



 */


