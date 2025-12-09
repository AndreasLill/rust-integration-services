use std::{panic::AssertUnwindSafe, pin::Pin, sync::Arc};

use futures::FutureExt;
use time::{Date, OffsetDateTime, Time, Duration};
use tokio::{signal::unix::{signal, SignalKind}, task::JoinSet, time::sleep};
use uuid::Uuid;

use super::schedule_interval::ScheduleInterval;

type TriggerCallback = Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub struct ScheduleReceiver {
    interval: ScheduleInterval,
    next_run: OffsetDateTime,
    callback_trigger: TriggerCallback,
}

impl ScheduleReceiver {
    pub fn new() -> Self {
        ScheduleReceiver {
            interval: ScheduleInterval::None,
            next_run: OffsetDateTime::now_utc(),
            callback_trigger: Arc::new(|_| Box::pin(async {})),
        }
    }
    
    /// Sets the `UTC` start date for the scheduled task.
    /// 
    /// If the provided date is in the past, the scheduler will calculate the next valid future run based on the defined interval.
    /// 
    /// Note: Scheduler is using `UTC: Coordinated Universal Time` to avoid daylight saving problems.
    pub fn start_date(mut self, year: i32, month: u8, day: u8) -> Self {
        let date = Date::from_calendar_date(year, month.try_into().unwrap(), day).expect("Not a valid date.");
        let time = self.next_run.time();
        self.next_run = date.with_time(time).assume_utc();
        self
    }
    
    /// Sets the `UTC` start time for the scheduled task.
    /// 
    /// If the provided time is in the past, the scheduler will calculate the next valid future run based on the defined interval.
    /// 
    /// Note: Scheduler is using `UTC: Coordinated Universal Time` to avoid daylight saving problems.
    pub fn start_time(mut self, hour: u8, minute: u8, second: u8) -> Self {
        let time = Time::from_hms(hour, minute, second).expect("Not a valid time.");
        let date = self.next_run.date();
        self.next_run = date.with_time(time).assume_utc();
        self
    }
    
    /// Sets the interval of how frequently the task should run.
    pub fn interval(mut self, interval: ScheduleInterval) -> Self {
        self.interval = interval;
        self
    }

    pub fn trigger<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn(String) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.callback_trigger = Arc::new(move |uuid| Box::pin(callback(uuid)));
        self
    }

    pub async fn receive(mut self) {
        let mut receiver_join_set = JoinSet::new();
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");

        if self.next_run < OffsetDateTime::now_utc() {
            self.next_run = Self::calculate_next_run(self.next_run, self.interval).await;
        }

        tracing::trace!("started, next run at {:?}", self.next_run);
        receiver_join_set.spawn(async move {
            loop {
                let now = OffsetDateTime::now_utc();
                if self.next_run > now {
                    let duration = self.next_run - now;
                    tracing::trace!("sleep: {:?}", duration);
                    sleep(Self::to_std_duration(duration)).await;
                }
                
                if self.interval != ScheduleInterval::None {
                    self.next_run = Self::calculate_next_run(self.next_run, self.interval).await;
                }

                let uuid = Uuid::new_v4().to_string();
                tracing::trace!("[{}] trigger started", uuid);
                let callback_fut = (self.callback_trigger)(uuid.clone());
                let result = AssertUnwindSafe(callback_fut).catch_unwind().await;
                if let Err(err) = result {
                    tracing::trace!("[{}] {:?}", uuid, err);
                }
                tracing::trace!("[{}] trigger completed", uuid);
                
                if self.interval == ScheduleInterval::None {
                    break;
                }

                tracing::trace!("next run at {:?}", self.next_run);
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

        tracing::trace!("shut down complete");
    }

    async fn calculate_next_run(next_run: OffsetDateTime, interval: ScheduleInterval) -> OffsetDateTime {
        let now = OffsetDateTime::now_utc();

        let interval_duration = match interval {
            ScheduleInterval::None => return next_run,
            ScheduleInterval::Seconds(seconds) => Duration::seconds(seconds),
            ScheduleInterval::Minutes(minutes) => Duration::minutes(minutes),
            ScheduleInterval::Hours(hours) => Duration::hours(hours),
            ScheduleInterval::Days(days) => Duration::days(days),
        };

        let mut calculated_next_run = next_run.clone();
        while calculated_next_run < now {
            calculated_next_run += interval_duration;
        }

        calculated_next_run
    }

    fn to_std_duration(duration: Duration) -> std::time::Duration {
        if duration.is_negative() {
            std::time::Duration::from_millis(0)
        } else {
            std::time::Duration::new(duration.whole_seconds() as u64, duration.subsec_nanoseconds().try_into().unwrap())
        }
    }
}