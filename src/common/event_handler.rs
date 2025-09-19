use std::sync::Arc;

use tokio::{sync::{mpsc, Notify}, task::JoinSet};

pub struct EventHandler<Event> {
    broadcast: mpsc::UnboundedSender<Event>,
    receiver: Option<mpsc::UnboundedReceiver<Event>>,
    join_set: JoinSet<()>,
    shutdown: Arc<Notify>,
}

impl<Event: Send + 'static> EventHandler<Event> {
    pub fn new() -> Self {
        let (event_broadcast, event_receiver) = mpsc::unbounded_channel();
        EventHandler {
            broadcast: event_broadcast,
            receiver: Some(event_receiver),
            join_set: JoinSet::new(),
            shutdown: Arc::new(Notify::new())
        }
    }

    pub fn broadcast(&self) -> mpsc::UnboundedSender<Event> {
        self.broadcast.clone()
    }

    /// Gracefully shutdown the event handler and wait for all remaining tasks to finish.
    pub async fn shutdown(&mut self) {
        self.shutdown.notify_waiters();
        while let Some(_) = self.join_set.join_next().await {}
    }

    /// Initializes the event handler loop and spawns it in a new `JoinSet`.
    ///
    /// This method takes ownership of the event receiver and continuously listens
    /// for incoming events or termination signals (`SIGTERM` / `SIGINT`). Each
    /// received event is passed to the provided `handler` future. The loop exits
    /// when a termination signal is received or the channel is closed.
    ///
    /// Returns a `JoinSet<()>` containing the spawned task, which can be awaited
    /// to track completion or perform graceful shutdown.
    pub fn init<T, Fut>(&mut self, handler: T)
    where
        T: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut receiver = self.receiver.take().expect("Event receiver already taken");
        let shutdown = self.shutdown.clone();

        self.join_set.spawn(async move {
            loop {
                tokio::select! {
                    _ = shutdown.notified() => {
                        break;
                    },
                    event = receiver.recv() => {
                        match event {
                            Some(event) => handler(event).await,
                            None => break,
                        }
                    }
                }
            }
        });
    }
}