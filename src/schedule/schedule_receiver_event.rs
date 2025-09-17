#[derive(Clone)]
pub enum ScheduleReceiverEvent {
    Trigger {
        uuid: String
    },
    Error {
        uuid: String,
        error: String
    },
}