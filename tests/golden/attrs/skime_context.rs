// DRAFT of a `#[pyclass]` for class `Context` (skime/ctx.py:1),
// written by `steelix attrs -m skime --root tests/projects/skime --pyclass skime.ctx.Context`.
//
// This is advice for a human, not generated code: it is not part of the build,
// every body is `todo!()` and nothing checks it against the program. Design §15
// (类的改写, step 2) describes how to finish it, design §10 why steelix does
// not do it.
//
// The class is generated as `pub mod context` in src/skime/ctx.rs; the struct and
// its `#[pymethods]` replace that mod, and in `exec_module` the
// `rt::class!(..)` becomes `m.set_global("Context", py.get_type::<Context>())`.
//
// What blocks or complicates the upgrade:
// - names of its attributes are also used through receivers of unknown class: `bytecode` 6x in
//   skime.compiler.disasm, skime.ctx, skime.insns, skime.proc; `env` 22x in
//   skime.compiler.compiler, skime.compiler.disasm, skime.env, skime.form, skime.insns,
//   skime.invoke, skime.proc; `form` 3x in skime.compiler.compiler, skime.insns; `ip` 30x in
//   skime.call_cc, skime.insns; `parent` 12x in skime.compiler.builder,
//   skime.compiler.compiler, skime.compiler.disasm, skime.env, skime.insns, skime.invoke,
//   skime.proc; `vm` 3x in skime.ctx, skime.insns. See `steelix attrs --class
//   skime.ctx.Context`.
// - names of its methods are called through receivers of unknown class: `clone`, `insert`,
//   `pop`, `pop_n`, `push`, `top` (inlining them in the callers changes what a subclass
//   override does).
//
// The analysis only sees the project's modules: code outside the project, tests and `eval` may
// also read, write or subclass the class.

use pyo3::prelude::*;
use pyo3::types::{PyDict, PyTuple, PyType};

#[pyclass(module = "skime.ctx", name = "Context")]
pub struct Context {
    /// `self.form`, assigned in Context.__init__, read in Context.__init__, Context.clone.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    form: Py<PyAny>,
    /// `self.env`, assigned in Context.__init__, read in Context.clone.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    env: Py<PyAny>,
    /// `self.parent`, assigned in Context.__init__, read in Context.clone.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    parent: Py<PyAny>,
    /// `self.vm`, assigned in Context.__init__, read in Context.clone.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    vm: Py<PyAny>,
    /// `self.ip`, assigned in Context.__init__, Context.clone, read in Context.clone,
    /// Context.__str__.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    ip: Py<PyAny>,
    /// `self.bytecode`, assigned in Context.__init__.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    bytecode: Py<PyAny>,
    /// `self.stack`, assigned in Context.__init__, Context.clone, read in Context.clone,
    /// Context.push, Context.pop, Context.pop_n, Context.top, Context.insert, 2 more.
    // TODO: choose the Rust type (and `#[pyo3(get, set)]` if Python reads it).
    stack: Py<PyAny>,
}

#[pymethods]
impl Context {
    /// `def __init__(self, form, env, parent=None, vm=None)` (line 2), generated as
    /// `context::__init__` in src/skime/ctx.rs.
    // `#[new]` is `__new__`: it builds the struct from the arguments instead of
    // assigning attributes of an existing instance.
    // Called in Context.clone (skime/ctx.py:22), Form.eval (skime/form.py:28), invoke
    // (skime/invoke.py:41), VM.__init__ (skime/vm.py:25).
    #[new]
    #[pyo3(signature = (form, env, parent = None, vm = None))]
    fn new<'py>(
        py: Python<'py>,
        form: &Bound<'py, PyAny>,
        env: &Bound<'py, PyAny>,
        parent: Option<&Bound<'py, PyAny>>,
        vm: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Self> {
        todo!()
    }

    /// `def clone(self)` (line 20), generated as `context::clone` in src/skime/ctx.rs.
    fn clone<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `def push(self, val)` (line 27), generated as `context::push` in src/skime/ctx.rs.
    fn push<'py>(&self, py: Python<'py>, val: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `def pop(self)` (line 31), generated as `context::pop` in src/skime/ctx.rs.
    fn pop<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `def pop_n(self, n)` (line 35), generated as `context::pop_n` in src/skime/ctx.rs.
    fn pop_n<'py>(&self, py: Python<'py>, n: &Bound<'py, PyAny>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `def top(self, idx=1)` (line 40), generated as `context::top` in src/skime/ctx.rs.
    // TODO: `idx` defaults to `1` in Python.
    #[pyo3(signature = (idx = None))]
    fn top<'py>(&self, py: Python<'py>, idx: Option<&Bound<'py, PyAny>>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `def insert(self, idx, val)` (line 44), generated as `context::insert` in
    /// src/skime/ctx.rs.
    fn insert<'py>(
        &self,
        py: Python<'py>,
        idx: &Bound<'py, PyAny>,
        val: &Bound<'py, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `def insert_n(self, idx, val)` (line 51), generated as `context::insert_n` in
    /// src/skime/ctx.rs.
    fn insert_n<'py>(
        &self,
        py: Python<'py>,
        idx: &Bound<'py, PyAny>,
        val: &Bound<'py, PyAny>,
    ) -> PyResult<Py<PyAny>> {
        todo!()
    }

    /// `def __str__(self)` (line 58), generated as `context::__str__` in src/skime/ctx.rs.
    fn __str__<'py>(&self, py: Python<'py>) -> PyResult<Py<PyAny>> {
        todo!()
    }

    // pickle and `copy`: a `#[pyclass]` has no `__dict__` and does not pickle by
    // default. Add `__reduce__` returning the type and the arguments that rebuild
    // the instance (the type must stay reachable as `skime.ctx.Context`).
    // fn __reduce__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyTuple>> {
    //     todo!()
    // }
}
