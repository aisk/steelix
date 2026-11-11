//! `Function`: the Python-visible object for a transpiled `def` or `lambda`.
//!
//! `wrap_pyfunction!` produces a `builtin_function_or_method`, which can not
//! take attributes, can not have its `__name__` rewritten by
//! `functools.wraps`, and does not bind `self` when stored on a class. This
//! type behaves like a Python function instead: it binds arguments to its
//! parameters exactly like CPython (defaults, keywords, `*args`, `**kwargs`,
//! keyword-only and positional-only parameters, with the same `TypeError`
//! messages), has a `__dict__` and writable metadata, and implements
//! `__get__`.

use std::ffi::CString;
use std::sync::Mutex;

use pyo3::exceptions::{PyAttributeError, PyNameError, PyNotImplementedError, PyTypeError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple};
use pyo3::{IntoPyObjectExt, PyTraverseError, PyVisit, ffi};
use smallvec::SmallVec;

use crate::Module;

/// The parameter list of a function, fixed when it is transpiled.
///
/// `names` holds every parameter in source order: positional ones (the first
/// `posonly` of them positional-only), then `*args` if `varargs`, then the
/// keyword-only ones, then `**kwargs` if `varkw`. The Rust function receives
/// its arguments in the same order. Names may be spelled as Rust raw
/// identifiers (`r#type`); the `r#` is not part of the Python name.
pub struct Signature {
    pub names: &'static [&'static str],
    pub posonly: usize,
    pub positional: usize,
    pub varargs: bool,
    pub kwonly: usize,
    pub varkw: bool,
}

impl Signature {
    fn name(&self, index: usize) -> &'static str {
        python_name(self.names[index])
    }

    fn kwonly_start(&self) -> usize {
        self.positional + usize::from(self.varargs)
    }

    /// Whether all parameters are positional: no `*args`, keyword-only
    /// parameters or `**kwargs`.
    fn is_positional(&self) -> bool {
        self.names.len() == self.positional
    }

    /// Index of a parameter that can be passed by keyword.
    fn keyword_index(&self, keyword: &str) -> Option<usize> {
        let kwonly = self.kwonly_start()..self.kwonly_start() + self.kwonly;
        (self.posonly..self.positional)
            .chain(kwonly)
            .find(|&i| self.name(i) == keyword)
    }
}

/// Strips the `r#` of a Rust raw identifier, giving the Python name.
pub fn python_name(name: &'static str) -> &'static str {
    name.strip_prefix("r#").unwrap_or(name)
}

/// Signature shared by all function bodies stored in a [`Function`]: the
/// module, the closure cells and the bound arguments, one per parameter in
/// [`Signature`] order. The `function!` macro generates a small closure of
/// this type that hands them to the plain Rust fn.
pub type Body = for<'py> fn(
    &Module<'py>,
    &[Bound<'py, PyAny>],
    &[Bound<'py, PyAny>],
) -> PyResult<Bound<'py, PyAny>>;

/// What the Rust fn of a plain function returns: the object, or `()` for a
/// function that only ever returns `None`. `function!` hands the object to
/// Python either way.
pub trait IntoReturn<'py> {
    fn into_return(self, py: Python<'py>) -> Bound<'py, PyAny>;
}

impl<'py, T> IntoReturn<'py> for Bound<'py, T> {
    fn into_return(self, _py: Python<'py>) -> Bound<'py, PyAny> {
        self.into_any()
    }
}

impl<'py> IntoReturn<'py> for () {
    fn into_return(self, py: Python<'py>) -> Bound<'py, PyAny> {
        crate::none(py)
    }
}

/// What a call of the function runs.
pub enum Kind {
    /// A plain function: the body runs right away.
    Function(Body),
    /// `async def`: the call returns a coroutine, which runs the body when
    /// it is awaited (design §8).
    Coroutine(crate::coroutine::AsyncBody),
    /// A function with `yield`: the call returns a generator.
    Generator(crate::coroutine::AsyncBody),
}

/// Everything `function!` knows about a `def` or `lambda` when it executes.
pub struct FunctionDef<'py> {
    /// The Python name, when it is not the last part of `qualname`.
    pub name: Option<&'static str>,
    /// `None` for module level functions, whose qualified name is the name.
    pub qualname: Option<&'static str>,
    /// The name of the Rust function, the Python name if nothing else says.
    pub rust_name: &'static str,
    pub doc: Option<&'static str>,
    pub signature: &'static Signature,
    /// Values of the trailing positional parameters that have defaults.
    pub defaults: Vec<Bound<'py, PyAny>>,
    /// Keyword-only parameters that have defaults.
    pub kwdefaults: Vec<(&'static str, Bound<'py, PyAny>)>,
    /// Cell objects of the captured variables.
    pub closure: Vec<Bound<'py, PyAny>>,
    /// Names of the captured variables, as in `__code__.co_freevars`.
    pub closure_names: &'static [&'static str],
    /// Annotations as Python source, evaluated lazily like PEP 649.
    pub annotations: &'static [(&'static str, &'static str)],
    /// The `__classdict__` cell of the class body defining a method, where
    /// its annotations look names up first.
    pub classdict: Option<Bound<'py, PyAny>>,
    /// The qualified name of the class whose body defines the function: a
    /// method's qualified name is `<class>.<name>` unless `qualname` says
    /// otherwise.
    pub class_qualname: Option<&'static str>,
    /// The qualified name of the function whose body defines this one,
    /// which is then `<function>.<locals>.<name>` unless `qualname` says
    /// otherwise.
    pub enclosing_function: Option<&'static str>,
    pub body: Kind,
}

/// The `__name__` of a function created by `function!`: `name`, or the last
/// part of `qualname`, or the Rust name without `r#`.
#[doc(hidden)]
pub fn def_name(
    name: Option<&'static str>,
    qualname: Option<&'static str>,
    rust_name: &'static str,
) -> &'static str {
    match (name, qualname) {
        (Some(name), _) => name,
        (None, Some(qualname)) => qualname.rsplit('.').next().unwrap_or(qualname),
        (None, None) => python_name(rust_name),
    }
}

