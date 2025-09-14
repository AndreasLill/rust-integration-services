# ScheduleReceiver

Run a task once every hour and receive an event when it triggers.
``` rust
ScheduleReceiver::new()
.interval(ScheduleInterval::Hours(1))
.on_event(async move |event| {
    match event {
        ScheduleReceiverEventSignal::OnTrigger(uuid) => println!("{}", uuid),
    }
})
.receive()
.await;
```

Run a task once every day at 03:00 UTC and receive an event when it triggers.
``` rust
ScheduleReceiver::new()
.start_time(03, 00, 00)
.interval(ScheduleInterval::Days(1))
.on_event(async move |event| {
    match event {
        ScheduleReceiverEventSignal::OnTrigger(uuid) => println!("{}", uuid),
    }
})
.receive()
.await;
```