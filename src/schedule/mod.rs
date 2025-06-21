#[cfg(feature = "schedule")]
pub mod schedule_timezone;
#[cfg(feature = "schedule")]
pub mod schedule_interval;
#[cfg(feature = "schedule")]
pub mod schedule_receiver;

#[cfg(feature = "schedule")]
#[cfg(test)]
mod test {
    use crate::schedule::schedule_receiver::ScheduleReceiver;

    #[tokio::test]
    async fn http_receiver_sender() {
        let result = ScheduleReceiver::new().receive().await;
        assert!(result.is_ok());
    }
}