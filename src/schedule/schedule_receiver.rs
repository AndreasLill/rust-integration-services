use std::{pin::Pin, sync::Arc};
use chrono::{Duration as ChronoDuration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use tokio::{signal::unix::{signal, SignalKind}, task::JoinSet, time::sleep};

use super::schedule_interval::ScheduleInterval;

type TriggerCallback = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub struct ScheduleReceiver {
    interval: ScheduleInterval,
    next_run: NaiveDateTime,
    on_trigger: TriggerCallback,
}

impl ScheduleReceiver {
    pub fn new() -> Self {
        ScheduleReceiver {
            interval: ScheduleInterval::None,
            next_run: Local::now().naive_local(),
            on_trigger: Arc::new(|| Box::pin(async {})),
        }
    }
    
    pub fn start_date(mut self, year: i32, month: u32, day: u32) -> Self {
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("Not a valid date.");
        let time = self.next_run.time();
        self.next_run = date.and_time(time);
        self
    }
    
    pub fn start_time(mut self, hour: u32, minute: u32, second: u32) -> Self {
        let date = self.next_run.date();
        let time = NaiveTime::from_hms_opt(hour, minute, second).expect("Not a valid time.");
        self.next_run = date.and_time(time);
        self
    }
    
    pub fn interval(mut self, interval: ScheduleInterval) -> Self {
        self.interval = interval;
        self
    }

    pub fn on_trigger<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_trigger = Arc::new(move || Box::pin(callback()));
        self
    }

    pub async fn run(mut self) {
        let mut join_set = JoinSet::new();
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver.");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver.");

        join_set.spawn(async move {
            loop {
                let now = Local::now().naive_local();
                
                match (self.next_run - now).to_std() {
                    Ok(duration_until_target) => sleep(duration_until_target).await,
                    Err(_) => {},
                }

                (self.on_trigger)().await;

                match self.interval {
                    ScheduleInterval::None => break,
                    ScheduleInterval::Seconds(seconds) => {
                        self.next_run += ChronoDuration::seconds(seconds);
                    },
                    ScheduleInterval::Minutes(minutes) => {
                        self.next_run += ChronoDuration::minutes(minutes);
                    },
                    ScheduleInterval::Hours(hours) => {
                        self.next_run += ChronoDuration::hours(hours);
                    },
                    ScheduleInterval::Days(days) => {
                        self.next_run += ChronoDuration::days(days);
                    },
                }
            }
        });

        loop {
            tokio::select! {
                _ = sigterm.recv() => {
                    join_set.abort_all();
                    break;
                },
                _ = sigint.recv() => {
                    join_set.abort_all();
                    break;
                },
                task = join_set.join_next() => {
                    match task {
                        Some(_) => {},
                        None => break,
                    }
                }
            }
        }
    }
}