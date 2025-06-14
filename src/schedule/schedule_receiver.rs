use std::{path::Path, pin::Pin, sync::Arc};
use chrono::{Duration as ChronoDuration, Local, NaiveDate, NaiveDateTime, NaiveTime};
use tokio::{fs::OpenOptions, io::{AsyncReadExt, AsyncWriteExt}, signal::unix::{signal, SignalKind}, task::JoinSet, time::sleep};
use uuid::Uuid;

use super::schedule_interval::ScheduleInterval;

type TriggerCallback = Arc<dyn Fn(String) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>;

pub struct ScheduleReceiver {
    interval: ScheduleInterval,
    next_run: NaiveDateTime,
    on_trigger: TriggerCallback,
    state_file: Option<String>,
}

impl ScheduleReceiver {
    pub fn new() -> Self {
        ScheduleReceiver {
            interval: ScheduleInterval::None,
            next_run: Local::now().naive_local(),
            on_trigger: Arc::new(|_| Box::pin(async {})),
            state_file: None,
        }
    }

    /// Save and load the state by file, if the file does not exist it will be created.
    pub fn state_file(mut self, file_path: &str) -> Self {
        self.state_file = Some(file_path.to_string());
        self
    }
    
    /// Sets the start date of the first run.
    /// 
    /// Will be ignored if state file already contains a previous saved state.
    pub fn start_date(mut self, year: i32, month: u32, day: u32) -> Self {
        let date = NaiveDate::from_ymd_opt(year, month, day).expect("Not a valid date.");
        let time = self.next_run.time();
        self.next_run = date.and_time(time);
        self
    }
    
    /// Sets the start time of the first run.
    /// 
    /// Will be ignored if state file already contains a previous saved state.
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

        let state_file = self.state_file.clone();
        if let Some(path) = state_file {
            match Self::load_state(&path, self.next_run).await {
                Ok(date_time) => self.next_run = date_time,
                Err(err) => panic!("Error loading state from file: {}", err.to_string()),
            }
        }

        let state_file = Arc::new(self.state_file);
        join_set.spawn(async move {
            loop {
                let now = Local::now().naive_local();
                
                match (self.next_run - now).to_std() {
                    Ok(duration_until_target) => sleep(duration_until_target).await,
                    Err(_) => {},
                }

                let uuid = Uuid::new_v4().to_string();
                (self.on_trigger)(uuid).await;

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

                if let Some(path) = state_file.as_ref() {
                    Self::save_state(&path, self.next_run).await.ok();
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

    async fn save_state(path: &str, datetime: NaiveDateTime) -> tokio::io::Result<()> {
        let path = Path::new(path);
        let mut file = OpenOptions::new().create(true).write(true).truncate(true).open(path).await?;
        let data = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        file.write_all(data.as_bytes()).await?;

        Ok(())
    }

    async fn load_state(path: &str, current: NaiveDateTime) -> tokio::io::Result<NaiveDateTime> {
        let path = Path::new(path);
        let mut file = OpenOptions::new().create(true).write(true).read(true).open(path).await?;
        let mut data = String::new();
        file.read_to_string(&mut data).await?;

        let date_time = match NaiveDateTime::parse_from_str(&data, "%Y-%m-%d %H:%M:%S") {
            Ok(data) => data,
            Err(_) => current,
        };
        Ok(date_time)
    }
}