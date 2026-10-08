use std::{panic::AssertUnwindSafe, pin::Pin, sync::Arc};

use chrono::Utc;
use croner::Cron;
use futures::FutureExt;
use tokio::{signal::unix::{SignalKind, signal}, task::JoinSet, time::sleep};

type TriggerCallback = Arc<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub struct Scheduler {
    cron: String,
    once: bool,
    callback: TriggerCallback,
}

impl Scheduler {

    /// Create a new scheduler with a cron expression.
    /// 
    /// Example - run every 10 seconds: `*/10 * * * * *`
    /// 
    /// 1 - (optional) second (0 - 59)
    /// 
    /// 2 - minute (0 - 59)
    /// 
    /// 3 - hour (0 - 23)
    /// 
    /// 4 - day of month (1 - 31)
    /// 
    /// 5 - month (1 - 12, JAN-DEC)
    /// 
    /// 6 - day of week (0 - 6, SUN-Mon)
    /// 
    /// For more information, refer to https://crates.io/crates/croner
    pub fn new(cron: impl Into<String>) -> Self {
        Self {
            cron: cron.into(),
            once: false,
            callback: Arc::new(|| Box::pin(async {})),
        }
    }

    /// Sets the asynchronous job to execute when the scheduler triggers.
    pub fn job<T, Fut>(mut self, callback: T) -> Self
    where
        T: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.callback = Arc::new(move || Box::pin(callback()));
        self
    }

    /// Set the scheduler to only run once.
    pub fn once(mut self) -> Self {
        self.once = true;
        self
    }

    /// Run the scheduler.
    pub async fn run(self) {
        let mut receiver_join_set = JoinSet::new();
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");
        let cron: Cron = self.cron.parse().expect("Could not parse cron expression");

        receiver_join_set.spawn(async move {
            loop {
                let now = Utc::now();
                let next = cron.find_next_occurrence(&now, false).expect("Could not find next cron occurrence");
                tracing::trace!("Cron: {:?}", next);

                let duration = (next - now).to_std().expect("Cron occurrence is before now");
                sleep(duration).await;

                let callback_fut = (self.callback)();
                let result = AssertUnwindSafe(callback_fut).catch_unwind().await;
                if let Err(err) = result {
                    tracing::trace!("{:?}", err);
                }

                if self.once {
                    tracing::trace!("Once was configured, stopping...");
                    break;
                }
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

        tracing::trace!("Scheduler stopped");
    }
}