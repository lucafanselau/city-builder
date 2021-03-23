use async_channel::Sender;
use futures_lite::{future, stream, Future};
use std::{mem, sync::Arc, thread::JoinHandle};

use crate::task::Task;

#[derive(Debug)]
pub struct TaskPoolInner {
    handles: Vec<JoinHandle<()>>,
    shutdown: Sender<()>,
    name: String,
}

#[derive(Clone, Debug)]
pub struct TaskPool {
    inner: Arc<TaskPoolInner>,
    executor: Arc<async_executor::Executor<'static>>,
}

impl TaskPool {
    pub fn new(num_threads: Option<usize>, name: &str) -> Self {
        let executor = Arc::new(async_executor::Executor::new());
        let num_threads = num_threads.unwrap_or_else(num_cpus::get);
        let (shutdown_sender, shutdown_receiver) = async_channel::bounded(1);
        let handles = (0..num_threads)
            .into_iter()
            .map(|i| {
                let executor = executor.clone();
                let shutdown_receiver = shutdown_receiver.clone();

                let thread_name = format!("{} [{}]", name, i);

                let builder = std::thread::Builder::new().name(thread_name.clone());

                builder
                    .spawn(move || {
                        let executor_future = executor.run(shutdown_receiver.recv());
                        log::trace!("Started thread {}", thread_name);
                        future::block_on(executor_future).unwrap_err();
                    })
                    .unwrap()
            })
            .collect();

        Self {
            inner: Arc::new(TaskPoolInner {
                handles,
                shutdown: shutdown_sender,
                name: name.into(),
            }),
            executor,
        }
    }

    pub fn spawn<T: Send + 'static>(
        &self,
        future: impl Future<Output = T> + Send + 'static,
    ) -> Task<T> {
        Task::new(self.executor.spawn(future))
    }

    pub fn scope<'scope, T, Function>(&self, cb: Function) -> Vec<T>
    where
        T: Send + 'scope,
        Function: FnOnce(&mut Scope<'scope, T>) + Send + 'scope,
    {
        use futures_lite::StreamExt;

        let executor: &async_executor::Executor<'static> = &*self.executor;
        let executor: &'scope async_executor::Executor<'scope> =
            unsafe { mem::transmute(executor) };
        let mut scope: Scope<T> = Scope {
            executor,
            spawned: Vec::new(),
        };
        cb(&mut scope);
        match scope.spawned.len() {
            0 => Vec::default(),
            1 => vec![future::block_on(&mut scope.spawned[0])],
            _ => {
                let future = async move {
                    let stream = stream::iter(scope.spawned.into_iter());
                    stream
                        .then(|t| async move { t.await })
                        .collect::<Vec<_>>()
                        .await
                };

                let mut collect_task = executor.spawn(future);
                loop {
                    if let Some(results) = future::block_on(future::poll_once(&mut collect_task)) {
                        break results;
                    }
                    // otherwise support executor threads
                    executor.try_tick();
                }
            }
        }
    }
}

impl Drop for TaskPoolInner {
    fn drop(&mut self) {
        self.shutdown.close();
        let name = self.name.clone();
        for t in self.handles.drain(..) {
            t.join()
                .unwrap_or_else(|_| panic!("[TaskPool] {} failed to join handle", name));
        }
    }
}

pub struct Scope<'scope, T> {
    executor: &'scope async_executor::Executor<'scope>,
    spawned: Vec<async_executor::Task<T>>,
}

impl<'scope, T: Send + 'static> Scope<'scope, T> {
    pub fn spawn(&mut self, future: impl Future<Output = T> + Send + 'static) {
        let task = self.executor.spawn(future);
        self.spawned.push(task);
    }
}

#[cfg(test)]
mod tests {
    use futures_lite::StreamExt;
    use std::time::Duration;

    use super::*;

    #[test]
    #[ignore]
    fn simple_task_pool() {
        let pool = TaskPool::new(None, "simple_task_pool");
        let tasks: Vec<Task<()>> = (0..8)
            .into_iter()
            .map(|i| {
                pool.spawn(async move {
                    let mut interval =
                        async_std::stream::interval(Duration::from_secs_f32(i as f32 * 0.5 + 0.5));
                    while interval.next().await.is_some() {
                        println!("Got Interval on task {}", i);
                    }
                })
            })
            .collect();

        let complete = async move {
            for t in tasks.into_iter() {
                t.await
            }
        };
        future::block_on(complete);
    }

    #[test]
    fn scoping() {
        fn fib(i: usize) -> usize {
            match i {
                0 | 1 => 1,
                _ => fib(i - 1) + fib(i - 2),
            }
        }
        {
            let pool = TaskPool::new(None, "scoping_pool");
            let fib = pool.scope(|scope| {
                (0..20)
                    .into_iter()
                    .for_each(|i| scope.spawn(async move { fib(i) }));
            });
            println!("{:#?}", fib);
        }
    }
}
