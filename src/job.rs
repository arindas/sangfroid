//! Jobs to be dispatched and executed.

use std::sync::mpsc::{channel, Receiver, SendError, Sender};

/// Represents a job to be submitted to the threadpool.
pub struct Job<Req, Res>
where
    Req: Send,
    Res: Send,
{
    /// Task to be executed
    task: Box<dyn FnMut(Req) -> Res + Send + 'static>,

    /// request to service i.e args for task
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
    /// Creates a new job from a closure and Req arguments to be processed.
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


    /// Creates a new job like `::new()` with a result sink for responding
    /// the result computed.
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

    /// Consumes this job and responds with the result computed.
    pub fn resp_with_result(self) -> Result<(), SendError<Res>> {
        let mut task = self.task;
        let result = task(self.req);

        if let Some(sender) = self.result_sink {
            sender.send(result)?;
        }

        Ok(())
    }
}