/// Where a `def` or `lambda` runs: in module (or function) code, or in a
/// class body, whose names the annotations of its methods can see.
#[doc(hidden)]
pub trait Definer<'py> {
    fn module(&self) -> &Module<'py>;
    /// The `__classdict__` cell, for a function with annotations.
    fn classdict(&self) -> Option<Bound<'py, PyAny>>;
    /// The qualified name of the class whose body this is.
    fn class_qualname(&self) -> Option<&'static str>;
    /// The qualified name of the function whose body this is (a
    /// [`Frame`](crate::Frame)): what it defines is `<function>.<locals>.<name>`.
    fn enclosing_function(&self) -> Option<&'static str> {
        None
    }
    /// Reads a name like the code there does: `LOAD_GLOBAL` in a module,
    /// `LOAD_NAME` in a class body. For the decorators of `def!`.
    fn lookup(&self, name: &'static str) -> PyResult<Bound<'py, PyAny>>;
    /// Binds the name of a `def`, in the globals or the class namespace.
    fn bind(&self, name: &'static str, value: Bound<'py, PyAny>) -> PyResult<()>;
}

impl<'py> Definer<'py> for Module<'py> {
    fn module(&self) -> &Module<'py> {
        self
    }
    fn classdict(&self) -> Option<Bound<'py, PyAny>> {
        None
    }
    fn class_qualname(&self) -> Option<&'static str> {
        None
    }
    fn lookup(&self, name: &'static str) -> PyResult<Bound<'py, PyAny>> {
        self.global(name)
    }
    fn bind(&self, name: &'static str, value: Bound<'py, PyAny>) -> PyResult<()> {
        self.set_global(name, value)
    }
}

impl<'py> Definer<'py> for crate::Namespace<'py> {
    fn module(&self) -> &Module<'py> {
        crate::Namespace::module(self)
    }
    fn classdict(&self) -> Option<Bound<'py, PyAny>> {
        // With `from __future__ import annotations` methods have no
        // annotation scopes, and CPython creates no `__classdict__` cell.
        (!self.future_annotations()).then(|| crate::Namespace::classdict(self).clone())
    }
    fn class_qualname(&self) -> Option<&'static str> {
        Some(self.qualname())
    }
    fn lookup(&self, name: &'static str) -> PyResult<Bound<'py, PyAny>> {
        self.get(name)
    }
    fn bind(&self, name: &'static str, value: Bound<'py, PyAny>) -> PyResult<()> {
        self.set(name, value)
    }
}

/// The writable attributes, behind a lock because the class is frozen. The
/// lock is never held while running Python code, which includes dropping a
/// value: a `__del__` may read the metadata again (see `Function::replace`).
struct Meta {
    name: Py<PyString>,
    qualname: Py<PyString>,
    doc: Py<PyAny>,
    module: Py<PyAny>,
    defaults: Option<Py<PyTuple>>,
    kwdefaults: Option<Py<PyDict>>,
    annotate: Option<Py<PyAny>>,
    annotations: Option<Py<PyDict>>,
    type_params: Py<PyTuple>,
}

#[pyclass(frozen, dict, weakref, name = "function", module = "steelix_runtime")]
pub struct Function {
    /// Read by CPython through `tp_vectorcall_offset`, see [`vectorcall`].
    vectorcall: ffi::vectorcallfunc,
    /// The name the `def` gave it, which `Module::default` uses to tell the
    /// function a direct call was compiled against from a replacement.
    def_name: &'static str,
    signature: &'static Signature,
    body: Kind,
    globals: Py<PyDict>,
    closure: Option<Py<PyTuple>>,
    closure_names: &'static [&'static str],
    meta: Mutex<Meta>,
}

impl Function {
    /// Creates a function the way executing a `def` statement (or evaluating
    /// a `lambda`) does.
    pub fn create<'py>(module: &Module<'py>, def: FunctionDef<'py>) -> PyResult<Bound<'py, PyAny>> {
        let py = module.py();
        let globals = module.globals();
        let name = def_name(def.name, def.qualname, def.rust_name);
        let qualname = match (def.qualname, def.class_qualname, def.enclosing_function) {
            (Some(qualname), ..) => qualname.to_owned(),
            (None, Some(class), _) => format!("{class}.{name}"),
            (None, None, Some(function)) => format!("{function}.<locals>.{name}"),
            (None, None, None) => name.to_owned(),
        };
        let defaults = (!def.defaults.is_empty())
            .then(|| PyTuple::new(py, def.defaults))
            .transpose()?;
        let kwdefaults = if def.kwdefaults.is_empty() {
            None
        } else {
            let dict = PyDict::new(py);
            for (name, value) in def.kwdefaults {
                dict.set_item(python_name(name), value)?;
            }
            Some(dict.unbind())
        };
        let closure = (!def.closure.is_empty())
            .then(|| PyTuple::new(py, def.closure))
            .transpose()?;
        let annotate = if def.annotations.is_empty() {
            None
        } else {
            let annotate = Annotate {
                globals: globals.clone().unbind(),
                owner: std::sync::OnceLock::new(),
                classdict: def.classdict.map(Bound::unbind),
                qualname: format!("{qualname}.__annotate__"),
                entries: def.annotations,
            };
            Some(Bound::new(py, annotate)?.into_any().unbind())
        };
        let meta = Meta {
            name: PyString::new(py, name).unbind(),
            qualname: PyString::new(py, &qualname).unbind(),
            doc: def.doc.into_py_any(py)?,
            module: match globals.get_item("__name__")? {
                Some(module) => module.unbind(),
                None => py.None(),
            },
            defaults: defaults.map(Bound::unbind),
            kwdefaults,
            annotate,
            annotations: None,
            type_params: PyTuple::empty(py).unbind(),
        };
        let function = Function {
            vectorcall,
            def_name: name,
            signature: def.signature,
            body: def.body,
            globals: globals.clone().unbind(),
            closure: closure.map(Bound::unbind),
            closure_names: def.closure_names,
            meta: Mutex::new(meta),
        };
        let function = Bound::new(py, function)?;
        enable_vectorcall(&function);
        Ok(function.into_any())
    }

    fn meta(&self) -> std::sync::MutexGuard<'_, Meta> {
        // A panic while holding the lock can only come from a bug here; the
        // data is still consistent, so keep going.
        self.meta
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Stores `value` in a field of the metadata and drops the old value
    /// after releasing the lock: dropping it may run a `__del__`, which may
    /// read this function's metadata (from this thread or another one).
    fn replace<T>(&self, field: impl FnOnce(&mut Meta) -> &mut T, value: T) {
        let old = std::mem::replace(field(&mut self.meta()), value);
        drop(old);
    }

