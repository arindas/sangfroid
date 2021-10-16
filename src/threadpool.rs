//! A load balanced threadpool.

use std::{
    convert::TryInto,
    error::Error,
    fmt::{Debug, Display},
    ops::DerefMut,
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use crate::{job::Job, message::Message, worker::Worker};
use bheap::BinaryMaxHeap;

#[derive(Debug)]
pub enum ThreadPoolError {
    LockError,
    LookupError,
    WorkerUnavailable,
    JobSchedulingFailed,
    TermNoticeFailed,
    JoinFailed,
}

impl Display for ThreadPoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            ThreadPoolError::LockError => write!(f, "unable to acquire lock on thread pool"),
            ThreadPoolError::LookupError => write!(f, "unable to look up worker"),
            ThreadPoolError::WorkerUnavailable => write!(f, "worker unavailable"),
            ThreadPoolError::JobSchedulingFailed => write!(f, "unable to schedule job"),
            ThreadPoolError::TermNoticeFailed => {
                write!(f, "failed to send term notice to balancer thread")
            }
            ThreadPoolError::JoinFailed => write!(f, "join() on balancer thread failed"),
        }
    }
}

impl Error for ThreadPoolError {}

fn restore_worker_pool_order<Req, Res>(
    worker_pool: &mut BinaryMaxHeap<Worker<Req, Res>>,
    worker_uid: u64,
) -> Result<(), ThreadPoolError>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    let mut pool_restored = false;

    if let Some(i) = worker_pool.index_in_heap_from_uid(worker_uid) {
        if let Some(worker) = worker_pool.get(i) {
            worker.dec_load();
            pool_restored = true;
        }
        worker_pool.restore_heap_property(i);
    }

    return if pool_restored {
        Ok(())
    } else {
        Err(ThreadPoolError::LookupError)
    };
}

fn worker_pool_schedule_job<Req, Res>(
    worker_pool: &mut BinaryMaxHeap<Worker<Req, Res>>,
    job: Job<Req, Res>,
) -> Result<(), ThreadPoolError>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    if worker_pool.is_empty() {
        return Err(ThreadPoolError::WorkerUnavailable);
    }

    if let Some(worker) = worker_pool.get(0) {
        worker
            .dispatch(job)
            .or(Err(ThreadPoolError::JobSchedulingFailed))?;
        worker.inc_load();
    }

    worker_pool.restore_heap_property(0);

    Ok(())
}

pub struct ThreadPool<Req, Res>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    pool: Option<Arc<Mutex<BinaryMaxHeap<Worker<Req, Res>>>>>,

    done_channel: Sender<Option<u64>>,

    balancer: Option<JoinHandle<Result<(), ThreadPoolError>>>,
}

impl<Req, Res> ThreadPool<Req, Res>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    pub fn new(workers: usize) -> Self {
        let (worker_vec, (done_tx, done_rx)) = Self::new_workers(workers);
        let worker_pool = Arc::new(Mutex::new(BinaryMaxHeap::from_vec(worker_vec)));

        let balancer = Self::balancer_thread(done_rx, Arc::clone(&worker_pool));

        ThreadPool {
            pool: Some(worker_pool),
            done_channel: done_tx,
            balancer: Some(balancer),
        }
    }

    pub fn balancer_thread(
        done_channel: Receiver<Option<u64>>,
        worker_heap: Arc<Mutex<BinaryMaxHeap<Worker<Req, Res>>>>,
    ) -> JoinHandle<Result<(), ThreadPoolError>> {
        thread::spawn(move || -> Result<(), ThreadPoolError> {
            while let Ok(Some(uid)) = done_channel.recv() {
                restore_worker_pool_order(
                    worker_heap
                        .lock()
                        .or(Err(ThreadPoolError::LockError))?
                        .deref_mut(),
                    uid,
                )?;
            }

            Ok(())
        })
    }

    pub fn new_workers(
        workers: usize,
    ) -> (
        Vec<Worker<Req, Res>>,
        (Sender<Option<u64>>, Receiver<Option<u64>>),
    ) {
        let (done_tx, done_rx) = channel::<Option<u64>>();
        let mut worker_vec = Vec::<Worker<Req, Res>>::with_capacity(workers);

        for i in 0..workers {
            let (wtx, wrx) = channel::<Message<Req, Res>>();
            worker_vec.push(Worker::new(
                wrx,
                wtx,
                done_tx.clone(),
                i.try_into().unwrap(),
            ));
        }

        (worker_vec, (done_tx, done_rx))
    }

    pub fn schedule(&self, job: Job<Req, Res>) -> Result<(), ThreadPoolError> {
        if let Some(worker_pool) = &self.pool {
            worker_pool_schedule_job(
                worker_pool
                    .lock()
                    .or(Err(ThreadPoolError::LockError))?
                    .deref_mut(),
                job,
            )?;
        }

        Ok(())
    }

    pub fn terminate(&mut self) -> Result<(), ThreadPoolError> {
        // Ensure that all threads complete their jobs and
        // complete pending done notifications if any.
        // This is necessary since the receive end of the
        // done channel is to be dropped.
        drop(self.pool.take());

        if self.balancer.is_none() {
            return Ok(());
        }

        self.done_channel
            .send(None)
            .or(Err(ThreadPoolError::TermNoticeFailed))?;

        return match self.balancer.take().unwrap().join() {
            Ok(result) => result,
            Err(_) => Err(ThreadPoolError::JoinFailed),
        };
    }
}

impl<Req, Res> Drop for ThreadPool<Req, Res>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    /// Invokes terminate()
    fn drop(&mut self) {
        self.terminate().unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::ThreadPool;

    #[test]
    fn thread_pool_new_and_terminate() {
        ThreadPool::<(), ()>::new(0);

        ThreadPool::<(), ()>::new(10);
    }
}
