# sangfroid

A load balanced thread pool.

## How does it work?

We maintain a binary heap of worker threads. Worker threads are ordered by the number of
pending tasks associated with it. Whenever a new job is scheduled, we pick up the least
loaded worker and dispatch the job to it. We update its pending task count and restore
the heap accordingly.

When a worker is done executing a job, it sends its Uid to the done channel. A background
balancer thread continuously receives on this channel until it receives a None value.
When the balancer thread receives an uid, it looks up the associated worker from the
worker pool heap, decrements it's load and restores the heap accordingly.

All communication to and from threads is done via channels. We use locks as sparingly as
possible. More specifically, we only lock on the binary heap in the thread pool with a
Mutex since it is also used by the balancer thread.
We stay true to the concept:
>Do not communicate by sharing memory; instead, share memory by communicating.

## Why did you make this?
This crate was inspired from the load balancer described in the excellent
["Concurrency is not Parallelism"](https://youtu.be/oV9rvDllKEg) talk by Rob Pike.

## Well, did you learn anything?
Thread management and synchorization in Rust. I was also able to improve my ownership concepts.
This was a great project for applying the knowledge I assimilated from the aforementioned
talk and improve my ability to express my ideas in `Rust`.

## Dependencies
This does not depend on any third party crates. The only dependency is the
[`bheap`](github.com/arindas/bheap) crate, which I wrote for managing the workers.

## Usage
This is a library crate. You may include it in your `Cargo.toml` as follows:
```toml
[dependencies]
sangfroid = { git = "https://github.com/arindas/sangfroid" }
```

## Why the weird name?
>sangfroid /sɒ̃ˈfrwɑː/ noun;
>composure or coolness shown in danger or under trying circumstances.

I found it fitting.