use std::sync::{Arc, atomic::{AtomicBool, Ordering}};

use crate::scheduler::scheduler::Scheduler;

#[tokio::test]
async fn scheduler_runs_once() {

    let called = Arc::new(AtomicBool::new(false));
    let called_clone = called.clone();

    Scheduler::new("*/1 * * * * *")
        .once()
        .job(move || {
            let called = called_clone.clone();

            async move {
                called.store(true, Ordering::SeqCst);
            }
        })
        .run()
        .await;

    assert!(called.load(Ordering::SeqCst));
}