use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::{Arc};
use std::thread;
use async_std::task;

use std::time::Duration;
use async_trait::async_trait;
use chrono::{DateTime, Local};
use rand::Rng;
use tokio::sync::RwLock;
use phi_accrual_detector::{Detector, PhiInteraction};

#[derive(Debug)]
struct HistoryElement {
    phi: f64,
    time: DateTime<Local>,
}

struct Monitor {
    detector: Arc<Detector>,
    history: RwLock<Vec<HistoryElement>>,
}

#[async_trait]
trait MonitorInteraction {
    async fn ping(&self);
    async fn suspicion(&self) -> f64;
    async fn show_history(&self);
    async fn publish_csv(&self, filename: &str);
}

impl Monitor {
    pub fn new(detector: Arc<Detector>) -> Self {
        Monitor { detector, history: Default::default() }
    }
}

#[async_trait]
impl MonitorInteraction for Monitor {
    async fn ping(&self) {
        let current_time = Local::now();
        self.detector.insert(current_time).await.expect("Some panic occurred");
    }

    async fn suspicion(&self) -> f64 {
        let current_time = Local::now();
        let last_arrived_at = self.detector.last_arrived_at().await.expect("Some panic occurred");
        let phi = self.detector.phi(current_time).await.unwrap();
        let mut history = self.history.write().await;
        println!("suspicion: {:?} when last ping was at : {:?}", phi, last_arrived_at);
        history.push(HistoryElement { phi, time: current_time });
        phi
    }

    async fn show_history(&self) {
        let history = self.history.read().await;
        println!("Suspicion History: {:?}", history);
    }

    async fn publish_csv(&self, file_path: &str) {
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .open(file_path)
            .unwrap();
        let history = self.history.read().await;

        for element in history.iter() {
            let line = format!("{},{}\n", element.phi, element.time.format("%M:%S:%.6f"));
            file.write_all(line.as_bytes()).unwrap();
        }
        println!("metrics published");
    }
}

#[tokio::main]
async fn main() {
    let detector = Arc::new(Detector::new(1000));
    let monitor = Arc::new(Monitor::new(detector.clone()));
    let monitor_phi = Arc::clone(&monitor);

    let ping_thread = thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                loop {
                    let dur = rand::thread_rng().gen_range(100..1000);
                    if dur > 900 {
                        println!("Simulating shutdown at: {:?}", Local::now().to_rfc3339());
                        break;
                    }
                    // Simulate the "ping" process
                    task::sleep(Duration::from_millis(dur)).await;
                    println!("Pinging the monitor");
                    monitor.ping().await;
                }
            });
    });
    let phi_thread = thread::spawn(move || {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(async {
                let mut i = 0;
                loop {
                    // Simulate the "ping" process
                    task::sleep(Duration::from_millis(200)).await;
                    monitor_phi.suspicion().await;
                    if i % 10 == 0 {
                        // monitor_phi.show_history().await;
                        monitor_phi.publish_csv("history.csv").await;
                    }
                    i += 1;
                }
            });
    });

    ping_thread.join().unwrap();
    phi_thread.join().unwrap();
}