//! Message entity for communicating with workers.

use crate::job::Job;

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
