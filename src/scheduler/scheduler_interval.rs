#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SchedulerInterval {
    None,
    Seconds(i64),
    Minutes(i64),
    Hours(i64),
    Days(i64),
}