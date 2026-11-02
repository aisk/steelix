//! Class statements (design §10).
//!
//! A transpiled `class` statement runs exactly like CPython's
//! `builtins.__build_class__`: the bases are resolved (`__mro_entries__`), the
//! metaclass is chosen, `__prepare__` makes the namespace, the class body runs
//! in it, and the metaclass creates the class. The body is a Rust function
//! that reads and writes names through a [`Namespace`]; methods are ordinary
//! [`Function`](crate::Function) objects stored in it.
//!
//! ```ignore
//! m.set_global("Point", rt::class!(m, "Point"(Base), point::body, line = 3)?)?;
//! ```

use std::cell::{OnceCell, RefCell};

use pyo3::exceptions::{PyKeyError, PyNameError, PyRuntimeError, PyTypeError};
use pyo3::ffi;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyString, PyTuple, PyType};
use pyo3::{PyTraverseError, PyVisit};

use crate::{Cell, Module};

/// The body of a class statement: the Rust function generated for it.
pub type ClassBody = for<'py> fn(&Module<'py>, &Namespace<'py>) -> PyResult<()>;

/// What `class!` knows about a class statement when it is transpiled.
pub struct ClassDef {
    pub name: &'static str,
    /// `None` for a module level class, whose qualified name is its name.
    pub qualname: Option<&'static str>,
    /// The line of the first decorator, or of `class`: `__firstlineno__`.
    pub first_line: u32,
    pub body: ClassBody,
    /// `__static_attributes__`: the `self.x` its methods assign, sorted.
    pub attributes: &'static [&'static str],
    /// Annotations of the class body as Python source, evaluated lazily by
    /// the class's `__annotate__` (PEP 649).
    pub annotations: &'static [(&'static str, &'static str)],
}

impl ClassDef {
    fn qualname(&self) -> &'static str {
        self.qualname.unwrap_or(self.name)
    }
}

/// The namespace a class body runs in: the mapping `__prepare__` returned.
///
/// Names in a class body are looked up like `LOAD_NAME` does, in the
/// namespace first, then in the module's globals and builtins:
///
/// ```ignore
/// ns.set("origin", ns.get("Point")?.call0()?)?;   // origin = Point()
/// ```
pub struct Namespace<'py> {
    module: Module<'py>,
    mapping: Bound<'py, PyAny>,
    qualname: &'static str,
    class_cell: OnceCell<Bound<'py, PyAny>>,
    classdict: OnceCell<Bound<'py, PyAny>>,
    /// `__class__` and `__classdict__` cells of code kept as Python, which
    /// get the values of the class's own cells once the class exists.
    kept_cells: RefCell<Vec<(&'static str, Bound<'py, PyAny>)>>,
}

impl<'py> Namespace<'py> {
    fn new(module: Module<'py>, mapping: Bound<'py, PyAny>, qualname: &'static str) -> Self {
        Self {
            module,
            mapping,
            qualname,
            class_cell: OnceCell::new(),
            classdict: OnceCell::new(),
            kept_cells: RefCell::new(Vec::new()),
        }
    }

