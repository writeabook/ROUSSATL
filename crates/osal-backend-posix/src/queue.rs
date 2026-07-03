//! POSIX queue implementation.
//!
//! Wraps [`ByteQueue`] with `pthread_mutex_t` + `pthread_cond_t` for
//! thread-safe access, implementing the [`Queue`] trait.

use alloc::sync::Arc;
use core::cell::UnsafeCell;

use osal_api::error::{Error, Result};
use osal_api::time::Timeout;
use osal_api::traits::queue::Queue;

use osal_portable::byte_queue::ByteQueue;
use osal_shared::validation;

use crate::sys::condvar::{self, PosixCondvar};
use crate::sys::mutex::PosixMutex;

// ---------------------------------------------------------------------------
// Inner state
// ---------------------------------------------------------------------------

struct QueueInner {
    mutex: PosixMutex,
    not_empty: PosixCondvar,
    not_full: PosixCondvar,
    buffer: UnsafeCell<ByteQueue>,
    closed: UnsafeCell<bool>,
}

unsafe impl Send for QueueInner {}
unsafe impl Sync for QueueInner {}

// ---------------------------------------------------------------------------
// Public type
// ---------------------------------------------------------------------------

pub struct PosixQueue {
    inner: Arc<QueueInner>,
}

impl Clone for PosixQueue {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl PosixQueue {
    pub fn new(capacity: usize, msg_size: usize) -> Result<Self> {
        validation::validate_queue_capacity(capacity)?;
        validation::validate_queue_message_size(msg_size)?;
        Ok(Self {
            inner: Arc::new(QueueInner {
                mutex: PosixMutex::new()?,
                not_empty: PosixCondvar::new()?,
                not_full: PosixCondvar::new()?,
                buffer: UnsafeCell::new(ByteQueue::new(capacity, msg_size)?),
                closed: UnsafeCell::new(false),
            }),
        })
    }

    fn buffer(&self) -> &mut ByteQueue {
        unsafe { &mut *self.inner.buffer.get() }
    }

    fn is_closed(&self) -> bool {
        unsafe { *self.inner.closed.get() }
    }

    fn set_closed(&self) {
        unsafe { *self.inner.closed.get() = true; }
    }
}

// ---------------------------------------------------------------------------
// Queue trait
// ---------------------------------------------------------------------------

impl Queue for PosixQueue {
    fn new(capacity: usize, msg_size: usize) -> Result<Self> {
        Self::new(capacity, msg_size)
    }

    fn send(&self, data: &[u8], timeout: Timeout) -> Result<()> {
        validation::validate_send_message_size(self.msg_size(), data.len())?;
        self.inner.mutex.lock()?;

        if self.is_closed() {
            self.inner.mutex.unlock()?;
            return Err(Error::QueueClosed);
        }

        match timeout {
            Timeout::NoWait => {
                let result = self.buffer().try_send(data);
                if result.is_ok() {
                    let _ = self.inner.not_empty.signal();
                }
                self.inner.mutex.unlock()?;
                result
            }
            Timeout::After(d) => {
                let deadline = condvar::abs_deadline(d);
                loop {
                    if self.is_closed() {
                        self.inner.mutex.unlock()?;
                        return Err(Error::QueueClosed);
                    }
                    match self.buffer().try_send(data) {
                        Ok(()) => {
                            let _ = self.inner.not_empty.signal();
                            self.inner.mutex.unlock()?;
                            return Ok(());
                        }
                        Err(Error::QueueFull) => {
                            // Wait on not_full with deadline.
                            match self.inner.not_full.timed_wait(&self.inner.mutex, &deadline) {
                                Err(Error::Timeout) => {
                                    self.inner.mutex.unlock()?;
                                    return Err(Error::Timeout);
                                }
                                Err(e) => {
                                    self.inner.mutex.unlock()?;
                                    return Err(e);
                                }
                                Ok(()) => { /* retry */ }
                            }
                        }
                        Err(e) => {
                            self.inner.mutex.unlock()?;
                            return Err(e);
                        }
                    }
                }
            }
            Timeout::Forever => loop {
                if self.is_closed() {
                    self.inner.mutex.unlock()?;
                    return Err(Error::QueueClosed);
                }
                match self.buffer().try_send(data) {
                    Ok(()) => {
                        let _ = self.inner.not_empty.signal();
                        self.inner.mutex.unlock()?;
                        return Ok(());
                    }
                    Err(Error::QueueFull) => {
                        self.inner.not_full.wait(&self.inner.mutex)?;
                    }
                    Err(e) => {
                        self.inner.mutex.unlock()?;
                        return Err(e);
                    }
                }
            },
        }
    }