    /// Whether this is the function that `def name` created in `globals`.
    pub(crate) fn is_def(&self, name: &str, globals: &Bound<'_, PyDict>) -> bool {
        self.def_name == name && self.globals.bind(globals.py()).is(globals)
    }

    fn defaults<'py>(&self, py: Python<'py>) -> Option<Bound<'py, PyTuple>> {
        let meta = self.meta();
        meta.defaults
            .as_ref()
            .map(|d| d.clone_ref(py).into_bound(py))
    }

    fn qualname(&self, py: Python<'_>) -> String {
        // `str()` of a `str` subclass may run Python code: not under the lock.
        let qualname = self.meta().qualname.clone_ref(py);
        qualname.bind(py).to_string()
    }

    /// The value a call that omits `param` uses, read from `__defaults__` or
    /// `__kwdefaults__` like CPython does at call time.
    pub(crate) fn default<'py>(&self, py: Python<'py>, param: &str) -> PyResult<Bound<'py, PyAny>> {
        let sig = self.signature;
        let Some(index) = (0..sig.names.len()).find(|&i| sig.name(i) == param) else {
            return Err(PyTypeError::new_err(format!(
                "{}() has no parameter '{param}'",
                self.qualname(py)
            )));
        };
        let (defaults, kwdefaults) = {
            let meta = self.meta();
            (
                meta.defaults.as_ref().map(|d| d.clone_ref(py)),
                meta.kwdefaults.as_ref().map(|d| d.clone_ref(py)),
            )
        };
        if index < sig.positional {
            let defaults = defaults.map(|d| d.into_bound(py));
            let count = defaults.as_ref().map_or(0, |d| d.len());
            let first = sig.positional as isize - count as isize;
            if let Some(defaults) = defaults
                && index as isize >= first
            {
                return defaults.get_item((index as isize - first) as usize);
            }
            return Err(missing(
                &self.qualname(py),
                "positional",
                &[sig.name(index)],
            ));
        }
        if let Some(kwdefaults) = kwdefaults
            && let Some(value) = kwdefaults.bind(py).get_item(param)?
        {
            return Ok(value);
        }
        Err(missing(
            &self.qualname(py),
            "keyword-only",
            &[sig.name(index)],
        ))
    }

    /// Calls the function: binds the arguments to the parameters and runs
    /// the body. `kwnames` are the keywords of `kwvalues`, like vectorcall.
    fn call<'py>(
        &self,
        py: Python<'py>,
        args: &[Bound<'py, PyAny>],
        kwnames: &[Bound<'py, PyAny>],
        kwvalues: &[Bound<'py, PyAny>],
    ) -> PyResult<Bound<'py, PyAny>> {
        let module = Module::from_globals(self.globals.bind(py).clone());
        let closure = match &self.closure {
            Some(closure) => closure.bind(py).as_slice(),
            None => &[],
        };
        let sig = self.signature;
        // The common calls need no binding: positional arguments only, for
        // a function with positional parameters only.
        if kwnames.is_empty() && sig.is_positional() {
            if args.len() == sig.positional {
                return self.run(&module, closure, args);
            }
            if args.len() < sig.positional
                && let Some(defaults) = self.defaults(py)
                && let Some(first) = defaults.len().checked_sub(sig.positional - args.len())
            {
                // The missing ones are the last parameters, whose defaults
                // are the last ones. On the stack: this is a common call.
                let defaults = &defaults.as_slice()[first..];
                let bound: SmallVec<[Bound<'py, PyAny>; 8]> =
                    args.iter().chain(defaults).cloned().collect();
                return self.run(&module, closure, &bound);
            }
        }
        let bound = self.bind(py, args, kwnames, kwvalues)?;
        self.run(&module, closure, &bound)
    }

    /// Runs the body with the bound arguments, or creates the coroutine or
    /// generator that will run it. Like CPython, its names are the
    /// function's names at the time of the call.
    fn run<'py>(
        &self,
        module: &Module<'py>,
        closure: &[Bound<'py, PyAny>],
        args: &[Bound<'py, PyAny>],
    ) -> PyResult<Bound<'py, PyAny>> {
        let (body, generator) = match self.body {
            Kind::Function(body) => return body(module, closure, args),
            Kind::Coroutine(body) => (body, false),
            Kind::Generator(body) => (body, true),
        };
        let py = module.py();
        let (name, qualname) = {
            let meta = self.meta();
            (meta.name.clone_ref(py), meta.qualname.clone_ref(py))
        };
        crate::coroutine::create(module, closure, args, body, generator, name, qualname)
    }

    /// Binds call arguments to parameters like CPython's `initialize_locals`,
    /// raising the same `TypeError`s.
    fn bind<'py>(
        &self,
        py: Python<'py>,
        args: &[Bound<'py, PyAny>],
        kwnames: &[Bound<'py, PyAny>],
        kwvalues: &[Bound<'py, PyAny>],
    ) -> PyResult<Vec<Bound<'py, PyAny>>> {
        let sig = self.signature;
        let (defaults, kwdefaults) = {
            let meta = self.meta();
            (
                meta.defaults
                    .as_ref()
                    .map(|d| d.clone_ref(py).into_bound(py)),
                meta.kwdefaults
                    .as_ref()
                    .map(|d| d.clone_ref(py).into_bound(py)),
            )
        };
        let mut slots: Vec<Option<Bound<'py, PyAny>>> = vec![None; sig.names.len()];
        let given = args.len();
        for (slot, arg) in slots.iter_mut().zip(args).take(sig.positional) {
            *slot = Some(arg.clone());
        }
        if sig.varargs {
            let extra = PyTuple::new(py, &args[sig.positional.min(given)..])?;
            slots[sig.positional] = Some(extra.into_any());
        }
        let varkw = if sig.varkw {
            let dict = PyDict::new(py);
            slots[sig.names.len() - 1] = Some(dict.clone().into_any());
            Some(dict)
        } else {
            None
        };

        for (key, value) in kwnames.iter().zip(kwvalues) {
            let Ok(keyword) = key.cast::<PyString>() else {
                return Err(PyTypeError::new_err("keywords must be strings"));
            };
            let keyword = keyword.to_cow()?;
            match sig.keyword_index(&keyword) {
                Some(index) if slots[index].is_some() => {
                    return Err(PyTypeError::new_err(format!(
                        "{}() got multiple values for argument '{keyword}'",
                        self.qualname(py)
                    )));
                }
                Some(index) => slots[index] = Some(value.clone()),
                None => match &varkw {
                    Some(dict) => dict.set_item(key, value)?,
                    None => return Err(self.unexpected_keyword(py, kwnames, &keyword)),
                },
            }
        }

        if given > sig.positional && !sig.varargs {
            return Err(self.too_many_positional(py, &slots, given, defaults.as_ref()));
        }

        if given < sig.positional {
            let count = defaults.as_ref().map_or(0, |d| d.len());
            let first = sig.positional.saturating_sub(count);
            let missing_names: Vec<&str> = (given..first)
                .filter(|&i| slots[i].is_none())
                .map(|i| sig.name(i))
                .collect();
            if !missing_names.is_empty() {
                return Err(missing(&self.qualname(py), "positional", &missing_names));
            }
            if let Some(defaults) = &defaults {
                // Defaults line up with the last positional parameters.
                let offset = count as isize - sig.positional as isize;
                for (i, slot) in slots
                    .iter_mut()
                    .enumerate()
                    .take(sig.positional)
                    .skip(first)
                {
                    if slot.is_none() {
                        *slot = Some(defaults.get_item((i as isize + offset) as usize)?);
                    }
                }
            }
        }

        if sig.kwonly > 0 {
            let start = sig.kwonly_start();
            let mut missing_names = Vec::new();
            for (i, slot) in slots.iter_mut().enumerate().skip(start).take(sig.kwonly) {
                if slot.is_some() {
                    continue;
                }
                match kwdefaults
                    .as_ref()
                    .map(|d| d.get_item(crate::interned(py, sig.name(i))))
                    .transpose()?
                    .flatten()
                {
                    Some(value) => *slot = Some(value),
                    None => missing_names.push(sig.name(i)),
                }
            }
            if !missing_names.is_empty() {
                return Err(missing(&self.qualname(py), "keyword-only", &missing_names));
            }
        }

        Ok(slots
            .into_iter()
            .map(|slot| slot.expect("every parameter is bound"))
            .collect())
    }

    fn unexpected_keyword(
        &self,
        py: Python<'_>,
        kwnames: &[Bound<'_, PyAny>],
        keyword: &str,
    ) -> PyErr {
        let sig = self.signature;
        let qualname = self.qualname(py);
        // Positional-only parameters passed by keyword get their own message.
        let posonly: Vec<&str> = (0..sig.posonly)
            .map(|i| sig.name(i))
            .filter(|&name| kwnames.iter().any(|key| key.eq(name).unwrap_or(false)))
            .collect();
        if !posonly.is_empty() {
            return PyTypeError::new_err(format!(
                "{qualname}() got some positional-only arguments passed as keyword arguments: '{}'",
                posonly.join(", ")
            ));
        }
        let candidates: Vec<&str> = (sig.posonly..sig.names.len())
            .filter(|&i| !(sig.varargs && i == sig.positional))
            .filter(|&i| !(sig.varkw && i == sig.names.len() - 1))
            .map(|i| sig.name(i))
            .collect();
        match suggestion(py, &candidates, keyword) {
            Some(suggestion) => PyTypeError::new_err(format!(
                "{qualname}() got an unexpected keyword argument '{keyword}'. Did you mean '{suggestion}'?"
            )),
            None => PyTypeError::new_err(format!(
                "{qualname}() got an unexpected keyword argument '{keyword}'"
            )),
        }
    }

    fn too_many_positional(
        &self,
        py: Python<'_>,
        slots: &[Option<Bound<'_, PyAny>>],
        given: usize,
        defaults: Option<&Bound<'_, PyTuple>>,
    ) -> PyErr {
        let sig = self.signature;
        let start = sig.kwonly_start();
        let kwonly_given = slots[start..start + sig.kwonly]
            .iter()
            .filter(|slot| slot.is_some())
            .count();
        let count = defaults.map_or(0, |d| d.len());
        let (expected, plural) = if count > 0 {
            let at_least = sig.positional as isize - count as isize;
            (format!("from {at_least} to {}", sig.positional), true)
        } else {
            (sig.positional.to_string(), sig.positional != 1)
        };
        let kwonly = if kwonly_given > 0 {
            format!(
                " positional argument{} (and {kwonly_given} keyword-only argument{})",
                if given != 1 { "s" } else { "" },
                if kwonly_given != 1 { "s" } else { "" }
            )
        } else {
            String::new()
        };
        PyTypeError::new_err(format!(
            "{}() takes {expected} positional argument{} but {given}{kwonly} {} given",
            self.qualname(py),
            if plural { "s" } else { "" },
            if given == 1 && kwonly_given == 0 {
                "was"
            } else {
                "were"
            }
        ))
    }
}

#[pymethods]
impl Function {
    #[pyo3(signature = (*args, **kwargs))]
    fn __call__<'py>(
        slf: &Bound<'py, Self>,
        args: &Bound<'py, PyTuple>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        let (kwnames, kwvalues): (Vec<_>, Vec<_>) = match kwargs {
            Some(kwargs) => kwargs.iter().unzip(),
            None => (Vec::new(), Vec::new()),
        };
        slf.get().call(py, args.as_slice(), &kwnames, &kwvalues)
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        instance: Option<&Bound<'py, PyAny>>,
        _owner: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        match instance {
            Some(instance) if !instance.is_none() => {
                // SAFETY: valid object pointers; the result is a new reference or NULL.
                unsafe {
                    Bound::from_owned_ptr_or_err(
                        slf.py(),
                        PyMethod_New(slf.as_ptr(), instance.as_ptr()),
                    )
                }
            }
            _ => Ok(slf.clone().into_any()),
        }
    }

    fn __repr__(slf: &Bound<'_, Self>) -> String {
        let qualname = slf.get().qualname(slf.py());
        format!("<function {qualname} at {:#x}>", slf.as_ptr() as usize)
    }

    #[getter]
    fn __name__(&self, py: Python<'_>) -> Py<PyString> {
        self.meta().name.clone_ref(py)
    }

    #[setter(__name__)]
    fn set_name(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        // Deleting these attributes raises `AttributeError` before getting here.
        let value = Some(value);
        let name = string_attribute(value, "__name__")?;
        self.replace(|meta| &mut meta.name, name);
        Ok(())
    }

    #[getter]
    fn __qualname__(&self, py: Python<'_>) -> Py<PyString> {
        self.meta().qualname.clone_ref(py)
    }

    #[setter(__qualname__)]
    fn set_qualname(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        // Deleting these attributes raises `AttributeError` before getting here.
        let value = Some(value);
        let qualname = string_attribute(value, "__qualname__")?;
        self.replace(|meta| &mut meta.qualname, qualname);
        Ok(())
    }

    #[getter]
    fn __doc__(&self, py: Python<'_>) -> Py<PyAny> {
        self.meta().doc.clone_ref(py)
    }

    #[setter(__doc__)]
    fn set_doc(&self, py: Python<'_>, value: &Bound<'_, PyAny>) {
        // Deleting these attributes raises `AttributeError` before getting here.
        let value = Some(value);
        let doc = value.map_or_else(|| py.None(), |v| v.clone().unbind());
        self.replace(|meta| &mut meta.doc, doc);
    }

    #[getter]
    fn __module__(&self, py: Python<'_>) -> Py<PyAny> {
        self.meta().module.clone_ref(py)
    }

    #[setter(__module__)]
    fn set_module(&self, py: Python<'_>, value: &Bound<'_, PyAny>) {
        // Deleting these attributes raises `AttributeError` before getting here.
        let value = Some(value);
        let module = value.map_or_else(|| py.None(), |v| v.clone().unbind());
        self.replace(|meta| &mut meta.module, module);
    }

    #[getter]
    fn __defaults__(&self, py: Python<'_>) -> Option<Py<PyTuple>> {
        self.meta().defaults.as_ref().map(|d| d.clone_ref(py))
    }

    #[setter(__defaults__)]
    fn set_defaults(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        // Deleting these attributes raises `AttributeError` before getting here.
        let value = Some(value);
        let value = match value {
            Some(value) if !value.is_none() => match value.cast::<PyTuple>() {
                Ok(tuple) => Some(tuple.clone().unbind()),
                Err(_) => {
                    return Err(PyTypeError::new_err(
                        "__defaults__ must be set to a tuple object",
                    ));
                }
            },
            _ => None,
        };
        self.replace(|meta| &mut meta.defaults, value);
        Ok(())
    }

    #[getter]
    fn __kwdefaults__(&self, py: Python<'_>) -> Option<Py<PyDict>> {
        self.meta().kwdefaults.as_ref().map(|d| d.clone_ref(py))
    }

    #[setter(__kwdefaults__)]
    fn set_kwdefaults(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        // Deleting these attributes raises `AttributeError` before getting here.
        let value = Some(value);
        let value = match value {
            Some(value) if !value.is_none() => match value.cast::<PyDict>() {
                Ok(dict) => Some(dict.clone().unbind()),
                Err(_) => {
                    return Err(PyTypeError::new_err(
                        "__kwdefaults__ must be set to a dict object",
                    ));
                }
            },
            _ => None,
        };
        self.replace(|meta| &mut meta.kwdefaults, value);
        Ok(())
    }

    /// Computed on first access by calling `__annotate__(VALUE)` (PEP 649).
    #[getter]
    fn __annotations__<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let annotate = {
            let meta = self.meta();
            if let Some(annotations) = &meta.annotations {
                return Ok(annotations.bind(py).clone());
            }
            meta.annotate.as_ref().map(|a| a.clone_ref(py))
        };
        let annotations = match annotate {
            Some(annotate) => {
                let value = annotate.bind(py).call1((FORMAT_VALUE,))?;
                match value.cast_into::<PyDict>() {
                    Ok(dict) => dict,
                    Err(err) => {
                        return Err(PyTypeError::new_err(format!(
                            "__annotate__ returned non-dict of type '{}'",
                            err.into_inner().get_type().name()?
                        )));
                    }
                }
            }
            None => PyDict::new(py),
        };
        self.replace(
            |meta| &mut meta.annotations,
            Some(annotations.clone().unbind()),
        );
        Ok(annotations)
    }

    #[setter(__annotations__)]
    fn set_annotations(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        // Deleting these attributes raises `AttributeError` before getting here.
        let value = Some(value);
        let value = match value {
            Some(value) if !value.is_none() => match value.cast::<PyDict>() {
                Ok(dict) => Some(dict.clone().unbind()),
                Err(_) => {
                    return Err(PyTypeError::new_err(
                        "__annotations__ must be set to a dict object",
                    ));
                }
            },
            _ => None,
        };
        let old = {
            let mut meta = self.meta();
            (
                std::mem::replace(&mut meta.annotations, value),
                meta.annotate.take(),
            )
        };
        drop(old);
        Ok(())
    }

    #[getter]
    fn __annotate__(&self, py: Python<'_>) -> Py<PyAny> {
        match &self.meta().annotate {
            Some(annotate) => annotate.clone_ref(py),
            None => py.None(),
        }
    }

    #[setter(__annotate__)]
    fn set_annotate(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        // Deleting these attributes raises `AttributeError` before getting here.
        if value.is_none() {
            self.replace(|meta| &mut meta.annotate, None);
        } else if value.is_callable() {
            let old = {
                let mut meta = self.meta();
                (
                    meta.annotate.replace(value.clone().unbind()),
                    meta.annotations.take(),
                )
            };
            drop(old);
        } else {
            return Err(PyTypeError::new_err(
                "__annotate__ must be callable or None",
            ));
        }
        Ok(())
    }

    #[getter]
    fn __type_params__(&self, py: Python<'_>) -> Py<PyTuple> {
        self.meta().type_params.clone_ref(py)
    }

    #[setter(__type_params__)]
    fn set_type_params(&self, value: &Bound<'_, PyAny>) -> PyResult<()> {
        // Deleting these attributes raises `AttributeError` before getting here.
        let value = Some(value);
        match value.map(|v| v.cast::<PyTuple>()) {
            Some(Ok(tuple)) => {
                self.replace(|meta| &mut meta.type_params, tuple.clone().unbind());
                Ok(())
            }
            _ => Err(PyTypeError::new_err(
                "__type_params__ must be set to a tuple",
            )),
        }
    }

    /// What `inspect.signature` reports, built from the parameter list like
    /// CPython builds it from `__code__`. A function wrapped with
    /// `functools.wraps` has none, so that `inspect` follows `__wrapped__`.
    #[getter]
    fn __signature__<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        if slf.getattr("__dict__")?.contains("__wrapped__")? {
            return Err(PyAttributeError::new_err("__signature__"));
        }
        let this = slf.get();
        let sig = this.signature;
        let inspect = py.import("inspect")?;
        let parameter = inspect.getattr("Parameter")?;
        let empty = parameter.getattr("empty")?;
        let annotations = Self::__annotations__(this, py)?;
        let (defaults, kwdefaults) = {
            let meta = this.meta();
            (
                meta.defaults
                    .as_ref()
                    .map(|d| d.clone_ref(py).into_bound(py)),
                meta.kwdefaults
                    .as_ref()
                    .map(|d| d.clone_ref(py).into_bound(py)),
            )
        };
        let count = defaults.as_ref().map_or(0, |d| d.len());
        let first_default = sig.positional as isize - count as isize;
        let mut params = Vec::new();
        for i in 0..sig.names.len() {
            let name = sig.name(i);
            let (kind, default) = if i < sig.positional {
                let kind = if i < sig.posonly {
                    "POSITIONAL_ONLY"
                } else {
                    "POSITIONAL_OR_KEYWORD"
                };
                let default = match &defaults {
                    Some(d) if i as isize >= first_default => {
                        Some(d.get_item((i as isize - first_default) as usize)?)
                    }
                    _ => None,
                };
                (kind, default)
            } else if sig.varargs && i == sig.positional {
                ("VAR_POSITIONAL", None)
            } else if sig.varkw && i == sig.names.len() - 1 {
                ("VAR_KEYWORD", None)
            } else {
                let default = match &kwdefaults {
                    Some(d) => d.get_item(name)?,
                    None => None,
                };
                ("KEYWORD_ONLY", default)
            };
            let kwargs = PyDict::new(py);
            kwargs.set_item("default", default.unwrap_or_else(|| empty.clone()))?;
            kwargs.set_item(
                "annotation",
                annotations.get_item(name)?.unwrap_or_else(|| empty.clone()),
            )?;
            params.push(parameter.call((name, parameter.getattr(kind)?), Some(&kwargs))?);
        }
        let kwargs = PyDict::new(py);
        if let Some(ret) = annotations.get_item("return")? {
            kwargs.set_item("return_annotation", ret)?;
        }
        inspect.getattr("Signature")?.call((params,), Some(&kwargs))
    }

    /// Pickles by reference to the module level name, like CPython functions.
    fn __reduce__(&self, py: Python<'_>) -> Py<PyString> {
        self.meta().qualname.clone_ref(py)
    }

    #[getter]
    fn __globals__<'py>(&self, py: Python<'py>) -> Bound<'py, PyDict> {
        self.globals.bind(py).clone()
    }

    #[getter]
    fn __closure__(&self, py: Python<'_>) -> Option<Py<PyTuple>> {
        self.closure.as_ref().map(|c| c.clone_ref(py))
    }

    /// A code object describing the parameters and the captured variables
    /// (`co_freevars`, in the order of `__closure__`), without bytecode.
    /// `dataclass(slots=True)` uses it to find the `__class__` cell.
    #[getter]
    fn __code__<'py>(slf: &Bound<'py, Self>) -> PyResult<Bound<'py, PyAny>> {
        let py = slf.py();
        let this = slf.get();
        let sig = this.signature;
        let module = Module::from_globals(this.globals.bind(py).clone());
        let filename = crate::python::filename(&module)?;
        let (name, qualname) = {
            let meta = this.meta();
            (meta.name.clone_ref(py), meta.qualname.clone_ref(py))
        };
        let empty = py
            .import("builtins")?
            .getattr("compile")?
            .call1(("", filename, "exec"))?;
        let names: Vec<&str> = (0..sig.names.len()).map(|i| sig.name(i)).collect();
        // CO_OPTIMIZED | CO_NEWLOCALS, CO_VARARGS, CO_VARKEYWORDS, CO_NESTED.
        let mut flags = 0x3;
        if sig.varargs {
            flags |= 0x4;
        }
        if sig.varkw {
            flags |= 0x8;
        }
        if this.closure.is_some() {
            flags |= 0x10;
        }
        // CO_GENERATOR, CO_COROUTINE: `inspect.isgeneratorfunction` and
        // `inspect.iscoroutinefunction` read them.
        match this.body {
            Kind::Function(_) => {}
            Kind::Generator(_) => flags |= 0x20,
            Kind::Coroutine(_) => flags |= 0x80,
        }
        let changes = PyDict::new(py);
        changes.set_item("co_name", name)?;
        changes.set_item("co_qualname", qualname)?;
        changes.set_item("co_argcount", sig.positional)?;
        changes.set_item("co_posonlyargcount", sig.posonly)?;
        changes.set_item("co_kwonlyargcount", sig.kwonly)?;
        changes.set_item("co_nlocals", names.len())?;
        changes.set_item("co_varnames", PyTuple::new(py, &names)?)?;
        let frees = this.closure_names.iter().map(|name| python_name(name));
        changes.set_item("co_freevars", PyTuple::new(py, frees)?)?;
        changes.set_item("co_flags", flags)?;
        empty.call_method("replace", (), Some(&changes))
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.globals)?;
        visit.call(&self.closure)?;
        // The lock is only held briefly and never across Python code, so it
        // is free unless another thread is inside a getter or setter.
        if let Ok(meta) = self.meta.try_lock() {
            visit.call(&meta.name)?;
            visit.call(&meta.qualname)?;
            visit.call(&meta.doc)?;
            visit.call(&meta.module)?;
            visit.call(&meta.defaults)?;
            visit.call(&meta.kwdefaults)?;
            visit.call(&meta.annotate)?;
            visit.call(&meta.annotations)?;
            visit.call(&meta.type_params)?;
        }
        Ok(())
    }

    fn __clear__(&self) {
        let old = {
            let mut meta = self.meta();
            (
                meta.defaults.take(),
                meta.kwdefaults.take(),
                meta.annotate.take(),
                meta.annotations.take(),
            )
        };
        drop(old);
    }
}

