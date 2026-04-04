use std::{panic::AssertUnwindSafe, pin::Pin, sync::Arc};

use futures::FutureExt;
use time::{OffsetDateTime, Duration};
use tokio::{signal::unix::{signal, SignalKind}, task::JoinSet, time::sleep};

use crate::scheduler::scheduler_config::SchedulerConfig;

type TriggerCallback = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub struct Scheduler {
    config: SchedulerConfig,
    next_run: OffsetDateTime,
    callback: TriggerCallback,
}

impl Scheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        let start_date = config.start_date;
        let start_time = config.start_time;
        Scheduler {
            config,
            next_run: start_date.with_time(start_time).assume_utc(),
            callback: Arc::new(|| Box::pin(async {})),
        }
    }

    pub fn trigger<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.callback = Arc::new(move || Box::pin(callback()));
        self
    }

    pub async fn run(mut self) {
        let mut receiver_join_set = JoinSet::new();
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");

        if self.next_run < OffsetDateTime::now_utc() {
            self.next_run = Self::calculate_next_run(self.next_run, self.config.interval).await;
        }

        tracing::trace!("Scheduler next run at {:?}", self.next_run);
        receiver_join_set.spawn(async move {
            loop {
                let now = OffsetDateTime::now_utc();
                if self.next_run > now {
                    let duration = self.next_run - now;
                    tracing::trace!("sleep: {:?}", duration);
                    sleep(Self::to_std_duration(duration)).await;
                }
                
                if self.config.interval != None {
                    self.next_run = Self::calculate_next_run(self.next_run, self.config.interval).await;
                }

                let callback_fut = (self.callback)();
                let result = AssertUnwindSafe(callback_fut).catch_unwind().await;
                if let Err(err) = result {
                    tracing::trace!("{:?}", err);
                }
                
                if self.config.interval == None {
                    break;
                }

                tracing::trace!("Scheduler next run at {:?}", self.next_run);
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

    async fn calculate_next_run(next_run: OffsetDateTime, interval: Option<Duration>) -> OffsetDateTime {
        
        if let Some(duration) = interval {
            let now = OffsetDateTime::now_utc();
            let mut calculated_next_run = next_run;
            while calculated_next_run < now {
                calculated_next_run += duration;
            }
            return calculated_next_run;
        }

        next_run
    }

    fn to_std_duration(duration: Duration) -> std::time::Duration {
        if duration.is_negative() {
            std::time::Duration::from_millis(0)
        } else {
            std::time::Duration::new(duration.whole_seconds() as u64, duration.subsec_nanoseconds().try_into().unwrap())
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self {
        Scheduler::new(SchedulerConfig::new())
    }
}