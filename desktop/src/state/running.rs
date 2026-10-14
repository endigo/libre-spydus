//! A job others can wait for (iOS keeps the `Task` and awaits `.value`): the job holds a `Done`
//! and drops it when it ends (or is dropped), and every clone of its `Running` resolves then.

use std::future::Future;

use futures::FutureExt;
use futures::channel::oneshot;
use futures::future::{BoxFuture, Shared};

#[derive(Clone)]
pub struct Running(Shared<BoxFuture<'static, ()>>);

/// Held by the job; dropping it marks the job done.
pub struct Done(#[allow(dead_code)] oneshot::Sender<()>);

impl Running {
    pub fn new() -> (Done, Running) {
        let (sender, receiver) = oneshot::channel::<()>();
        (Done(sender), Running(receiver.map(|_| ()).boxed().shared()))
    }

    /// Resolves when the job has ended.
    pub fn wait(&self) -> impl Future<Output = ()> + Send + use<> {
        self.0.clone()
    }
}
