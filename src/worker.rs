//! Workers in a threadpool.

use std::{
    error::Error,
    fmt::{Debug, Display},
    sync::mpsc::{Receiver, Sender},
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
    result: Option<Sender<Res>>,
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

        let worker = thread::spawn(move || -> Result<(), WorkerError> {
            while let Ok(Message::Request(job)) = job_source.recv() {
                let mut task = job.task;

                println!("[{}]: req = {:?}", uid, &job.req);
                let result = task(job.req);
                println!("[{}]: res = {:?}", uid, &job.result);

                if let Some(sender) = job.result {
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

        match self.worker.take().unwrap().join() {
            Ok(result) => return result,
            Err(_) => return Err(WorkerError::JoinFailed),
        }
    }

    /// Increments pending tasks by 1.
    pub fn inc_load(&mut self) {
        self.pending += 1;
    }

    /// Decrements pending tasks by 1.
    pub fn dec_load(&mut self) {
        self.pending -= 1;
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
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}