/// Lets CPython call functions without building an argument tuple and
/// dict (PEP 590), and lets `obj.method()` call a method without creating a
/// bound method object, like it does for Python functions.
///
/// PyO3 classes can not declare either, so the type object is completed
/// when the first function is created: `tp_vectorcall_offset` points at the
/// `vectorcall` field, and `Py_TPFLAGS_METHOD_DESCRIPTOR` promises that
/// `f.__get__(obj)(*args)` is `f(obj, *args)`, which `__get__` guarantees.
/// `__call__` and `__get__` stay as they are for everything else.
fn enable_vectorcall(function: &Bound<'_, Function>) {
    static DONE: std::sync::Once = std::sync::Once::new();
    DONE.call_once(|| {
        let field = &function.get().vectorcall as *const ffi::vectorcallfunc;
        let offset = field as isize - function.as_ptr() as isize;
        // SAFETY: we are attached to the interpreter and no function exists
        // yet besides this one, so nothing has looked at these slots. Every
        // instance has the same layout, so the field is at the same offset.
        unsafe {
            let ty = function.get_type_ptr();
            (*ty).tp_vectorcall_offset = offset as ffi::Py_ssize_t;
            (*ty).tp_flags |= ffi::Py_TPFLAGS_HAVE_VECTORCALL | ffi::Py_TPFLAGS_METHOD_DESCRIPTOR;
            ffi::PyType_Modified(ty);
        }
    });
}