    pub fn py(&self) -> Python<'py> {
        self.mapping.py()
    }

    /// The module the class is defined in.
    pub fn module(&self) -> &Module<'py> {
        &self.module
    }

    /// The namespace object itself, what `locals()` returns in a class body.
    pub fn mapping(&self) -> &Bound<'py, PyAny> {
        &self.mapping
    }

    pub(crate) fn qualname(&self) -> &'static str {
        self.qualname
    }

    /// Reads a name like `LOAD_NAME`: the namespace, then the module's
    /// globals, then builtins, raising `NameError` if none has it.
    pub fn get(&self, name: &'static str) -> PyResult<Bound<'py, PyAny>> {
        let py = self.py();
        let key = crate::interned(py, name);
        if let Ok(dict) = self.mapping.cast_exact::<PyDict>() {
            if let Some(value) = dict.get_item(&key)? {
                return Ok(value);
            }
        } else {
            match self.mapping.get_item(&key) {
                Ok(value) => return Ok(value),
                Err(err) if err.is_instance_of::<PyKeyError>(py) => {}
                Err(err) => return Err(err),
            }
        }
        self.module.global(name)
    }

    /// Binds a name in the class namespace (`STORE_NAME`).
    pub fn set(&self, name: &str, value: impl IntoPyObject<'py>) -> PyResult<()> {
        self.mapping.set_item(name, value)
    }

    /// `del name` in a class body, raising `NameError` if it is not bound.
    pub fn del(&self, name: &str) -> PyResult<()> {
        match self.mapping.del_item(name) {
            Err(err) if err.is_instance_of::<PyKeyError>(self.py()) => Err(crate::raised(
                PyNameError::new_err(format!("name '{name}' is not defined")),
            )),
            other => other,
        }
    }

    /// The implicit `del name` at the end of `except .. as name`, which
    /// CPython compiles as `name = None; del name`.
    pub fn unbind(&self, name: &str) -> PyResult<()> {
        self.set(name, self.py().None())?;
        self.del(name)
    }

    /// The `__class__` cell that methods using `super()` or `__class__`
    /// capture. The class is stored in it once it is created.
    pub fn class_cell(&self) -> Cell<'py> {
        let cell = self.class_cell.get_or_init(|| empty_cell(self.py()));
        Cell::free(cell, "__class__")
    }

    /// The `__classdict__` cell: the class's dict once it exists, in which
    /// the annotations of the class and its methods look names up first.
    pub(crate) fn classdict(&self) -> &Bound<'py, PyAny> {
        self.classdict.get_or_init(|| empty_cell(self.py()))
    }

    /// `SETUP_ANNOTATIONS`: with `from __future__ import annotations`, a
    /// class body with annotations starts by creating `__annotations__`.
    pub fn setup_annotations(&self) -> PyResult<()> {
        let py = self.py();
        let key = crate::interned(py, "__annotations__");
        let present = if let Ok(dict) = self.mapping.cast_exact::<PyDict>() {
            dict.contains(&key)?
        } else {
            match self.mapping.get_item(&key) {
                Ok(_) => true,
                Err(err) if err.is_instance_of::<PyKeyError>(py) => false,
                Err(err) => return Err(err),
            }
        };
        if !present {
            self.mapping.set_item(key, PyDict::new(py))?;
        }
        Ok(())
    }

    /// `name: annotation` with `from __future__ import annotations`: the
    /// annotation string goes into `__annotations__`.
    pub fn annotate(&self, name: &str, annotation: &str) -> PyResult<()> {
        self.get("__annotations__")?.set_item(name, annotation)
    }

    /// After code kept as Python ran in the namespace: takes back the cells
    /// it stored there, which the class fills like its own.
    pub(crate) fn adopt_kept_cells(&self) -> PyResult<()> {
        for (key, own) in [("__classcell__", "class"), ("__classdictcell__", "dict")] {
            let Some(cell) = self.pop(key)? else { continue };
            if own == "class" {
                self.class_cell();
            } else {
                self.classdict();
            }
            self.kept_cells.borrow_mut().push((own, cell));
        }
        Ok(())
    }

    /// Removes and returns `key` if the namespace has it.
    pub(crate) fn pop(&self, key: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
        match self.mapping.get_item(key) {
            Ok(value) => {
                self.mapping.del_item(key)?;
                Ok(Some(value))
            }
            Err(err) if err.is_instance_of::<PyKeyError>(self.py()) => Ok(None),
            Err(err) => Err(err),
        }
    }

    /// Like `pop`, without removing it.
    pub(crate) fn peek(&self, key: &str) -> PyResult<Option<Bound<'py, PyAny>>> {
        match self.mapping.get_item(key) {
            Ok(value) => Ok(Some(value)),
            Err(err) if err.is_instance_of::<PyKeyError>(self.py()) => Ok(None),
            Err(err) => Err(err),
        }
    }
}

