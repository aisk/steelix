//! Interned Python strings for the names spelled in generated code.

use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::sync::Mutex;
use std::sync::atomic::{AtomicPtr, AtomicUsize, Ordering};

use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::PyString;

/// The interned Python string for a name spelled in generated code.
///
/// Looking a global or an attribute up by `&str` would create (and hash) a
/// new Python string every time, which costs several times the dictionary
/// lookup itself (measured: 44 ns per global read, 18 ns with a cache), and
/// CPython's attribute caches only work with interned names. Names are
/// `'static`, so their address and length identify them.
///
/// Like the strings of `pyo3::intern!`, they are never freed. Most names are
/// found in a table indexed by their address, read without a lock, which is
/// only correct with the GIL (see the check for free-threaded builds in `lib.rs`).
/// A name whose entries are all taken by other names goes to a map behind a
/// lock.
pub(crate) fn interned<'py>(
    py: Python<'py>,
    name: &'static str,
) -> Borrowed<'static, 'py, PyString> {
    let first = name_slot(name);
    for probe in 0..NAME_PROBES {
        let slot = &NAME_TABLE[(first + probe) % NAME_SLOTS];
        let mut address = slot.name.load(Ordering::Acquire);
        if address.is_null() {
            let string = PyString::intern(py, name);
            // Creating the string may have run other Python code, which may
            // have filled the slot. Otherwise no other thread runs until the
            // slot is written: attached threads take turns.
            address = slot.name.load(Ordering::Acquire);
            if address.is_null() {
                slot.len.store(name.len(), Ordering::Relaxed);
                slot.string.store(string.into_ptr(), Ordering::Relaxed);
                slot.name.store(name.as_ptr().cast_mut(), Ordering::Release);
                address = name.as_ptr().cast_mut();
            }
        }
        if address.cast_const() == name.as_ptr() && slot.len.load(Ordering::Relaxed) == name.len() {
            let string = slot.string.load(Ordering::Relaxed);
            // SAFETY: the slot owns a reference to this string, never released.
            return unsafe { Borrowed::from_ptr(py, string).cast_unchecked() };
        }
    }
    interned_slow(py, name)
}

/// One name in the table of [`interned`]: written once, `name` last.
struct NameSlot {
    name: AtomicPtr<u8>,
    len: AtomicUsize,
    string: AtomicPtr<ffi::PyObject>,
}

const NAME_SLOTS: usize = 4096;
const NAME_PROBES: usize = 4;

static NAME_TABLE: [NameSlot; NAME_SLOTS] = [const {
    NameSlot {
        name: AtomicPtr::new(std::ptr::null_mut()),
        len: AtomicUsize::new(0),
        string: AtomicPtr::new(std::ptr::null_mut()),
    }
}; NAME_SLOTS];

fn name_slot(name: &'static str) -> usize {
    let address = (name.as_ptr() as usize ^ name.len().rotate_left(48)) as u64;
    (address.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 52) as usize % NAME_SLOTS
}

fn interned_slow<'py>(py: Python<'py>, name: &'static str) -> Borrowed<'static, 'py, PyString> {
    type Names = HashMap<(usize, usize), &'static Py<PyString>, BuildHasherDefault<AddressHasher>>;
    static NAMES: Mutex<Names> = Mutex::new(HashMap::with_hasher(BuildHasherDefault::new()));
    let names = || {
        NAMES
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    };
    let key = (name.as_ptr() as usize, name.len());
    if let Some(string) = names().get(&key).copied() {
        return string.bind_borrowed(py);
    }
    // Created outside the lock: it may run Python code that looks up names
    // too. Another thread may have added the name meanwhile; then this
    // string is just dropped, it is the same interned string anyway.
    let string = PyString::intern(py, name).unbind();
    let mut names = names();
    let string = match names.get(&key) {
        Some(&existing) => existing,
        None => *names.entry(key).or_insert(Box::leak(Box::new(string))),
    };
    string.bind_borrowed(py)
}

#[derive(Default)]
struct AddressHasher(u64);

impl Hasher for AddressHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, _: &[u8]) {
        unreachable!("only addresses are hashed")
    }
    fn write_usize(&mut self, n: usize) {
        self.0 = self.0.rotate_left(5) ^ n as u64;
    }
}
