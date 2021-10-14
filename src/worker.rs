//! Workers in a threadpool.

use std::{
    error::Error,
    fmt::{Debug, Display},
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

/// Represents a job to be submitted to the threadpool.
pub struct Job<Req, Res>
where
    Req: Send,
    Res: Send,
{
    /// Task to be executed
    task: Box<dyn FnMut(Req) -> Res + Send + 'static>,

    /// request to service
    req: Req,

    /// Optional result channel to send
    /// the result of the execution
    result_sink: Option<Sender<Res>>,
}

impl<Req, Res> Job<Req, Res>
where
    Req: Send,
    Res: Send,
{
    pub fn new<F>(f: F, req: Req) -> Self
    where
        F: FnMut(Req) -> Res + Send + 'static,
    {
        Job {
            task: Box::new(f),
            req,
            result_sink: None,
        }
    }

    pub fn with_result_sink<F>(f: F, req: Req) -> (Self, Receiver<Res>)
    where
        F: FnMut(Req) -> Res + Send + 'static,
    {
        let (tx, rx) = channel::<Res>();

        (
            Job {
                task: Box::new(f),
                req,
                result_sink: Some(tx),
            },
            rx,
        )
    }
}

/// Message represents a message to be sent to workers
/// in a threadpool.
pub enum Message<Req, Res>
where
    Req: Send,
    Res: Send,
{
    /// Request for job execution
    Request(Job<Req, Res>),

    /// Message the thread to terminate itself.
    Terminate,
}

/// Worker represents a worker thread capable for receiving
/// and servicing jobs.
pub struct Worker<Req, Res>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    uid: u64,

    disp_q: Sender<Message<Req, Res>>,
    job_source: Arc<Mutex<Receiver<Message<Req, Res>>>>,

    worker: Option<JoinHandle<Result<(), WorkerError>>>,
    pending: usize,
}

impl<Req, Res> bheap::Uid for Worker<Req, Res>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    fn uid(&self) -> u64 {
        self.uid
    }
}

#[derive(Debug)]
pub enum WorkerError {
    DoneNotificationFailed,
    ResultResponseFailed,
    DispatchFailed,
    TermNoticeFailed,
    JoinFailed,
}

impl Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self {
            WorkerError::DoneNotificationFailed => write!(f, "unable to signal completion"),
            WorkerError::ResultResponseFailed => write!(f, "unable to respond with result"),
            WorkerError::DispatchFailed => write!(f, "unable to dispatch job"),
            WorkerError::TermNoticeFailed => write!(f, "unable to send term signal"),
            WorkerError::JoinFailed => write!(f, "unable to join() on worker thread"),
        }
    }
}

impl Error for WorkerError {}

impl<Req, Res> Worker<Req, Res>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    /// Creates a new Worker from the given source, dispatch queue channel and done notice channel.
    /// The done channel and job_source channel are moved into the worker thread closure for
    /// receiving requests and notifying done status respectively.
    /// The core work loop for the worker thread may be expressed as follows:
    /// ```text
    /// // ...
    /// while let Ok(Request(job)) = job_source.recv() {
    ///     job.result_channel.send(job.task(job.req));
    ///     done.send(worker_uid);
    /// }
    /// // ...
    /// ```
    pub fn new(
        job_source: Receiver<Message<Req, Res>>,
        disp_q: Sender<Message<Req, Res>>,
        done: Sender<u64>,
        uid: u64,
    ) -> Self {
        let job_source = Arc::new(Mutex::new(job_source));

        let jobs = Arc::clone(&job_source);
        let worker = thread::spawn(move || -> Result<(), WorkerError> {
            while let Ok(Message::Request(job)) = jobs.lock().unwrap().recv() {
                let mut task = job.task;

                let result = task(job.req);

                if let Some(sender) = job.result_sink {
                    sender
                        .send(result)
                        .or(Err(WorkerError::ResultResponseFailed))?
                }

                done.send(uid)
                    .or(Err(WorkerError::DoneNotificationFailed))?
            }

            Ok(())
        });

        Worker {
            uid,
            disp_q,
            job_source,
            worker: Some(worker),
            pending: 0,
        }
    }

    /// Terminates this worker by sending a Terminate message to the underlying
    /// worker thread and the invoking join() on it.
    pub fn terminate(&mut self) -> Result<(), WorkerError> {
        if self.worker.is_none() {
            return Ok(());
        }

        self.disp_q
            .send(Message::Terminate)
            .or(Err(WorkerError::TermNoticeFailed))?;

        return match self.worker.take().unwrap().join() {
            Ok(result) => result,
            Err(_) => Err(WorkerError::JoinFailed),
        };
    }

    /// Increments pending tasks by 1.
    pub fn inc_load(&mut self) {
        self.pending += 1;
    }

    /// Decrements pending tasks by 1.
    pub fn dec_load(&mut self) {
        self.pending -= 1;
    }

    /// Steals a message from this worker and returns the job within
    /// that message.
    pub fn steal_job(&mut self) -> Option<Job<Req, Res>> {
        return match self.job_source.lock().unwrap().recv().ok() {
            Some(Message::Request(job)) => Some(job),
            _ => None,
        };
    }
}

impl<Req, Res> Drop for Worker<Req, Res>
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
    use std::sync::mpsc::channel;
    use super::{Job, Message, Worker};

    #[test]
    fn worker_new() {
        let (disp_q, jobs) = channel::<Message<(), ()>>();
        let (done, _) = channel::<u64>();

        Worker::new(jobs, disp_q, done, 0);
    }

    #[test]
    fn worker_task() {
        let (disp_q, jobs) = channel::<Message<(u8, u8), u8>>();
        let (done, _receiver) = channel::<u64>();

        {
            let _worker = Worker::new(jobs, disp_q.clone(), done.clone(), 1);

            let (job, result_src) =
                Job::with_result_sink(|(operand1, operand2): (u8, u8)| operand1 + operand2, (2, 2));

            disp_q
                .send(Message::Request(job))
                .expect("message not sent!");

            assert_eq!(result_src.recv().unwrap(), 4);
        }
    }

    #[test]
    fn worker_multiple_tasks() {
        let (disp_q, jobs) = channel::<Message<(u8, u8), u8>>();
        let (done, _receiver) = channel::<u64>();

        {
            let _worker = Worker::new(jobs, disp_q.clone(), done.clone(), 1);

            let (job, result_src) =
                Job::with_result_sink(|(operand1, operand2): (u8, u8)| operand1 + operand2, (2, 2));

            disp_q
                .send(Message::Request(job))
                .expect("message not sent!");

            assert_eq!(result_src.recv().unwrap(), 4);

            let (job, result_src) =
                Job::with_result_sink(|(operand1, operand2): (u8, u8)| operand1 - operand2, (2, 2));

            disp_q
                .send(Message::Request(job))
                .expect("message not sent!");

            assert_eq!(result_src.recv().unwrap(), 0);

            let (job, result_src) =
                Job::with_result_sink(|(operand1, operand2): (u8, u8)| operand1 * operand2, (2, 2));

            disp_q
                .send(Message::Request(job))
                .expect("message not sent!");

            assert_eq!(result_src.recv().unwrap(), 4);

            let (job, result_src) =
                Job::with_result_sink(|(operand1, operand2): (u8, u8)| operand1 / operand2, (2, 2));

            disp_q
                .send(Message::Request(job))
                .expect("message not sent!");

            assert_eq!(result_src.recv().unwrap(), 1);
        }
    }
}