fn empty_cell(py: Python<'_>) -> Bound<'_, PyAny> {
    // SAFETY: `PyCell_New` accepts NULL and returns a new reference; it only
    // fails without memory, which aborts here like any allocation failure.
    unsafe {
        Bound::from_owned_ptr_or_err(py, ffi::PyCell_New(std::ptr::null_mut()))
            .expect("creating a cell")
    }
}

fn cell_get<'py>(cell: &Bound<'py, PyAny>) -> Option<Bound<'py, PyAny>> {
    // SAFETY: a cell object; `PyCell_Get` returns a new reference or NULL
    // (without an exception) when the cell is empty.
    unsafe { Bound::from_owned_ptr_or_opt(cell.py(), ffi::PyCell_Get(cell.as_ptr())) }
}

fn cell_set(cell: &Bound<'_, PyAny>, value: &Bound<'_, PyAny>) -> PyResult<()> {
    // SAFETY: a cell object and a valid value; `PyCell_Set` takes its own reference.
    if unsafe { ffi::PyCell_Set(cell.as_ptr(), value.as_ptr()) } < 0 {
        return Err(PyErr::fetch(cell.py()));
    }
    Ok(())
}

/// The callable that `class!` calls with the bases and keywords of the class
/// statement, like the `__build_class__` call CPython compiles it to. It is
/// named like it, so that argument errors read the same.
#[pyclass(frozen, name = "__build_class__", module = "builtins")]
pub struct BuildClass {
    def: &'static ClassDef,
    globals: Py<PyDict>,
}

/// The builder of the class statement `def` in `module`. Use it through
/// [`class!`](crate::class).
#[doc(hidden)]
pub fn builder<'py>(module: &Module<'py>, def: &'static ClassDef) -> PyResult<Bound<'py, PyAny>> {
    let builder = BuildClass {
        def,
        globals: module.globals().clone().unbind(),
    };
    Ok(Bound::new(module.py(), builder)?.into_any())
}

#[pymethods]
impl BuildClass {
    #[getter]
    fn __qualname__(&self) -> &'static str {
        "__build_class__"
    }

    #[pyo3(signature = (*args, **kwargs))]
    fn __call__<'py>(
        &self,
        args: &Bound<'py, PyTuple>,
        kwargs: Option<&Bound<'py, PyDict>>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let module = Module::from_globals(self.globals.bind(args.py()).clone());
        // The first argument is the name, the others are the bases.
        let bases = args.get_slice(1, args.len());
        build_class(&module, self.def, &bases, kwargs)
    }

    fn __traverse__(&self, visit: PyVisit<'_>) -> Result<(), PyTraverseError> {
        visit.call(&self.globals)
    }
}

