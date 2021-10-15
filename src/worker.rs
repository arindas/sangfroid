//! Workers in a threadpool.

use std::{
    error::Error,
    fmt::{Debug, Display},
    sync::mpsc::{Receiver, Sender},
    thread::{self, JoinHandle},
};

use crate::message::Message;

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
    ///
    /// The worker expects the `mpsc::Receiver` for the done `mpsc::Sender` to outlive itself.
    pub fn new(
        job_source: Receiver<Message<Req, Res>>,
        disp_q: Sender<Message<Req, Res>>,
        done: Sender<u64>,
        uid: u64,
    ) -> Self {
        Worker {
            uid,
            disp_q,
            worker: Some(Self::worker_thread(job_source, done, uid)),
            pending: 0,
        }
    }

    /// Creates a worker thread from the given job source, done notification channel and worker uid.
    /// This is not meant to be used directly. It is a advisable to construct a `Worker` instead
    /// since the `Worker` instance also manages the lifecycle and cleanup of the thread.
    /// ```text
    /// /// The worker thread core loop
    ///
    /// // ...
    /// while let Ok(Request(job)) = job_source.recv() {
    ///     job.result_channel.send(job.task(job.req));
    ///     done.send(worker_uid);
    /// }
    /// // ...
    /// ```
    #[inline]
    pub fn worker_thread(
        jobs: Receiver<Message<Req, Res>>,
        done: Sender<u64>,
        uid: u64,
    ) -> JoinHandle<Result<(), WorkerError>> {
        thread::spawn(move || -> Result<(), WorkerError> {
            let job_source = jobs;

            while let Ok(Message::Request(job)) = job_source.recv() {
                job.resp_with_result()
                    .or(Err(WorkerError::ResultResponseFailed))?;

                done.send(uid)
                    .or(Err(WorkerError::DoneNotificationFailed))?
            }

            Ok(())
        })
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
    use super::{Message, Worker};
    use crate::job::Job;
    use std::sync::mpsc::channel;

    #[test]
    fn worker_new() {
        let (disp_q, jobs) = channel::<Message<(), ()>>();
        let (done, _) = channel::<u64>();

        Worker::new(jobs, disp_q, done, 0);
    }

    #[test]
    fn worker_task() {
        let (disp_q, jobs) = channel::<Message<u8, u8>>();
        let (done, _receiver) = channel::<u64>();

        {
            let _worker = Worker::new(jobs, disp_q.clone(), done.clone(), 1);

            let (job, result_src) = Job::with_result_sink(|x: u8| x, 1);

            disp_q
                .send(Message::Request(job))
                .expect("message not sent!");

            assert_eq!(result_src.recv().unwrap(), 1);
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
