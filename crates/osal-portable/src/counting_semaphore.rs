//! Portable counting semaphore state machine.
//!
//! `CountingSemaphoreState` handles the core count logic without any
//! threading, timeout, or platform primitives. Backends wrap this type
//! with their own synchronization (condvars for POSIX, Rc<RefCell<>> for
//! Mock).

use osal_api::error::{Error, Result};

/// Pure state for a counting semaphore.
///
/// Range: `0 <= count <= max_count`. Does not handle blocking or
/// timeouts — those are the backend's responsibility.
#[derive(Debug)]
pub struct CountingSemaphoreState {
    max_count: u32,
    count: u32,
}

impl CountingSemaphoreState {
    /// Create a new state with the given bounds.
    ///
    /// Returns `Error::InvalidParameter` if `max_count == 0` or
    /// `initial_count > max_count`.
    pub fn new(max_count: u32, initial_count: u32) -> Result<Self> {
        if max_count == 0 {
            return Err(Error::InvalidParameter);
        }
        if initial_count > max_count {
            return Err(Error::InvalidParameter);
        }
        Ok(Self {
            max_count,
            count: initial_count,
        })
    }

    /// Attempt to acquire one permit.
    ///
    /// Returns `true` if the permit was acquired (count was > 0 and
    /// was decremented). Returns `false` if count is zero.
    pub fn try_acquire(&mut self) -> bool {
        if self.count > 0 {
            self.count -= 1;
            true
        } else {
            false
        }
    }

    /// Release one permit.
    ///
    /// Returns `Error::Overflow` if already at `max_count`. The count
    /// is unchanged on overflow.
    pub fn release(&mut self) -> Result<()> {
        if self.count >= self.max_count {
            return Err(Error::Overflow);
        }
        self.count += 1;
        Ok(())
    }

    /// Maximum count configured at creation.
    pub fn max_count(&self) -> u32 {
        self.max_count
    }

    /// Current count.
    pub fn count(&self) -> u32 {
        self.count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_valid() {
        let s = CountingSemaphoreState::new(3, 2).unwrap();
        assert_eq!(s.max_count(), 3);
        assert_eq!(s.count(), 2);
    }

    #[test]
    fn reject_max_zero() {
        assert_eq!(
            CountingSemaphoreState::new(0, 0).unwrap_err(),
            Error::InvalidParameter
        );
    }

    #[test]
    fn reject_initial_gt_max() {
        assert_eq!(
            CountingSemaphoreState::new(3, 5).unwrap_err(),
            Error::InvalidParameter
        );
    }

    #[test]
    fn acquire_decrements() {
        let mut s = CountingSemaphoreState::new(3, 2).unwrap();
        assert!(s.try_acquire());
        assert_eq!(s.count(), 1);
    }

    #[test]
    fn acquire_on_empty_returns_false() {
        let mut s = CountingSemaphoreState::new(3, 0).unwrap();
        assert!(!s.try_acquire());
        assert_eq!(s.count(), 0);
    }

    #[test]
    fn release_increments() {
        let mut s = CountingSemaphoreState::new(3, 0).unwrap();
        s.release().unwrap();
        assert_eq!(s.count(), 1);
    }

    #[test]
    fn release_at_max_overflows() {
        let mut s = CountingSemaphoreState::new(2, 2).unwrap();
        assert_eq!(s.release().unwrap_err(), Error::Overflow);
        assert_eq!(s.count(), 2); // unchanged
    }

    #[test]
    fn full_cycle() {
        let mut s = CountingSemaphoreState::new(2, 0).unwrap();
        s.release().unwrap();
        s.release().unwrap();
        assert_eq!(s.release().unwrap_err(), Error::Overflow);
        assert!(s.try_acquire());
        assert!(s.try_acquire());
        assert!(!s.try_acquire());
    }

    #[test]
    fn max_count_never_changes() {
        let mut s = CountingSemaphoreState::new(5, 3).unwrap();
        s.try_acquire();
        s.release().unwrap();
        assert_eq!(s.max_count(), 5);
    }
}