/// `builtins.__build_class__` for a transpiled class body.
fn build_class<'py>(
    module: &Module<'py>,
    def: &'static ClassDef,
    orig_bases: &Bound<'py, PyTuple>,
    kwargs: Option<&Bound<'py, PyDict>>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = module.py();
    let name = PyString::new(py, def.name);
    let bases = update_bases(orig_bases)?;
    let kwargs = kwargs.map(|k| k.copy()).transpose()?;
    let explicit = match &kwargs {
        Some(kwargs) => {
            let meta = kwargs.get_item("metaclass")?;
            if meta.is_some() {
                kwargs.del_item("metaclass")?;
            }
            meta
        }
        None => None,
    };
    let (mut meta, is_class) = match explicit {
        Some(meta) => {
            let is_class = meta.is_instance_of::<PyType>();
            (meta, is_class)
        }
        None if bases.is_empty() => (py.get_type::<PyType>().into_any(), true),
        None => (bases.get_item(0)?.get_type().into_any(), true),
    };
    if is_class {
        meta = calculate_metaclass(meta.cast_into::<PyType>()?, &bases)?.into_any();
    }
    let kwargs = kwargs.filter(|k| !k.is_empty());
    let mapping = match meta.getattr_opt("__prepare__")? {
        Some(prepare) => prepare.call((&name, &bases), kwargs.as_ref())?,
        None => PyDict::new(py).into_any(),
    };
    // SAFETY: a valid object pointer.
    if unsafe { ffi::PyMapping_Check(mapping.as_ptr()) } == 0 {
        let meta_name = if is_class {
            meta.cast::<PyType>()?.name()?.to_string()
        } else {
            "<metaclass>".to_owned()
        };
        return Err(crate::raised(PyTypeError::new_err(format!(
            "{meta_name}.__prepare__() must return a mapping, not {}",
            mapping.get_type().name()?
        ))));
    }

    let ns = Namespace::new(module.clone(), mapping, def.qualname());
    // What CPython's code for the class body does before and after the body.
    ns.set("__module__", ns.get("__name__")?)?;
    ns.set("__qualname__", def.qualname())?;
    ns.set("__firstlineno__", def.first_line)?;
    (def.body)(module, &ns)?;
    let annotate = if def.annotations.is_empty() {
        None
    } else {
        let annotate = crate::function::class_annotate(module, &ns, def.annotations)?;
        ns.set("__annotate_func__", &annotate)?;
        Some(annotate)
    };
    ns.set("__static_attributes__", PyTuple::new(py, def.attributes)?)?;
    if let Some(classdict) = ns.classdict.get() {
        ns.set("__classdictcell__", classdict)?;
    }
    if let Some(cell) = ns.class_cell.get() {
        ns.set("__classcell__", cell)?;
    }

    if !bases.is(orig_bases) {
        ns.set("__orig_bases__", orig_bases)?;
    }
    let class = meta.call((&name, &bases, &ns.mapping), kwargs.as_ref())?;
    if class.is_instance_of::<PyType>() {
        if let Some(cell) = ns.class_cell.get() {
            check_class_cell(cell, &name, &class)?;
        }
        fill_kept_cells(&ns, &class)?;
        if let Some(annotate) = &annotate {
            crate::function::set_annotate_owner(annotate, &class);
        }
        wrap_special_methods(&class)?;
    }
    Ok(class)
}

/// `__mro_entries__`: bases that are not classes may stand for others.
fn update_bases<'py>(bases: &Bound<'py, PyTuple>) -> PyResult<Bound<'py, PyTuple>> {
    let py = bases.py();
    let mut updated: Option<Vec<Bound<'py, PyAny>>> = None;
    for (i, base) in bases.iter().enumerate() {
        let entries = if base.is_instance_of::<PyType>() {
            None
        } else {
            base.getattr_opt("__mro_entries__")?
        };
        let Some(entries) = entries else {
            if let Some(updated) = &mut updated {
                updated.push(base);
            }
            continue;
        };
        let new_bases = entries.call1((bases,))?;
        let Ok(new_bases) = new_bases.cast_into::<PyTuple>() else {
            return Err(crate::raised(PyTypeError::new_err(
                "__mro_entries__ must return a tuple",
            )));
        };
        let updated = updated.get_or_insert_with(|| bases.iter().take(i).collect());
        updated.extend(new_bases.iter());
    }
    match updated {
        Some(updated) => PyTuple::new(py, updated),
        None => Ok(bases.clone()),
    }
}

/// `_PyType_CalculateMetaclass`: the most derived of the metaclass and the
/// types of the bases.
fn calculate_metaclass<'py>(
    meta: Bound<'py, PyType>,
    bases: &Bound<'py, PyTuple>,
) -> PyResult<Bound<'py, PyType>> {
    let mut winner = meta;
    for base in bases.iter() {
        let ty = base.get_type();
        // SAFETY: both are type objects.
        let (winner_sub, ty_sub) = unsafe {
            (
                ffi::PyType_IsSubtype(winner.as_type_ptr(), ty.as_type_ptr()) != 0,
                ffi::PyType_IsSubtype(ty.as_type_ptr(), winner.as_type_ptr()) != 0,
            )
        };
        if winner_sub {
            continue;
        }
        if ty_sub {
            winner = ty;
            continue;
        }
        return Err(crate::raised(PyTypeError::new_err(
            "metaclass conflict: the metaclass of a derived class must be a (non-strict) \
             subclass of the metaclasses of all its bases",
        )));
    }
    Ok(winner)
}

