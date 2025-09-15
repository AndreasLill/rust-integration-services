#[derive(Clone)]
pub enum ScheduleReceiverEvent {
    OnTrigger(String),
}