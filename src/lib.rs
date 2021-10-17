//![![ci-tests](https://github.com/arindas/sangfroid/actions/workflows/ci-tests.yml/badge.svg)](https://github.com/arindas/sangfroid/actions/workflows/ci-tests.yml)
//![![rustdoc](https://github.com/arindas/sangfroid/actions/workflows/rustdoc.yml/badge.svg)](https://github.com/arindas/sangfroid/actions/workflows/rustdoc.yml)
//!
//!A load balanced thread pool.
//!
//!## How does it work?
//!
//!We maintain a binary heap of worker threads. Worker threads are ordered by the number of
//!pending tasks associated with it. Whenever a new job is scheduled, we pick up the least
//!loaded worker and dispatch the job to it. We update its pending task count and restore
//!the heap accordingly.
//!
//!When a worker is done executing a job, it sends its Uid to the done channel. A background
//!balancer thread continuously receives on this channel until it receives a None value.
//!When the balancer thread receives an uid, it looks up the associated worker from the
//!worker pool heap, decrements it's load and restores the heap accordingly.
//!
//!All communication to and from threads is done via channels. We use locks as sparingly as
//!possible. More specifically, we only lock on the binary heap in the thread pool with a
//!Mutex since it is also used by the balancer thread.
//!We stay true to the concept:
//!>Do not communicate by sharing memory; instead, share memory by communicating.

pub mod job;
pub mod message;
pub mod threadpool;
pub mod worker;