/// The vectorcall entry point (`vectorcallfunc`): positional arguments
/// first, then the values of the keywords named in `kwnames`.
///
/// It goes through the trampoline PyO3 wraps its own fastcall functions in
/// (same arguments, `nargsf` as a signed size), which turns errors and
/// panics into exceptions and tells PyO3 that the thread is attached.
unsafe extern "C" fn vectorcall(
    callable: *mut ffi::PyObject,
    args: *const *mut ffi::PyObject,
    nargsf: usize,
    kwnames: *mut ffi::PyObject,
) -> *mut ffi::PyObject {
    use pyo3::impl_::trampoline::{MethodDef, fastcall_cfunction_with_keywords as fastcall};
    struct Call;
    impl MethodDef<fastcall::Func> for Call {
        const METH: fastcall::Func = call_vector;
    }
    // SAFETY: CPython calls this with the thread attached.
    unsafe { fastcall::<Call>(callable, args, nargsf as ffi::Py_ssize_t, kwnames) }
}

/// Calls the `Function` `callable` with vectorcall arguments.
///
/// Unlike `tp_call`, CPython does not check the recursion limit before a
/// vectorcall, so this does, with the same message.
///
/// # Safety
///
/// The arguments are those of a vectorcall of a `Function`.
unsafe fn call_vector(
    py: Python<'_>,
    callable: *mut ffi::PyObject,
    args: *const *mut ffi::PyObject,
    nargsf: ffi::Py_ssize_t,
    kwnames: *mut ffi::PyObject,
) -> PyResult<*mut ffi::PyObject> {
    let _guard = crate::RecursionGuard::enter(py)?;
    // SAFETY: `callable` is a `Function`, the type that has this slot.
    let function = unsafe { Borrowed::from_ptr(py, callable).cast_unchecked::<Function>() };
    // SAFETY: `kwnames` is NULL or a tuple of strings that outlives the call.
    let kwnames = unsafe { Borrowed::from_ptr_or_opt(py, kwnames) }
        .map(|names| unsafe { names.cast_unchecked::<PyTuple>() });
    let kwnames = kwnames.as_deref().map_or(&[][..], |names| names.as_slice());
    let nargs = nargsf as usize & !ffi::PY_VECTORCALL_ARGUMENTS_OFFSET;
    let total = nargs + kwnames.len();
    let all: &[Bound<'_, PyAny>] = if total == 0 {
        &[]
    } else {
        // SAFETY: `args` holds `total` object pointers, borrowed for the
        // duration of the call, and `Bound<PyAny>` has the layout of a
        // pointer (which `PyTuple::as_slice` relies on too).
        unsafe { std::slice::from_raw_parts(args.cast(), total) }
    };
    let (args, kwvalues) = all.split_at(nargs);
    let result = function.get().call(py, args, kwnames, kwvalues)?;
    Ok(result.into_ptr())
}

