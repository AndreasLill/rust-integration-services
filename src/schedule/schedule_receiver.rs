use std::{pin::Pin, sync::Arc};
use chrono::{DateTime, Duration as ChronoDuration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use tokio::{signal::unix::{signal, SignalKind}, task::JoinSet, time::sleep};
use uuid::Uuid;

use super::schedule_interval::ScheduleInterval;

type TriggerCallback = Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

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
            on_trigger: Arc::new(|_| Box::pin(async {})),
        }
    }
    
    /// Sets the start date for the scheduled task.
    /// 
    /// If the provided date is in the past, the scheduler will calculate the next valid future run based on the defined interval.
    pub fn start_date(mut self, year: i32, month: u32, day: u32) -> Self {
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("Not a valid date.");
        let time = self.next_run.time();
        self.next_run = date.and_time(time);
        self
    }
    
    /// Sets the start time for the scheduled task.
    /// 
    /// If the provided time is in the past, the scheduler will calculate the next valid future run based on the defined interval.
    pub fn start_time(mut self, hour: u32, minute: u32, second: u32) -> Self {
        let date = self.next_run.date();
        let time = NaiveTime::from_hms_opt(hour, minute, second).expect("Not a valid time.");
        self.next_run = date.and_time(time);
        self
    }
    
    /// Sets the interval of how frequently the task should run.
    pub fn interval(mut self, interval: ScheduleInterval) -> Self {
        self.interval = interval;
        self
    }

    /// Asyncronous scheduled task callback.
    pub fn on_trigger<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.on_trigger = Arc::new(move |uuid| Box::pin(callback(uuid)));
        self
    }

    pub async fn run(mut self) {
        let mut join_set = JoinSet::new();
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver.");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver.");

        let now_comp = DateTime::from_timestamp(Local::now().naive_local().and_utc().timestamp(), 0).unwrap();
        let next_run_comp = DateTime::from_timestamp(self.next_run.and_utc().timestamp(), 0).unwrap();
        if next_run_comp < now_comp {
            self.next_run = Self::calculate_next_run(self.next_run, self.interval).await;
            println!("Next run: {}", self.next_run);
        }

        join_set.spawn(async move {
            loop {
                let now = Local::now().naive_local();
                if let Ok(duration) = (self.next_run - now).to_std() {
                    sleep(duration).await;
                }

                let uuid = Uuid::new_v4().to_string();
                (self.on_trigger)(uuid).await;

                if self.interval == ScheduleInterval::None {
                    break;
                }

                self.next_run = Self::calculate_next_run(self.next_run, self.interval).await;
                println!("Next run: {}", self.next_run);
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

    async fn calculate_next_run(next_run: NaiveDateTime, interval: ScheduleInterval) -> NaiveDateTime {
        let now = Local::now().naive_local();

        let interval_duration = match interval {
            ScheduleInterval::None => return next_run,
            ScheduleInterval::Seconds(seconds) => ChronoDuration::seconds(seconds),
            ScheduleInterval::Minutes(minutes) => ChronoDuration::minutes(minutes),
            ScheduleInterval::Hours(hours) => ChronoDuration::hours(hours),
            ScheduleInterval::Days(days) => ChronoDuration::days(days),
        };

        let mut calculated_next_run = next_run.clone();
        if interval_duration != ChronoDuration::zero() {
            while calculated_next_run < now {
                calculated_next_run += interval_duration;
            }
        }

        calculated_next_run
    }
}