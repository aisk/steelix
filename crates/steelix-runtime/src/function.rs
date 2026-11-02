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
    /// Annotations as Python source, evaluated lazily like PEP 649.
    pub annotations: &'static [(&'static str, &'static str)],
    pub body: Body,
}

/// The writable attributes, behind a lock because the class is frozen. The
/// lock is never held while running Python code.
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
    /// The name the `def` gave it, which `Module::default` uses to tell the
    /// function a direct call was compiled against from a replacement.
    def_name: &'static str,
    signature: &'static Signature,
    body: Body,
    globals: Py<PyDict>,
    closure: Option<Py<PyTuple>>,
    meta: Mutex<Meta>,
}

impl Function {
    /// Creates a function the way executing a `def` statement (or evaluating
    /// a `lambda`) does.
    pub fn create<'py>(module: &Module<'py>, def: FunctionDef<'py>) -> PyResult<Bound<'py, PyAny>> {
        let py = module.py();
        let globals = module.globals();
        let name = match (def.name, def.qualname) {
            (Some(name), _) => name,
            (None, Some(qualname)) => qualname.rsplit('.').next().unwrap_or(qualname),
            (None, None) => python_name(def.rust_name),
        };
        let qualname = def.qualname.unwrap_or(name);
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
                qualname: format!("{qualname}.__annotate__"),
                entries: def.annotations,
            };
            Some(Bound::new(py, annotate)?.into_any().unbind())
        };
        let meta = Meta {
            name: PyString::new(py, name).unbind(),
            qualname: PyString::new(py, qualname).unbind(),
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
            def_name: name,
            signature: def.signature,
            body: def.body,
            globals: globals.clone().unbind(),
            closure: closure.map(Bound::unbind),
            meta: Mutex::new(meta),
        };
        Ok(Bound::new(py, function)?.into_any())
    }

    fn meta(&self) -> std::sync::MutexGuard<'_, Meta> {
        // A panic while holding the lock can only come from a bug here; the
        // data is still consistent, so keep going.
        self.meta
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Whether this is the function that `def name` created in `globals`.
    pub(crate) fn is_def(&self, name: &str, globals: &Bound<'_, PyDict>) -> bool {
        self.def_name == name && self.globals.bind(globals.py()).is(globals)
    }

    fn qualname(&self, py: Python<'_>) -> String {
        self.meta().qualname.bind(py).to_string()
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

    /// Binds call arguments to parameters like CPython's `initialize_locals`,
    /// raising the same `TypeError`s.
    fn bind<'py>(
        &self,
        py: Python<'py>,
        args: &Bound<'py, PyTuple>,
        kwargs: Option<&Bound<'py, PyDict>>,
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
        for (slot, arg) in slots.iter_mut().zip(args.iter()).take(sig.positional) {
            *slot = Some(arg);
        }
        if sig.varargs {
            let extra = args.get_slice(sig.positional.min(given), given);
            slots[sig.positional] = Some(extra.into_any());
        }
        let varkw = if sig.varkw {
            let dict = PyDict::new(py);
            slots[sig.names.len() - 1] = Some(dict.clone().into_any());
            Some(dict)
        } else {
            None
        };

        if let Some(kwargs) = kwargs {
            for (key, value) in kwargs.iter() {
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
                    Some(index) => slots[index] = Some(value),
                    None => match &varkw {
                        Some(dict) => dict.set_item(key, value)?,
                        None => return Err(self.unexpected_keyword(py, kwargs, &keyword)),
                    },
                }
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
                    .map(|d| d.get_item(sig.name(i)))
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
        kwargs: &Bound<'_, PyDict>,
        keyword: &str,
    ) -> PyErr {
        let sig = self.signature;
        let qualname = self.qualname(py);
        // Positional-only parameters passed by keyword get their own message.
        let posonly: Vec<&str> = (0..sig.posonly)
            .map(|i| sig.name(i))
            .filter(|name| kwargs.contains(name).unwrap_or(false))
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
        let this = slf.get();
        let bound = this.bind(py, args, kwargs)?;
        let closure = match &this.closure {
            Some(closure) => closure.bind(py).iter().collect(),
            None => Vec::new(),
        };
        let module = Module::from_globals(this.globals.bind(py).clone());
        (this.body)(&module, &closure, &bound)
    }

    fn __get__<'py>(
        slf: &Bound<'py, Self>,
        instance: Option<&Bound<'py, PyAny>>,
        _owner: Option<&Bound<'py, PyAny>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        match instance {
            Some(instance) if !instance.is_none() => {
                let method_type = slf.py().import("types")?.getattr("MethodType")?;
                method_type.call1((slf, instance))
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
        self.meta().name = string_attribute(value, "__name__")?;
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
        self.meta().qualname = string_attribute(value, "__qualname__")?;
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
        self.meta().doc = value.map_or_else(|| py.None(), |v| v.clone().unbind());
    }

    #[getter]
    fn __module__(&self, py: Python<'_>) -> Py<PyAny> {
        self.meta().module.clone_ref(py)
    }

    #[setter(__module__)]
    fn set_module(&self, py: Python<'_>, value: &Bound<'_, PyAny>) {
        // Deleting these attributes raises `AttributeError` before getting here.
        let value = Some(value);
        self.meta().module = value.map_or_else(|| py.None(), |v| v.clone().unbind());
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
        self.meta().defaults = value;
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
        self.meta().kwdefaults = value;
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
        self.meta().annotations = Some(annotations.clone().unbind());
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
        let mut meta = self.meta();
        meta.annotations = value;
        meta.annotate = None;
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

        let mut meta = self.meta();
        if value.is_none() {
            meta.annotate = None;
        } else if value.is_callable() {
            meta.annotate = Some(value.clone().unbind());
            meta.annotations = None;
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
                self.meta().type_params = tuple.clone().unbind();
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
        let mut meta = self.meta();
        meta.defaults = None;
        meta.kwdefaults = None;
        meta.annotate = None;
        meta.annotations = None;
    }
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

/// Binds a function created by `function!` to its name in the module.
#[doc(hidden)]
pub fn bind_global(module: &Module<'_>, function: Bound<'_, PyAny>) -> PyResult<()> {
    let name = function.getattr("__name__")?;
    module.globals().set_item(name, function)
}

const FORMAT_VALUE: i32 = 1;
const FORMAT_FORWARDREF: i32 = 3;
const FORMAT_STRING: i32 = 4;

/// `__annotate__` of a transpiled function (PEP 649): annotations are kept as
/// Python source and evaluated in the module globals when asked for.
#[pyclass(frozen, name = "annotate", module = "steelix_runtime")]
struct Annotate {
    globals: Py<PyDict>,
    qualname: String,
    entries: &'static [(&'static str, &'static str)],
}

#[pymethods]
impl Annotate {
    fn __call__<'py>(&self, py: Python<'py>, format: i32) -> PyResult<Bound<'py, PyDict>> {
        let result = PyDict::new(py);
        let globals = self.globals.bind(py);
        for &(name, source) in self.entries {
            let name = python_name(name);
            let value = match format {
                FORMAT_VALUE => eval(globals, source)?,
                FORMAT_FORWARDREF => match eval(globals, source) {
                    Err(err) if err.is_instance_of::<PyNameError>(py) => {
                        py.import("annotationlib")?.getattr("ForwardRef")?.call(
                            (source,),
                            Some(&[("module", globals.get_item("__name__")?)].into_py_dict(py)?),
                        )?
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

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.globals)
    }
}

use pyo3::types::IntoPyDict;

fn eval<'py>(globals: &Bound<'py, PyDict>, source: &str) -> PyResult<Bound<'py, PyAny>> {
    let code = CString::new(source)?;
    globals.py().eval(&code, Some(globals), None)
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