fn string_attribute(value: Option<&Bound<'_, PyAny>>, attr: &str) -> PyResult<Py<PyString>> {
    match value.map(|v| v.cast::<PyString>()) {
        Some(Ok(s)) => Ok(s.clone().unbind()),
        _ => Err(PyTypeError::new_err(format!(
            "{attr} must be set to a string object"
        ))),
    }
}

fn missing(qualname: &str, kind: &str, names: &[&str]) -> PyErr {
    PyTypeError::new_err(format!(
        "{qualname}() missing {} required {kind} argument{}: {}",
        names.len(),
        if names.len() == 1 { "" } else { "s" },
        join_names(names)
    ))
}

/// Formats names like CPython: `'a'`, `'a' and 'b'`, `'a', 'b', and 'c'`.
fn join_names(names: &[&str]) -> String {
    let quoted: Vec<String> = names.iter().map(|name| format!("'{name}'")).collect();
    match quoted.as_slice() {
        [] => String::new(),
        [only] => only.clone(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// The "Did you mean" suggestion CPython adds to an unexpected keyword error.
fn suggestion(py: Python<'_>, candidates: &[&str], keyword: &str) -> Option<String> {
    if candidates.is_empty() {
        return None;
    }
    let suggestions = py.import("_suggestions").ok()?;
    let found = suggestions
        .call_method1("_generate_suggestions", (candidates.to_vec(), keyword))
        .ok()?;
    if found.is_none() {
        return None;
    }
    found.extract().ok()
}

/// The value of a captured variable that is never reassigned after the
/// capture: the nested function receives the value instead of the cell.
#[doc(hidden)]
pub fn cell_contents<'py>(
    cell: &Bound<'py, PyAny>,
    name: &'static str,
) -> PyResult<Bound<'py, PyAny>> {
    crate::Cell::free(cell, name).get()
}

/// A new cell holding `value`, for a captured variable that is never
/// reassigned after the capture.
#[doc(hidden)]
pub fn capture<'py>(value: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    // SAFETY: valid object pointer; `PyCell_New` returns a new reference.
    unsafe { Bound::from_owned_ptr_or_err(value.py(), ffi::PyCell_New(value.as_ptr())) }
}

