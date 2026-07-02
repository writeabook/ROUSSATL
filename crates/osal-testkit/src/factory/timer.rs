//! Factory for creating timer instances.

use osal_api::traits::timer::Timer;

/// Factory for creating timer instances in a backend-agnostic way.
pub trait TimerFactory {
    /// Concrete timer type.
    type Timer: Timer;
}