fn check_class_cell<'py>(
    cell: &Bound<'py, PyAny>,
    name: &Bound<'py, PyString>,
    class: &Bound<'py, PyAny>,
) -> PyResult<()> {
    match cell_get(cell) {
        Some(value) if value.is(class) => Ok(()),
        Some(value) => Err(crate::raised(PyTypeError::new_err(format!(
            "__class__ set to {} defining {} as {}",
            value.repr()?,
            name.repr()?,
            class.repr()?
        )))),
        None => Err(crate::raised(PyRuntimeError::new_err(format!(
            "__class__ not set defining {} as {}. Was __classcell__ propagated to type.__new__?",
            name.repr()?,
            class.repr()?
        )))),
    }
}

/// The cells of methods kept as Python get what the class's own cells got
/// from `type.__new__`.
fn fill_kept_cells<'py>(ns: &Namespace<'py>, class: &Bound<'py, PyAny>) -> PyResult<()> {
    for (kind, cell) in ns.kept_cells.borrow().iter() {
        let own = if *kind == "class" {
            ns.class_cell.get()
        } else {
            ns.classdict.get()
        };
        let value = match own.and_then(cell_get) {
            Some(value) => value,
            None if *kind == "class" => class.clone(),
            None => continue,
        };
        cell_set(cell, &value)?;
    }
    Ok(())
}

/// `type.__new__` makes a `__new__` defined by a `def` a staticmethod, and
/// `__init_subclass__` and `__class_getitem__` classmethods, but only Python
/// function objects. The same for transpiled functions, afterwards.
fn wrap_special_methods(class: &Bound<'_, PyAny>) -> PyResult<()> {
    let py = class.py();
    let ty = class.cast::<PyType>()?;
    // SAFETY: a type object; `PyType_GetDict` returns a new reference.
    let dict = unsafe { Bound::from_owned_ptr(py, ffi::PyType_GetDict(ty.as_type_ptr())) };
    let dict = dict.cast_into::<PyDict>()?;
    let wrappers = [
        ("__new__", "staticmethod"),
        ("__init_subclass__", "classmethod"),
        ("__class_getitem__", "classmethod"),
    ];
    for (name, wrapper) in wrappers {
        let Some(value) = dict.get_item(name)? else {
            continue;
        };
        if value.is_instance_of::<crate::Function>() {
            let wrapped = crate::builtin(py, wrapper)?.call1((value,))?;
            py.get_type::<PyType>()
                .getattr("__setattr__")?
                .call1((class, name, wrapped))?;
        }
    }
    Ok(())
}

/// Zero-argument `super()` in a method: `super(__class__, first_argument)`,
/// with CPython's errors when the `__class__` cell is not usable.
pub fn super_<'py>(class: &Cell<'py>, first: &Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
    let py = first.py();
    let Some(class) = cell_get(&class.object()) else {
        return Err(crate::raised(PyRuntimeError::new_err(
            "super(): empty __class__ cell",
        )));
    };
    if !class.is_instance_of::<PyType>() {
        return Err(crate::raised(PyRuntimeError::new_err(format!(
            "super(): __class__ is not a type ({})",
            class.get_type().name()?
        ))));
    }
    py.get_type::<pyo3::types::PySuper>().call1((class, first))
}