/// `*args` and `**kwargs` arrive as `tuple` and `dict` objects; this gives
/// them their concrete type for the Rust parameter.
#[doc(hidden)]
pub fn as_tuple<'a, 'py>(value: &'a Bound<'py, PyAny>) -> &'a Bound<'py, PyTuple> {
    value.cast().expect("`Function` passes `*args` as a tuple")
}

#[doc(hidden)]
pub fn as_dict<'a, 'py>(value: &'a Bound<'py, PyAny>) -> &'a Bound<'py, PyDict> {
    value
        .cast()
        .expect("`Function` passes `**kwargs` as a dict")
}

unsafe extern "C" {
    /// `types.MethodType(function, instance)`.
    fn PyMethod_New(
        function: *mut ffi::PyObject,
        instance: *mut ffi::PyObject,
    ) -> *mut ffi::PyObject;
}

/// The `__annotate__` of a class body (`__annotate_func__`), evaluating the
/// annotations in the class dict first, then in the module globals.
pub(crate) fn class_annotate<'py>(
    module: &Module<'py>,
    ns: &crate::Namespace<'py>,
    entries: &'static [(&'static str, &'static str)],
) -> PyResult<Bound<'py, Annotate>> {
    let annotate = Annotate {
        globals: module.globals().clone().unbind(),
        owner: std::sync::OnceLock::new(),
        classdict: Some(ns.classdict().clone().unbind()),
        qualname: format!("{}.__annotate__", ns.qualname()),
        entries,
    };
    Bound::new(module.py(), annotate)
}

