//! The iterator of a native `for` loop.

use pyo3::prelude::*;
use pyo3::types::PyIterator;

/// `for x in xs`: the items of `xs`, checking for signals before each item
/// after the first, where CPython's eval loop checks them at the end of
/// every iteration (so Ctrl-C stops a native loop with `KeyboardInterrupt`,
/// design §12). Like `xs.try_iter()?`, every item is a `PyResult`.
///
/// ```ignore
/// for x in rt::iter(&xs)? {
///     let x = x?;
///     ..
/// }
/// ```
pub fn iter<'py, T>(iterable: &Bound<'py, T>) -> PyResult<Iter<'py>> {
    Ok(Iter {
        iterator: iterable.as_any().try_iter()?,
        started: false,
    })
}

/// See [`iter`].
pub struct Iter<'py> {
    iterator: Bound<'py, PyIterator>,
    started: bool,
}

impl<'py> Iterator for Iter<'py> {
    type Item = PyResult<Bound<'py, PyAny>>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.started {
            if let Err(err) = self.iterator.py().check_signals() {
                return Some(Err(err));
            }
        } else {
            self.started = true;
        }
        self.iterator.next()
    }
}
