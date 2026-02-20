#[derive(Clone)]
pub struct Executor;

impl<F> hyper::rt::Executor<F> for Executor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}