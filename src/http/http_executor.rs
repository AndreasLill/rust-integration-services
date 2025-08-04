#[derive(Clone)]
pub struct HttpExecutor;

impl<F> hyper::rt::Executor<F> for HttpExecutor
where
    F: std::future::Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        tokio::task::spawn(fut);
    }
}