    fn recv(&self, buffer: &mut [u8], timeout: Timeout) -> Result<()> {
        validation::validate_recv_buffer_size(self.msg_size(), buffer.len())?;
        self.inner.mutex.lock()?;

        match timeout {
            Timeout::NoWait => {
                let result = self.buffer().try_recv(buffer).map(|_| ());
                if result.is_ok() {
                    let _ = self.inner.not_full.signal();
                }
                self.inner.mutex.unlock()?;
                result
            }
            Timeout::After(d) => {
                let deadline = condvar::abs_deadline(d);
                loop {
                    if self.is_closed() && self.buffer().len() == 0 {
                        self.inner.mutex.unlock()?;
                        return Err(Error::QueueClosed);
                    }
                    match self.buffer().try_recv(buffer) {
                        Ok(_) => {
                            let _ = self.inner.not_full.signal();
                            self.inner.mutex.unlock()?;
                            return Ok(());
                        }
                        Err(Error::QueueEmpty) => {
                            if self.is_closed() {
                                self.inner.mutex.unlock()?;
                                return Err(Error::QueueClosed);
                            }
                            match self.inner.not_empty.timed_wait(&self.inner.mutex, &deadline) {
                                Err(Error::Timeout) => {
                                    self.inner.mutex.unlock()?;
                                    return Err(Error::Timeout);
                                }
                                Err(e) => {
                                    self.inner.mutex.unlock()?;
                                    return Err(e);
                                }
                                Ok(()) => { /* retry */ }
                            }
                        }
                        Err(e) => {
                            self.inner.mutex.unlock()?;
                            return Err(e);
                        }
                    }
                }
            }
            Timeout::Forever => loop {
                if self.is_closed() && self.buffer().len() == 0 {
                    self.inner.mutex.unlock()?;
                    return Err(Error::QueueClosed);
                }
                match self.buffer().try_recv(buffer) {
                    Ok(_) => {
                        let _ = self.inner.not_full.signal();
                        self.inner.mutex.unlock()?;
                        return Ok(());
                    }
                    Err(Error::QueueEmpty) => {
                        if self.is_closed() {
                            self.inner.mutex.unlock()?;
                            return Err(Error::QueueClosed);
                        }
                        self.inner.not_empty.wait(&self.inner.mutex)?;
                    }
                    Err(e) => {
                        self.inner.mutex.unlock()?;
                        return Err(e);
                    }
                }
            },
        }
    }

    fn close(&self) {
        self.inner.mutex.lock().ok();
        self.set_closed();
        self.buffer().close();
        let _ = self.inner.not_empty.broadcast();
        let _ = self.inner.not_full.broadcast();
        self.inner.mutex.unlock().ok();
    }

    fn isr_send(&self, data: &[u8]) -> Result<()> {
        self.send(data, Timeout::NoWait)
    }

    fn isr_recv(&self, buffer: &mut [u8]) -> Result<()> {
        self.recv(buffer, Timeout::NoWait)
    }

    fn capacity(&self) -> usize {
        self.inner.mutex.lock().ok();
        let c = self.buffer().capacity();
        self.inner.mutex.unlock().ok();
        c
    }

    fn msg_size(&self) -> usize {
        self.inner.mutex.lock().ok();
        let s = self.buffer().message_size();
        self.inner.mutex.unlock().ok();
        s
    }

    fn len(&self) -> usize {
        self.inner.mutex.lock().ok();
        let l = self.buffer().len();
        self.inner.mutex.unlock().ok();
        l
    }
}

// ---------------------------------------------------------------------------
// Factory (testkit)
// ---------------------------------------------------------------------------

pub struct PosixQueueFactory;

#[cfg(feature = "testkit")]
impl osal_testkit::factory::QueueFactory for PosixQueueFactory {
    type Queue = PosixQueue;

    fn create_queue(&self, capacity: usize, msg_size: usize) -> Result<Self::Queue> {
        PosixQueue::new(capacity, msg_size)
    }
}
