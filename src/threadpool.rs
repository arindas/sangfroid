//! A load balanced threadpool.

use std::{
    fmt::Debug,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
};

use crate::worker::Worker;
use bheap::BinaryMaxHeap;

pub enum ThreadPoolError {
    LockError,
    LookupError,
}

pub struct ThreadPool<Req, Res>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    pool: Arc<Mutex<BinaryMaxHeap<Worker<Req, Res>>>>,
    done_channel: Sender<Option<u64>>,

    balancer: JoinHandle<()>,
}

impl<Req, Res> ThreadPool<Req, Res>
where
    Req: Send + Debug + 'static,
    Res: Send + Debug + 'static,
{
    pub fn balancer_thread(
        done_channel: Receiver<Option<u64>>,
        worker_heap: Arc<Mutex<BinaryMaxHeap<Worker<Req, Res>>>>,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            while let Ok(Some(uid)) = done_channel.recv() {
                let worker_heap_lock = worker_heap.lock();
                let mut workers = worker_heap_lock.unwrap();
                let i = workers.index_in_heap_from_uid(uid).unwrap();
                let worker = workers.get(i).unwrap();
                worker.dec_load();
                workers.restore_heap_property(i);
            }
        })
    }
}
