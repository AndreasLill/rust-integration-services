use tokio::{signal::unix::{signal, SignalKind}, sync::mpsc, task::JoinSet};

pub struct EventHandler<Event> {
    broadcast: mpsc::Sender<Event>,
    receiver: Option<mpsc::Receiver<Event>>,
}

impl<Event: Send + 'static> EventHandler<Event> {
    pub fn new() -> Self {
        let (event_broadcast, event_receiver) = mpsc::channel(128);
        EventHandler {
            broadcast: event_broadcast,
            receiver: Some(event_receiver),
        }
    }

    pub fn broadcast(&self) -> mpsc::Sender<Event> {
        self.broadcast.clone()
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
    pub fn init<T, Fut>(&mut self, handler: T) -> JoinSet<()>
    where
        T: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let mut receiver = self.receiver.take().expect("Event receiver already taken");
        let mut sigterm = signal(SignalKind::terminate()).expect("Failed to start SIGTERM signal receiver");
        let mut sigint = signal(SignalKind::interrupt()).expect("Failed to start SIGINT signal receiver");

        let mut join_set = JoinSet::new();
        join_set.spawn(async move {
            loop {
                tokio::select! {
                    _ = sigterm.recv() => break,
                    _ = sigint.recv() => break,
                    event = receiver.recv() => {
                        match event {
                            Some(event) => handler(event).await,
                            None => break,
                        }
                    }
                }
            }
        });

        join_set
    }
}