#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScheduleInterval {
    None,
    Seconds(i64),
    Minutes(i64),
    Hours(i64),
    Days(i64),
}