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
    WorkerTermFailed,
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
            ThreadPoolError::WorkerTermFailed => write!(f, "failed to terminate worker"),
        }
    }
}

impl Error for ThreadPoolError {}

/// Restores the order of the workers in the worker pool after any modifications to the
/// number of pending tasks they have.
fn restore_worker_pool_order<Req, Res>(
    worker_pool: &mut BinaryMaxHeap<Worker<Req, Res>>,
    worker_uid: u64,
) -> Result<(), ThreadPoolError>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    if worker_pool.is_empty() {
        return Ok(());
    }

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

/// Schedules a new job to the given worker pool by picking up the least
/// loaded worker and dispatching th job to it.
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

/// Terminates all workers in the given pool of workers by popping them
/// out and invoking `Worker::terminate()` on each of them.
fn worker_pool_terminate<Req, Res>(
    worker_pool: &mut BinaryMaxHeap<Worker<Req, Res>>,
) -> Result<(), ThreadPoolError>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    while let Some(mut worker) = worker_pool.pop() {
        worker
            .terminate()
            .or(Err(ThreadPoolError::WorkerTermFailed))?;
    }

    Ok(())
}

/// ThreadPool to keep track of worker threads, dynamically dispatch
/// jobs in a load balanced manner, and distribute load evenly.
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
    /// Creates a new threadpool with the given number of workers and the optionally provided Request and Result types.
    /// ```
    /// use sangfroid::threadpool::ThreadPool;
    /// use sangfroid::job::Job;
    ///
    /// let thread_pool = ThreadPool::<u8, u8>::new(1);
    ///
    /// let (job, result_src) = Job::with_result_sink(|x| x * 2, 2);
    /// thread_pool.schedule(job).expect("job not scheduled");
    /// assert_eq!(result_src.recv().unwrap(), 4);
    /// ```
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

    /// Returns a `JoinHandle` to a balancer thread for the given worker pool. The balancer
    /// listens on the given done receiver channel to receive Uids of workers who have
    /// finished their job and need to get their load decremented.
    /// The core loop of the balancer may be described as follows in pseudocode:
    /// ```text
    /// while uid = done_channel.recv() {
    ///     restore_worrker_pool_order(worker_pool, uid)
    /// }
    /// ```
    /// Since the worker pool is shared with the main thread for dispatching jobs, we need
    /// to wrap it in a Mutex.
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

    /// Creates the given number of workers and returns them in a vector along with the
    /// ends of the done channel. The workers send their Uid to the send end of the done
    /// channel to signify completion of a job. The balancer thread received on the
    /// receiving end of the done channel for uids and balances them accordingly.
    ///
    /// One of the key decisions behind this library is that we move channels where they
    /// are tobe used instead of sharing them with a lock. The send end of the channel
    /// is cloned and passed to each of the workers. The receiver end returned is meant
    /// to be moved to the balancer thread's closure.
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

    /// Schedules a new job to the worker pool by picking up the least loaded worker
    /// and dispatching the job to it.
    /// ```
    /// use sangfroid::threadpool::ThreadPool;
    /// use sangfroid::job::Job;
    ///
    /// let thread_pool = ThreadPool::<u8, u8>::new(2);
    ///
    /// let (job, result_src) = Job::with_result_sink(|x| x * 2, 2);
    /// thread_pool.schedule(job).expect("job not scheduled");
    /// assert_eq!(result_src.recv().unwrap(), 4);
    ///
    /// let (job, result_src) = Job::with_result_sink(|x| x * 3, 2);
    /// thread_pool.schedule(job).expect("job not scheduled");
    /// assert_eq!(result_src.recv().unwrap(), 6);
    ///
    /// let (job, result_src) = Job::with_result_sink(|x| x * 4, 2);
    /// thread_pool.schedule(job).expect("job not scheduled");
    /// assert_eq!(result_src.recv().unwrap(), 8);
    /// ```
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

    /// Terminates this threadpool by invoking `drop()` on the worker pool
    /// and the balancer thread, by sending a None to it via the done
    /// channel.
    ///
    /// We keep resources with custom cleanup inside Options so as to be
    /// able to move them in to the current scrope and `drop()` them.
    /// ```
    /// use sangfroid::threadpool::ThreadPool;
    /// use sangfroid::job::Job;
    ///
    /// let mut thread_pool = ThreadPool::<u8, u8>::new(2);
    /// thread_pool.terminate();
    /// ``` 
    pub fn terminate(&mut self) -> Result<(), ThreadPoolError> {
        // Ensure that all threads complete their jobs and
        // complete pending done notifications if any.
        // This is necessary since the receive end of the
        // done channel is to be dropped.
        if let Some(worker_pool) = self.pool.take() {
            worker_pool_terminate(
                worker_pool
                    .lock()
                    .or(Err(ThreadPoolError::LockError))?
                    .deref_mut(),
            )?;
        }

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
    use std::{
        collections::HashMap,
        sync::{Arc, RwLock},
        thread,
        time::Duration,
    };

    use crate::{job::Job, threadpool::ThreadPoolError};

    use super::ThreadPool;

    #[test]
    fn thread_pool_new_and_terminate() {
        ThreadPool::<(), ()>::new(0);

        ThreadPool::<(), ()>::new(10);
    }

    #[test]
    fn empty_thread_pool_schedule_job() {
        let empty_thread_pool = ThreadPool::<(), ()>::new(0);
        let job = Job::new(|()| (), ());
        match empty_thread_pool.schedule(job) {
            Err(ThreadPoolError::WorkerUnavailable) => {}
            _ => assert!(false),
        }
    }

    #[test]
    fn thread_pool_schedule_jobs() {
        let thread_pool = ThreadPool::<u8, u8>::new(1);

        let (job, result_src) = Job::with_result_sink(|x| x * 2, 2);
        thread_pool.schedule(job).expect("job not scheduled");
        assert_eq!(result_src.recv().unwrap(), 4);

        let (job, result_src) = Job::with_result_sink(|x| x * 3, 2);
        thread_pool.schedule(job).expect("job not scheduled");
        assert_eq!(result_src.recv().unwrap(), 6);

        let (job, result_src) = Job::with_result_sink(|x| x * 4, 2);
        thread_pool.schedule(job).expect("job not scheduled");
        assert_eq!(result_src.recv().unwrap(), 8);

        let (job, result_src) = Job::with_result_sink(|x| x * 5, 2);
        thread_pool.schedule(job).expect("job not scheduled");
        assert_eq!(result_src.recv().unwrap(), 10);

        // --

        let thread_pool = ThreadPool::<u8, u8>::new(2);

        let (job, result_src) = Job::with_result_sink(|x| x * 2, 2);
        thread_pool.schedule(job).expect("job not scheduled");
        assert_eq!(result_src.recv().unwrap(), 4);

        let (job, result_src) = Job::with_result_sink(
            |x| {
                thread::sleep(Duration::from_secs(3));

                return x * 2;
            },
            2,
        );
        thread_pool.schedule(job).expect("job not scheduled");
        assert_eq!(result_src.recv().unwrap(), 4);

        let (job, result_src) = Job::with_result_sink(|x| x * 3, 2);
        thread_pool.schedule(job).expect("job not scheduled");
        assert_eq!(result_src.recv().unwrap(), 6);

        let (job, result_src) = Job::with_result_sink(|x| x * 4, 2);
        thread_pool.schedule(job).expect("job not scheduled");
        assert_eq!(result_src.recv().unwrap(), 8);

        let (job, result_src) = Job::with_result_sink(
            |x| {
                thread::sleep(Duration::from_secs(1));

                return x * 4;
            },
            2,
        );
        thread_pool.schedule(job).expect("job not scheduled");
        assert_eq!(result_src.recv().unwrap(), 8);

        let (job, result_src) = Job::with_result_sink(|x| x * 5, 2);
        thread_pool.schedule(job).expect("job not scheduled");
        assert_eq!(result_src.recv().unwrap(), 10);
    }

    #[test]
    fn threadpool_schedule_jobs_with_shared_resource() {
        let shared_hashmap = Arc::new(RwLock::new(HashMap::<u8, u8>::new()));
        const PLACE_HOLDER: u8 = 5;

        assert_eq!(shared_hashmap.read().unwrap().get(&1), None);
        assert_eq!(shared_hashmap.read().unwrap().get(&7), None);

        let thread_pool = ThreadPool::<((u8, u8), Arc<RwLock<HashMap<u8, u8>>>), ()>::new(2);

        let job = Job::new(
            |((key, value), shared_map)| {
                shared_map.write().unwrap().insert(key, value);
            },
            ((1, 2), Arc::clone(&shared_hashmap)),
        );
        thread_pool.schedule(job).expect("job not scheduled");

        let job = Job::new(
            |((key, _), shared_map)| {
                shared_map.read().unwrap().get(&key);
            },
            ((1, PLACE_HOLDER), Arc::clone(&shared_hashmap)),
        );
        thread_pool.schedule(job).expect("job not scheduled");

        let job = Job::new(
            |((key, value), shared_map)| {
                shared_map.write().unwrap().insert(key, value);
            },
            ((7, 8), Arc::clone(&shared_hashmap)),
        );
        thread_pool.schedule(job).expect("job not scheduled");

        let job = Job::new(
            |((key, _), shared_map)| {
                shared_map.read().unwrap().get(&key);
            },
            ((5, PLACE_HOLDER), Arc::clone(&shared_hashmap)),
        );
        thread_pool.schedule(job).expect("job not scheduled");

        thread::sleep(Duration::from_secs(1));

        assert_eq!(shared_hashmap.read().unwrap().get(&1), Some(&2));
        assert_eq!(shared_hashmap.read().unwrap().get(&7), Some(&8));
    }
}
