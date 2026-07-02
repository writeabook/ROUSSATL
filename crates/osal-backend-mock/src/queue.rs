//! Mock queue implementation.
//!
//! Wraps [`ByteQueue`] in `Rc<RefCell<>>` for shared ownership,
//! implementing the [`Queue`] trait for contract testing.

use alloc::rc::Rc;
use core::cell::RefCell;

use osal_api::error::Result;
use osal_api::time::Timeout;
use osal_api::traits::queue::Queue;

use osal_portable::byte_queue::ByteQueue;
use osal_shared::close_state::CloseFlag;
use osal_shared::validation;

// ---------------------------------------------------------------------------
// Inner state
// ---------------------------------------------------------------------------

struct MockQueueInner {
    buffer: ByteQueue,
    close_flag: CloseFlag,
}

impl MockQueueInner {
    fn new(capacity: usize, msg_size: usize) -> Result<Self> {
        validation::validate_queue_capacity(capacity)?;
        validation::validate_queue_message_size(msg_size)?;
        Ok(Self {
            buffer: ByteQueue::new(capacity, msg_size)?,
            close_flag: CloseFlag::new(),
        })
    }
}

// ---------------------------------------------------------------------------
// Public type
// ---------------------------------------------------------------------------

/// A mock queue for contract testing.
///
/// Uses `Rc<RefCell<>>` internally so cloned handles share the same
/// backend resource. Supports immediate (non-blocking) operations.
pub struct MockQueue {
    inner: Rc<RefCell<MockQueueInner>>,
}

impl Clone for MockQueue {
    fn clone(&self) -> Self {
        Self {
            inner: Rc::clone(&self.inner),
        }
    }
}

impl MockQueue {
    /// Create a new mock queue.
    pub fn new(capacity: usize, msg_size: usize) -> Result<Self> {
        Ok(Self {
            inner: Rc::new(RefCell::new(MockQueueInner::new(capacity, msg_size)?)),
        })
    }
}

// ---------------------------------------------------------------------------
// Queue trait
// ---------------------------------------------------------------------------

impl Queue for MockQueue {
    fn new(capacity: usize, msg_size: usize) -> Result<Self> {
        Self::new(capacity, msg_size)
    }

    fn send(&self, data: &[u8], timeout: Timeout) -> Result<()> {
        match timeout {
            Timeout::NoWait => self.inner.borrow_mut().buffer.try_send(data),
            Timeout::After(_) => {
                // Non-blocking only for now; blocking not yet implemented.
                self.inner.borrow_mut().buffer.try_send(data)
            }
            Timeout::Forever => {
                // Non-blocking only for now; blocking not yet implemented.
                self.inner.borrow_mut().buffer.try_send(data)
            }
        }
    }

    fn recv(&self, buffer: &mut [u8], timeout: Timeout) -> Result<()> {
        // Drain semantics: recv succeeds while messages remain even after close.
        match timeout {
            Timeout::NoWait => {
                let _n = self.inner.borrow_mut().buffer.try_recv(buffer)?;
                Ok(())
            }
            Timeout::After(_) | Timeout::Forever => {
                let _n = self.inner.borrow_mut().buffer.try_recv(buffer)?;
                Ok(())
            }
        }
    }

    fn close(&self) {
        self.inner.borrow_mut().close_flag.close();
        self.inner.borrow_mut().buffer.close();
    }

    fn isr_send(&self, data: &[u8]) -> Result<()> {
        self.send(data, Timeout::NoWait)
    }

    fn isr_recv(&self, buffer: &mut [u8]) -> Result<()> {
        self.recv(buffer, Timeout::NoWait)
    }

    fn capacity(&self) -> usize {
        self.inner.borrow().buffer.capacity()
    }

    fn msg_size(&self) -> usize {
        self.inner.borrow().buffer.message_size()
    }

    fn len(&self) -> usize {
        self.inner.borrow().buffer.len()
    }
}

// ---------------------------------------------------------------------------
// QueueFactory implementation
// ---------------------------------------------------------------------------

/// Factory for creating mock queues.
pub struct MockQueueFactory;

impl osal_testkit::factory::QueueFactory for MockQueueFactory {
    type Queue = MockQueue;

    fn create_queue(&self, capacity: usize, msg_size: usize) -> Result<Self::Queue> {
        MockQueue::new(capacity, msg_size)
    }
}
