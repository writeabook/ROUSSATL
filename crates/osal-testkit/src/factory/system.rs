//! Factory for system operations.

use osal_api::traits::system::System;

/// Factory for accessing system-level operations.
pub trait SystemFactory {
    /// Concrete system type.
    type System: System;
}
