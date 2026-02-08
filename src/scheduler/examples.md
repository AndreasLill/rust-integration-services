# ScheduleReceiver

Run a task once every hour and receive an event when it triggers.
``` rust
ScheduleReceiver::new()
.interval(ScheduleInterval::Hours(1))
.trigger(async move |uuid| {
    println!("Triggered: {}", uuid);
})
.receive()
.await;
```

Run a task once every day at 03:00 UTC and receive an event when it triggers.
``` rust
ScheduleReceiver::new()
.start_time(03, 00, 00)
.interval(ScheduleInterval::Days(1))
.trigger(async move |uuid| {
    println!("Triggered: {}", uuid);
})
.receive()
.await;
```