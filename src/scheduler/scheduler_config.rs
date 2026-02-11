use time::{Date, OffsetDateTime, Time};

use crate::scheduler::scheduler_interval::SchedulerInterval;

pub struct SchedulerConfig {
    pub interval: SchedulerInterval,
    pub start_date: Date,
    pub start_time: Time,
}

impl SchedulerConfig {
    pub fn default() -> Self {
        SchedulerConfig {
            interval: SchedulerInterval::None,
            start_date: OffsetDateTime::now_utc().date(),
            start_time: OffsetDateTime::now_utc().time(),
        }
    }

    /// Sets the interval of how frequently the task should run.
    pub fn interval(mut self, interval: SchedulerInterval) -> Self {
        self.interval = interval;
        self
    }

    /// Sets the `UTC` start date for the scheduled task.
    /// 
    /// If the provided date is in the past, the scheduler will calculate the next valid future run based on the defined interval.
    /// 
    /// Note: Scheduler is using `UTC: Coordinated Universal Time` to avoid daylight saving problems.
    pub fn start_date(mut self, year: i32, month: u8, day: u8) -> Self {
        //let time = self.start_datetime.time();
        //self.start_date = date.with_time(time).assume_utc();
        self.start_date = Date::from_calendar_date(year, month.try_into().unwrap(), day).expect("Not a valid date.");
        self
    }
    
    /// Sets the `UTC` start time for the scheduled task.
    /// 
    /// If the provided time is in the past, the scheduler will calculate the next valid future run based on the defined interval.
    /// 
    /// Note: Scheduler is using `UTC: Coordinated Universal Time` to avoid daylight saving problems.
    pub fn start_time(mut self, hour: u8, minute: u8, second: u8) -> Self {
        //let date = self.start_datetime.date();
        self.start_time = Time::from_hms(hour, minute, second).expect("Not a valid time.");
        self
    }
}