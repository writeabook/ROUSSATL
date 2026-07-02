//! No-std-compatible assertion macros for contract tests.
//!
//! These macros provide embedded-friendly failure reporting with
//! file and line information, avoiding the formatting overhead of
//! the standard library's `assert!` macros.

/// Assert that a condition is true.
///
/// On failure, panics with the file, line, and a static message.
#[macro_export]
macro_rules! osal_assert {
    ($cond:expr) => {
        if !$cond {
            panic!(
                "{}:{}: assertion failed: {}",
                file!(),
                line!(),
                stringify!($cond)
            );
        }
    };
    ($cond:expr, $($arg:tt)*) => {
        if !$cond {
            panic!(
                "{}:{}: assertion failed: {} — {}",
                file!(),
                line!(),
                stringify!($cond),
                format_args!($($arg)*)
            );
        }
    };
}

/// Assert that two values are equal.
#[macro_export]
macro_rules! osal_assert_eq {
    ($left:expr, $right:expr) => {
        match (&$left, &$right) {
            (left_val, right_val) => {
                if !(*left_val == *right_val) {
                    panic!(
                        "{}:{}: assertion failed: `(left == right)`\
                         \n  left: `{:?}`\
                         \n right: `{:?}`",
                        file!(),
                        line!(),
                        left_val,
                        right_val
                    );
                }
            }
        }
    };
}

/// Assert that a `Result` is `Ok`, returning the inner value.
#[macro_export]
macro_rules! osal_assert_ok {
    ($expr:expr) => {
        match $expr {
            Ok(val) => val,
            Err(e) => panic!(
                "{}:{}: expected Ok, got Err({:?}) in `{}`",
                file!(),
                line!(),
                e,
                stringify!($expr)
            ),
        }
    };
}

/// Assert that a `Result` is the expected `Error` variant.
#[macro_export]
macro_rules! osal_assert_err {
    ($expr:expr, $expected:pat) => {
        match $expr {
            Err($expected) => {}
            Ok(val) => panic!(
                "{}:{}: expected Err, got Ok({:?}) in `{}`",
                file!(),
                line!(),
                val,
                stringify!($expr)
            ),
            Err(e) => panic!(
                "{}:{}: expected matching error, got {:?} in `{}`",
                file!(),
                line!(),
                e,
                stringify!($expr)
            ),
        }
    };
}