/// Runs class body statements kept as Python source in the namespace.
///
/// The source is compiled as the body of the class, inside synthetic
/// `class` statements for the enclosing classes (on the lines before it), so
/// that private names are mangled and qualified names come out as in the
/// file, and the class body code runs with the namespace as its locals. The
/// bookkeeping such a body does on its own (`__module__`, `__qualname__`,
/// `__firstlineno__` and `__static_attributes__`) is undone; the cells it
/// creates are filled in when the class is created.
pub(crate) fn exec_python(
    ns: &Namespace<'_>,
    line: usize,
    source: &str,
    future_annotations: bool,
) -> PyResult<()> {
    let py = ns.py();
    let parts: Vec<&str> = ns.qualname().split('.').collect();
    let indent: String = source
        .chars()
        .take_while(|c| *c == ' ' || *c == '\t')
        .collect();
    let mut padded = "\n".repeat(line.saturating_sub(parts.len() + 1));
    for (depth, part) in parts.iter().enumerate() {
        let prefix: String = indent.chars().take(depth).collect();
        padded += &format!("{prefix}class {part}:\n");
    }
    padded += source;
    let code = crate::python::compile_source(ns.module(), padded, future_annotations)?;
    let mut body = code;
    for part in &parts {
        body = class_code(&body, part)?;
    }

    let saved: Vec<_> = ["__module__", "__qualname__", "__firstlineno__"]
        .into_iter()
        .map(|key| ns.peek(key).map(|value| (key, value)))
        .collect::<PyResult<_>>()?;
    let globals = ns.module().globals();
    // SAFETY: a code object, a dict and a mapping; the result is a new
    // reference or NULL.
    let result = unsafe {
        Bound::from_owned_ptr_or_err(
            py,
            ffi::PyEval_EvalCode(body.as_ptr(), globals.as_ptr(), ns.mapping().as_ptr()),
        )
    };
    for (key, value) in saved {
        match value {
            Some(value) => ns.set(key, value)?,
            None => {
                ns.pop(key)?;
            }
        }
    }
    ns.pop("__static_attributes__")?;
    ns.adopt_kept_cells()?;
    result.map(drop)
}

/// The code of the class `name` defined by `code`.
fn class_code<'py>(code: &Bound<'py, PyAny>, name: &str) -> PyResult<Bound<'py, PyAny>> {
    for constant in code.getattr("co_consts")?.try_iter()? {
        let constant = constant?;
        // SAFETY: a valid object pointer.
        let is_code = unsafe { ffi::PyCode_Check(constant.as_ptr()) } != 0;
        if is_code && constant.getattr("co_name")?.eq(name)? {
            return Ok(constant);
        }
    }
    Err(PyRuntimeError::new_err(format!(
        "steelix: no code for class {name} in kept source"
    )))
}

/// The value a `__classdict__` cell read by annotations has: the class dict.
pub(crate) fn classdict_value<'py>(cell: &Bound<'py, PyAny>) -> Option<Bound<'py, PyAny>> {
    cell_get(cell)
}

/// Creates a class like executing a `class` statement: evaluates the bases
/// and keywords (`metaclass=`, and those for `__init_subclass__`), runs the
/// body and returns the new class, without binding it to its name.
///
/// ```ignore
/// rt::class!(m, "Point"(), point::body, line = 3)?;
/// rt::class!(m, "Inner"(Base, metaclass = meta), inner::body, line = 9,
///     qualname = "Outer.Inner", attributes = ["x", "y"])?;
/// rt::class!(m, "Config"(), config::body, line = 5,
///     annotations = [("name", "str"), ("size", "int")])?;
/// ```
///
/// The bases and keywords are written like the arguments of [`call!`](crate::call),
/// `*bases` and `**keywords` included. `line` is the line of the first
/// decorator (or of `class`), `attributes` the `__static_attributes__`.
#[macro_export]
macro_rules! class {
    ($module:expr, $name:literal ($($args:tt)*), $body:path, line = $line:literal
        $(, qualname = $qualname:literal)?
        $(, attributes = [$($attr:literal),* $(,)?])?
        $(, annotations = [$(($ann_name:literal, $ann:literal)),* $(,)?])?
        $(,)?) => {{
        static CLASS: $crate::class::ClassDef = $crate::class::ClassDef {
            name: $name,
            qualname: $crate::class!(@opt $($qualname)?),
            first_line: $line,
            body: $body,
            attributes: &[$($($attr),*)?],
            annotations: &[$($(($ann_name, $ann)),*)?],
        };
        let module: &$crate::Module<'_> = $module;
        // Called like `__build_class__(body, name, *bases, **keywords)`:
        // the name is the first argument, so `*bases` is never alone.
        $crate::call!($crate::class::builder(module, &CLASS)?, $name, $($args)*)
    }};
    (@opt) => { ::std::option::Option::None };
    (@opt $value:literal) => { ::std::option::Option::Some($value) };
}
