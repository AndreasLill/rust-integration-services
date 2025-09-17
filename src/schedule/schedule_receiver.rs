use std::sync::Arc;

use chrono::{DateTime, Duration as ChronoDuration, NaiveDate, NaiveTime, Utc};
use tokio::{signal::unix::{signal, SignalKind}, task::JoinSet, time::sleep};
use uuid::Uuid;

use crate::{common::event_handler::EventHandler, schedule::schedule_receiver_event::ScheduleReceiverEvent};

use super::schedule_interval::ScheduleInterval;

pub struct ScheduleReceiver {
    interval: ScheduleInterval,
    next_run: DateTime<Utc>,
    event_handler: EventHandler<ScheduleReceiverEvent>,
    event_join_set: JoinSet<()>,
}

impl ScheduleReceiver {
    pub fn new() -> Self {
        ScheduleReceiver {
            interval: ScheduleInterval::None,
            next_run: Utc::now(),
            event_handler: EventHandler::new(),
            event_join_set: JoinSet::new(),
        }
    }
    
    /// Sets the `UTC` start date for the scheduled task.
    /// 
    /// If the provided date is in the past, the scheduler will calculate the next valid future run based on the defined interval.
    /// 
    /// Note: Scheduler is using `UTC: Coordinated Universal Time` to avoid daylight saving problems.
    pub fn start_date(mut self, year: i32, month: u32, day: u32) -> Self {
        let naive_date = NaiveDate::from_ymd_opt(year, month, day).expect("Not a valid date.");
        let naive_time = self.next_run.time();
        let naive_dt = naive_date.and_time(naive_time);
        self.next_run = DateTime::from_naive_utc_and_offset(naive_dt, Utc);
        self
    }
    
    /// Sets the `UTC` start time for the scheduled task.
    /// 
    /// If the provided time is in the past, the scheduler will calculate the next valid future run based on the defined interval.
    /// 
    /// Note: Scheduler is using `UTC: Coordinated Universal Time` to avoid daylight saving problems.
    pub fn start_time(mut self, hour: u32, minute: u32, second: u32) -> Self {
        let naive_time = NaiveTime::from_hms_opt(hour, minute, second).expect("Not a valid time.");
        let naive_date = self.next_run.date_naive();
        let naive_dt = naive_date.and_time(naive_time);
        self.next_run = DateTime::from_naive_utc_and_offset(naive_dt, Utc);
        self
    }
    
    /// Sets the interval of how frequently the task should run.
    pub fn interval(mut self, interval: ScheduleInterval) -> Self {
        self.interval = interval;
        self
    }

    pub fn on_event<T, Fut>(mut self, handler: T) -> Self
    where
        T: Fn(ScheduleReceiverEvent) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.event_join_set = self.event_handler.init(handler);
        self
    }

    pub async fn receive(mut self) {
        let mut receiver_join_set = JoinSet::new();
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");
        let event_broadcast = Arc::new(self.event_handler.broadcast());

        let now_timestamp = Utc::now().timestamp();
        let next_run_timestamp = self.next_run.timestamp();
        if next_run_timestamp < now_timestamp {
            self.next_run = Self::calculate_next_run(self.next_run, self.interval).await;
        }

        receiver_join_set.spawn(async move {
            loop {
                let now = Utc::now();
                if let Ok(duration) = (self.next_run - now).to_std() {
                    sleep(duration).await;
                }

                let uuid = Uuid::new_v4().to_string();
                event_broadcast.send(ScheduleReceiverEvent::OnTrigger(uuid.to_string())).ok();
                if self.interval == ScheduleInterval::None {
                    break;
                }

                self.next_run = Self::calculate_next_run(self.next_run, self.interval).await;
            }
        });

        loop {
            tokio::select! {
                _ = sigterm.recv() => {
                    receiver_join_set.abort_all();
                    break;
                },
                _ = sigint.recv() => {
                    receiver_join_set.abort_all();
                    break;
                },
                task = receiver_join_set.join_next() => {
                    if task.is_none() {
                        break;
                    }
                }
            }
        }

        while let Some(_) = self.event_join_set.join_next().await {}
    }

    async fn calculate_next_run(next_run: DateTime<Utc>, interval: ScheduleInterval) -> DateTime<Utc> {
        let now = Utc::now();

        let interval_duration = match interval {
            ScheduleInterval::None => return next_run,
            ScheduleInterval::Seconds(seconds) => ChronoDuration::seconds(seconds),
            ScheduleInterval::Minutes(minutes) => ChronoDuration::minutes(minutes),
            ScheduleInterval::Hours(hours) => ChronoDuration::hours(hours),
            ScheduleInterval::Days(days) => ChronoDuration::days(days),
        };

        let mut calculated_next_run = next_run.clone();
        while calculated_next_run < now {
            calculated_next_run += interval_duration;
        }

        calculated_next_run
    }
}