/// Records the class that a class's `__annotate__` belongs to.
pub(crate) fn set_annotate_owner(annotate: &Bound<'_, Annotate>, owner: &Bound<'_, PyAny>) {
    let _ = annotate.get().owner.set(owner.clone().unbind());
}

const FORMAT_VALUE: i32 = 1;
const FORMAT_FORWARDREF: i32 = 3;
const FORMAT_STRING: i32 = 4;

/// `__annotate__` of a transpiled function (PEP 649): annotations are kept as
/// Python source and evaluated in the module globals when asked for.
#[pyclass(frozen, name = "annotate", module = "steelix_runtime")]
pub(crate) struct Annotate {
    globals: Py<PyDict>,
    /// The class whose annotations these are, once it exists.
    owner: std::sync::OnceLock<Py<PyAny>>,
    /// The `__classdict__` cell of the class body the annotations are in.
    classdict: Option<Py<PyAny>>,
    qualname: String,
    entries: &'static [(&'static str, &'static str)],
}

#[pymethods]
impl Annotate {
    /// None, like CPython's `__annotate__` functions, instead of the Rust
    /// documentation above.
    #[classattr]
    #[pyo3(name = "__doc__")]
    fn doc(py: Python<'_>) -> Py<PyAny> {
        py.None()
    }

    fn __call__<'py>(&self, py: Python<'py>, format: i32) -> PyResult<Bound<'py, PyDict>> {
        let result = PyDict::new(py);
        let globals = self.globals.bind(py);
        let locals = self
            .classdict
            .as_ref()
            .and_then(|cell| crate::class::classdict_value(cell.bind(py)));
        let locals = locals.as_ref();
        for &(name, source) in self.entries {
            let name = python_name(name);
            let value = match format {
                FORMAT_VALUE => eval(globals, locals, source)?,
                FORMAT_FORWARDREF => match eval(globals, locals, source) {
                    Err(err) if err.is_instance_of::<PyNameError>(py) => {
                        let options = match self.owner.get() {
                            // What `annotationlib` gives the annotations of a class.
                            Some(owner) => {
                                [("owner", owner.clone_ref(py).into_bound(py))].into_py_dict(py)?
                            }
                            None => [("module", globals.get_item("__name__")?)].into_py_dict(py)?,
                        };
                        if self.owner.get().is_some() {
                            options.set_item("is_class", true)?;
                        }
                        py.import("annotationlib")?
                            .getattr("ForwardRef")?
                            .call((source,), Some(&options))?
                    }
                    other => other?,
                },
                FORMAT_STRING => string_form(globals, source)?,
                // Including `VALUE_WITH_FAKE_GLOBALS`, which needs a code object.
                _ => {
                    return Err(PyNotImplementedError::new_err(()));
                }
            };
            result.set_item(name, value)?;
        }
        Ok(result)
    }

    fn __repr__(slf: &Bound<'_, Self>) -> String {
        let qualname = &slf.get().qualname;
        format!("<function {qualname} at {:#x}>", slf.as_ptr() as usize)
    }

    #[getter]
    fn __name__(&self) -> &'static str {
        "__annotate__"
    }

    #[getter]
    fn __qualname__(&self) -> &str {
        &self.qualname
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.globals)?;
        visit.call(&self.classdict)?;
        visit.call(self.owner.get())
    }
}

use pyo3::types::IntoPyDict;

fn eval<'py>(
    globals: &Bound<'py, PyDict>,
    locals: Option<&Bound<'py, PyAny>>,
    source: &str,
) -> PyResult<Bound<'py, PyAny>> {
    let code = CString::new(source)?;
    let py = globals.py();
    let Some(locals) = locals else {
        return py.eval(&code, Some(globals), None);
    };
    crate::builtin(py, "eval")?.call1((source, globals, locals))
}

/// The `STRING` format: the source, except that a string annotation is the
/// string itself.
fn string_form<'py>(globals: &Bound<'py, PyDict>, source: &str) -> PyResult<Bound<'py, PyAny>> {
    let py = globals.py();
    let literal = py.import("ast")?.call_method1("literal_eval", (source,));
    match literal {
        Ok(value) if value.is_instance_of::<PyString>() => Ok(value),
        _ => Ok(PyString::new(py, source).into_any()),
    }
}
