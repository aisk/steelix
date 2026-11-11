use pyo3::exceptions::PyBaseException;
use pyo3::exceptions::PyGeneratorExit;
use pyo3::exceptions::PyImportError;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// json/encoder.py:40
/// Return a JSON representation of a Python string
///
///
#[rt::frame(m, "py_encode_basestring", 40, frame)]
pub fn py_encode_basestring<'py>(
    m: &rt::Module<'py>,
    s: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // json/encoder.py:44
    #[rt::frame(m, "py_encode_basestring.<locals>.replace", 44)]
    fn replace<'py>(
        m: &rt::Module<'py>,
        r#match: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        m.global("ESCAPE_DCT")?
            .get_item(r#match.method1("group", (0,))?)
    }
    let replace = rt::function!(frame, replace(r#match))?;
    rt::str(m.py(), "\"")
        .add(m.global("ESCAPE")?.method1("sub", (&replace, s))?)?
        .add("\"")
}

// json/encoder.py:52
/// Return an ASCII-only JSON representation of a Python string
///
///
#[rt::frame(m, "py_encode_basestring_ascii", 52, frame)]
pub fn py_encode_basestring_ascii<'py>(
    m: &rt::Module<'py>,
    s: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    // json/encoder.py:56
    #[rt::frame(m, "py_encode_basestring_ascii.<locals>.replace", 56, frame)]
    fn replace<'py>(
        m: &rt::Module<'py>,
        r#match: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let s = r#match.method1("group", (0,))?;
        let mut n: Bound<'py, PyAny>;
        let s1: Bound<'py, PyAny>;
        let s2: Bound<'py, PyAny>;
        match (|| m.global("ESCAPE_DCT")?.get_item(&s))() {
            Ok(value) => Ok(value),
            Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                let _caught = frame.catch(err);
                n = rt::builtins::ord(py, (&s,))?;
                if n.lt(65536)? {
                    rt::str(py, "\\u{0:04x}").method1("format", (&n,))
                } else {
                    // return '\\u%04x' % (n,)
                    // surrogate pair
                    n = rt::isub(&n, 65536)?;
                    s1 = rt::int(py, 55296).bitor(n.rshift(10)?.bitand(1023)?)?;
                    s2 = rt::int(py, 56320).bitor(n.bitand(1023)?)?;
                    rt::str(py, "\\u{0:04x}\\u{1:04x}").method1("format", (&s1, &s2))
                }
            }
            Err(err) => Err(err),
        }
    }
    let replace = rt::function!(frame, replace(r#match))?;
    rt::str(m.py(), "\"")
        .add(m.global("ESCAPE_ASCII")?.method1("sub", (&replace, s))?)?
        .add("\"")
}

// # HACK: hand-optimized bytecode; turn globals into locals
// json/encoder.py:268
#[rt::frame(m, "_make_iterencode", 268, frame)]
pub fn _make_iterencode<'py>(
    m: &rt::Module<'py>,
    markers: &Bound<'py, PyAny>,
    _default: &Bound<'py, PyAny>,
    _encoder: &Bound<'py, PyAny>,
    _indent: &Bound<'py, PyAny>,
    _floatstr: &Bound<'py, PyAny>,
    _key_separator: &Bound<'py, PyAny>,
    _item_separator: &Bound<'py, PyAny>,
    _sort_keys: &Bound<'py, PyAny>,
    _skipkeys: &Bound<'py, PyAny>,
    _one_shot: &Bound<'py, PyAny>,
    ValueError: &Bound<'py, PyAny>,
    dict: &Bound<'py, PyAny>,
    float: &Bound<'py, PyAny>,
    id: &Bound<'py, PyAny>,
    int: &Bound<'py, PyAny>,
    isinstance: &Bound<'py, PyAny>,
    list: &Bound<'py, PyAny>,
    str: &Bound<'py, PyAny>,
    tuple: &Bound<'py, PyAny>,
    _intstr: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    let _iterencode_list = rt::Cell::empty(py, "_iterencode_list")?;
    let _iterencode_dict = rt::Cell::empty(py, "_iterencode_dict")?;
    let _iterencode = rt::Cell::empty(py, "_iterencode")?;
    // json/encoder.py:283
    #[rt::frame(m, "_make_iterencode.<locals>._iterencode_list", 283, frame)]
    async fn _iterencode_list_<'py>(
        m: &rt::Module<'py>,
        markers: &Bound<'py, PyAny>,
        id: &Bound<'py, PyAny>,
        ValueError: &Bound<'py, PyAny>,
        _indent: &Bound<'py, PyAny>,
        _item_separator: &Bound<'py, PyAny>,
        isinstance: &Bound<'py, PyAny>,
        str: &Bound<'py, PyAny>,
        _encoder: &Bound<'py, PyAny>,
        int: &Bound<'py, PyAny>,
        _intstr: &Bound<'py, PyAny>,
        float: &Bound<'py, PyAny>,
        _floatstr: &Bound<'py, PyAny>,
        list: &Bound<'py, PyAny>,
        tuple: &Bound<'py, PyAny>,
        _iterencode_list: &rt::Cell<'py>,
        dict: &Bound<'py, PyAny>,
        _iterencode_dict: &rt::Cell<'py>,
        _iterencode: &rt::Cell<'py>,
        lst: &Bound<'py, PyAny>,
        _current_indent_level: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let mut _current_indent_level = _current_indent_level.clone();
        if !lst.is_truthy()? {
            rt::yield_(rt::str(py, "[]")).await?;
            return Ok(rt::none(py));
        }
        let mut markerid: Option<Bound<'py, PyAny>> = None;
        if !markers.is_none() {
            markerid = Some(id.call1((lst,))?);
            if markers.contains(rt::local(&markerid, "markerid")?)? {
                return Err(frame.raise(ValueError.call1(("Circular reference detected",))?));
            }
            markers.set_item(rt::local(&markerid, "markerid")?, lst)?;
        }
        let mut buf = rt::str(py, "[");
        let newline_indent: Bound<'py, PyAny>;
        let separator: Bound<'py, PyAny>;
        if !_indent.is_none() {
            _current_indent_level = rt::iadd(&_current_indent_level, 1)?;
            newline_indent = rt::str(py, "\n").add(_indent.mul(&_current_indent_level)?)?;
            separator = _item_separator.add(&newline_indent)?;
            buf = rt::iadd(&buf, &newline_indent)?;
        } else {
            newline_indent = rt::none(py);
            separator = _item_separator.clone();
        }
        for item in rt::iter(&rt::builtins::enumerate(py, (lst,))?)? {
            let [i, value] = rt::unpack(&item?)?;
            if i.is_truthy()? {
                buf = separator.clone();
            }
            let result: PyResult<()> = async {
                let chunks: Bound<'py, PyAny>;
                if isinstance.call1((&value, str))?.is_truthy()? {
                    rt::yield_(buf.add(_encoder.call1((&value,))?)?).await?;
                } else if value.is_none() {
                    rt::yield_(buf.add("null")?).await?;
                } else if value.is(rt::bool(py, true)) {
                    rt::yield_(buf.add("true")?).await?;
                } else if value.is(rt::bool(py, false)) {
                    rt::yield_(buf.add("false")?).await?;
                } else if isinstance.call1((&value, int))?.is_truthy()? {
                    // Subclasses of int/float may override __repr__, but we still
                    // want to encode them as integers/floats in JSON. One example
                    // within the standard library is IntEnum.
                    rt::yield_(buf.add(_intstr.call1((&value,))?)?).await?;
                } else if isinstance.call1((&value, float))?.is_truthy()? {
                    // see comment above for int
                    rt::yield_(buf.add(_floatstr.call1((&value,))?)?).await?;
                } else {
                    rt::yield_(&buf).await?;
                    if isinstance
                        .call1((&value, PyTuple::new(py, [list, tuple])?))?
                        .is_truthy()?
                    {
                        chunks = _iterencode_list
                            .get()?
                            .call1((&value, &_current_indent_level))?;
                    } else if isinstance.call1((&value, dict))?.is_truthy()? {
                        chunks = _iterencode_dict
                            .get()?
                            .call1((&value, &_current_indent_level))?;
                    } else {
                        chunks = _iterencode.get()?.call1((&value, &_current_indent_level))?;
                    }
                    rt::yield_from(&chunks).await?;
                }
                Ok(())
            }
            .await;
            match result {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                    let _caught = frame.catch(err);
                    return Err(frame.reraise());
                }
                Err(err) if err.is_instance_of::<PyBaseException>(py) => {
                    let exc = frame.catch(err);
                    exc.attr("add_note")?.call1((rt::fstring(
                        py,
                        (
                            "when serializing ",
                            rt::format(&lst.get_type().attr("__name__")?, "")?,
                            " item ",
                            rt::format(&i, "")?,
                        ),
                    )?,))?;
                    return Err(frame.reraise());
                }
                Err(err) => return Err(err),
            }
        }
        if !newline_indent.is_none() {
            _current_indent_level = rt::isub(&_current_indent_level, 1)?;
            let value2 = rt::str(py, "\n").add(_indent.mul(&_current_indent_level)?)?;
            rt::yield_(value2).await?;
        }
        rt::yield_(rt::str(py, "]")).await?;
        if !markers.is_none() {
            markers.del_item(rt::local(&markerid, "markerid")?)?;
        }
        Ok(rt::none(py))
    }
    _iterencode_list.set(rt::function!(
        frame,
        gen _iterencode_list_ as "_iterencode_list"[markers, id, ValueError, _indent, _item_separator, isinstance, str, _encoder, int, _intstr, float, _floatstr, list, tuple, &_iterencode_list, dict, &_iterencode_dict, &_iterencode](lst, _current_indent_level)
    )?)?;
    // json/encoder.py:342
    #[rt::frame(m, "_make_iterencode.<locals>._iterencode_dict", 342, frame)]
    async fn _iterencode_dict_<'py>(
        m: &rt::Module<'py>,
        markers: &Bound<'py, PyAny>,
        id: &Bound<'py, PyAny>,
        ValueError: &Bound<'py, PyAny>,
        _indent: &Bound<'py, PyAny>,
        _item_separator: &Bound<'py, PyAny>,
        _sort_keys: &Bound<'py, PyAny>,
        isinstance: &Bound<'py, PyAny>,
        str: &Bound<'py, PyAny>,
        float: &Bound<'py, PyAny>,
        _floatstr: &Bound<'py, PyAny>,
        int: &Bound<'py, PyAny>,
        _intstr: &Bound<'py, PyAny>,
        _skipkeys: &Bound<'py, PyAny>,
        _encoder: &Bound<'py, PyAny>,
        _key_separator: &Bound<'py, PyAny>,
        list: &Bound<'py, PyAny>,
        tuple: &Bound<'py, PyAny>,
        _iterencode_list: &rt::Cell<'py>,
        dict: &Bound<'py, PyAny>,
        _iterencode_dict: &rt::Cell<'py>,
        _iterencode: &rt::Cell<'py>,
        dct: &Bound<'py, PyAny>,
        _current_indent_level: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let mut _current_indent_level = _current_indent_level.clone();
        if !dct.is_truthy()? {
            rt::yield_(rt::str(py, "{}")).await?;
            return Ok(rt::none(py));
        }
        let mut markerid: Option<Bound<'py, PyAny>> = None;
        if !markers.is_none() {
            markerid = Some(id.call1((dct,))?);
            if markers.contains(rt::local(&markerid, "markerid")?)? {
                return Err(frame.raise(ValueError.call1(("Circular reference detected",))?));
            }
            markers.set_item(rt::local(&markerid, "markerid")?, dct)?;
        }
        rt::yield_(rt::str(py, "{")).await?;
        let newline_indent: Bound<'py, PyAny>;
        let item_separator = if !_indent.is_none() {
            _current_indent_level = rt::iadd(&_current_indent_level, 1)?;
            newline_indent = rt::str(py, "\n").add(_indent.mul(&_current_indent_level)?)?;
            _item_separator.add(&newline_indent)?
        } else {
            newline_indent = rt::none(py);
            _item_separator.clone()
        };
        let mut first = true;
        let items = if _sort_keys.is_truthy()? {
            rt::builtins::sorted(py, (dct.method0("items")?,))?
        } else {
            dct.method0("items")?
        };
        let mut key: Bound<'py, PyAny>;
        let mut value: Bound<'py, PyAny>;
        for item in rt::iter(&items)? {
            [key, value] = rt::unpack(&item?)?;
            if isinstance.call1((&key, str))?.is_truthy()? {
            } else {
                // JavaScript is weakly typed for these, so it makes sense to
                // also allow them.  Many encoders seem to do something like this.
                if isinstance.call1((&key, float))?.is_truthy()? {
                    // see comment for int/float in _make_iterencode
                    key = _floatstr.call1((&key,))?;
                } else if key.is(rt::bool(py, true)) {
                    key = rt::str(py, "true");
                } else if key.is(rt::bool(py, false)) {
                    key = rt::str(py, "false");
                } else if key.is_none() {
                    key = rt::str(py, "null");
                } else if isinstance.call1((&key, int))?.is_truthy()? {
                    // see comment for int/float in _make_iterencode
                    key = _intstr.call1((&key,))?;
                } else if _skipkeys.is_truthy()? {
                    continue;
                } else {
                    return Err(frame.raise(PyTypeError::new_err((rt::fstring(
                        py,
                        (
                            "keys must be str, int, float, bool or None, not ",
                            rt::format(&key.attr("__class__")?.attr("__name__")?, "")?,
                        ),
                    )?
                    .unbind(),))));
                }
            }
            if first {
                first = false;
                if !newline_indent.is_none() {
                    rt::yield_(&newline_indent).await?;
                }
            } else {
                rt::yield_(&item_separator).await?;
            }
            rt::yield_(_encoder.call1((&key,))?).await?;
            rt::yield_(_key_separator).await?;
            let result: PyResult<()> = async {
                let chunks: Bound<'py, PyAny>;
                if isinstance.call1((&value, str))?.is_truthy()? {
                    rt::yield_(_encoder.call1((&value,))?).await?;
                } else if value.is_none() {
                    rt::yield_(rt::str(py, "null")).await?;
                } else if value.is(rt::bool(py, true)) {
                    rt::yield_(rt::str(py, "true")).await?;
                } else if value.is(rt::bool(py, false)) {
                    rt::yield_(rt::str(py, "false")).await?;
                } else if isinstance.call1((&value, int))?.is_truthy()? {
                    // see comment for int/float in _make_iterencode
                    rt::yield_(_intstr.call1((&value,))?).await?;
                } else if isinstance.call1((&value, float))?.is_truthy()? {
                    // see comment for int/float in _make_iterencode
                    rt::yield_(_floatstr.call1((&value,))?).await?;
                } else {
                    if isinstance
                        .call1((&value, PyTuple::new(py, [list, tuple])?))?
                        .is_truthy()?
                    {
                        chunks = _iterencode_list
                            .get()?
                            .call1((&value, &_current_indent_level))?;
                    } else if isinstance.call1((&value, dict))?.is_truthy()? {
                        chunks = _iterencode_dict
                            .get()?
                            .call1((&value, &_current_indent_level))?;
                    } else {
                        chunks = _iterencode.get()?.call1((&value, &_current_indent_level))?;
                    }
                    rt::yield_from(&chunks).await?;
                }
                Ok(())
            }
            .await;
            match result {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                    let _caught = frame.catch(err);
                    return Err(frame.reraise());
                }
                Err(err) if err.is_instance_of::<PyBaseException>(py) => {
                    let exc = frame.catch(err);
                    exc.attr("add_note")?.call1((rt::fstring(
                        py,
                        (
                            "when serializing ",
                            rt::format(&dct.get_type().attr("__name__")?, "")?,
                            " item ",
                            key.repr()?,
                        ),
                    )?,))?;
                    return Err(frame.reraise());
                }
                Err(err) => return Err(err),
            }
        }
        if !first && !newline_indent.is_none() {
            _current_indent_level = rt::isub(&_current_indent_level, 1)?;
            let value2 = rt::str(py, "\n").add(_indent.mul(&_current_indent_level)?)?;
            rt::yield_(value2).await?;
        }
        rt::yield_(rt::str(py, "}")).await?;
        if !markers.is_none() {
            markers.del_item(rt::local(&markerid, "markerid")?)?;
        }
        Ok(rt::none(py))
    }
    _iterencode_dict.set(rt::function!(
        frame,
        gen _iterencode_dict_ as "_iterencode_dict"[markers, id, ValueError, _indent, _item_separator, _sort_keys, isinstance, str, float, _floatstr, int, _intstr, _skipkeys, _encoder, _key_separator, list, tuple, &_iterencode_list, dict, &_iterencode_dict, &_iterencode](dct, _current_indent_level)
    )?)?;
    // json/encoder.py:429
    #[rt::frame(m, "_make_iterencode.<locals>._iterencode", 429, frame)]
    async fn _iterencode_<'py>(
        m: &rt::Module<'py>,
        isinstance: &Bound<'py, PyAny>,
        str: &Bound<'py, PyAny>,
        _encoder: &Bound<'py, PyAny>,
        int: &Bound<'py, PyAny>,
        _intstr: &Bound<'py, PyAny>,
        float: &Bound<'py, PyAny>,
        _floatstr: &Bound<'py, PyAny>,
        list: &Bound<'py, PyAny>,
        tuple: &Bound<'py, PyAny>,
        _iterencode_list: &rt::Cell<'py>,
        dict: &Bound<'py, PyAny>,
        _iterencode_dict: &rt::Cell<'py>,
        markers: &Bound<'py, PyAny>,
        id: &Bound<'py, PyAny>,
        ValueError: &Bound<'py, PyAny>,
        _default: &Bound<'py, PyAny>,
        _iterencode: &rt::Cell<'py>,
        o: &Bound<'py, PyAny>,
        _current_indent_level: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let mut markerid: Option<Bound<'py, PyAny>> = None;
        let newobj: Bound<'py, PyAny>;
        if isinstance.call1((o, str))?.is_truthy()? {
            rt::yield_(_encoder.call1((o,))?).await?;
        } else if o.is_none() {
            rt::yield_(rt::str(py, "null")).await?;
        } else if o.is(rt::bool(py, true)) {
            rt::yield_(rt::str(py, "true")).await?;
        } else if o.is(rt::bool(py, false)) {
            rt::yield_(rt::str(py, "false")).await?;
        } else if isinstance.call1((o, int))?.is_truthy()? {
            // see comment for int/float in _make_iterencode
            rt::yield_(_intstr.call1((o,))?).await?;
        } else if isinstance.call1((o, float))?.is_truthy()? {
            // see comment for int/float in _make_iterencode
            rt::yield_(_floatstr.call1((o,))?).await?;
        } else if isinstance
            .call1((o, PyTuple::new(py, [list, tuple])?))?
            .is_truthy()?
        {
            let iterable = _iterencode_list.get()?.call1((o, _current_indent_level))?;
            rt::yield_from(iterable).await?;
        } else if isinstance.call1((o, dict))?.is_truthy()? {
            let iterable = _iterencode_dict.get()?.call1((o, _current_indent_level))?;
            rt::yield_from(iterable).await?;
        } else {
            if !markers.is_none() {
                markerid = Some(id.call1((o,))?);
                if markers.contains(rt::local(&markerid, "markerid")?)? {
                    return Err(frame.raise(ValueError.call1(("Circular reference detected",))?));
                }
                markers.set_item(rt::local(&markerid, "markerid")?, o)?;
            }
            newobj = _default.call1((o,))?;
            match async {
                let iterable = _iterencode.get()?.call1((&newobj, _current_indent_level))?;
                rt::yield_from(iterable).await
            }
            .await
            {
                Ok(_) => {}
                Err(err) if err.is_instance_of::<PyGeneratorExit>(py) => {
                    let _caught = frame.catch(err);
                    return Err(frame.reraise());
                }
                Err(err) if err.is_instance_of::<PyBaseException>(py) => {
                    let exc = frame.catch(err);
                    exc.attr("add_note")?.call1((rt::fstring(
                        py,
                        (
                            "when serializing ",
                            rt::format(&o.get_type().attr("__name__")?, "")?,
                            " object",
                        ),
                    )?,))?;
                    return Err(frame.reraise());
                }
                Err(err) => return Err(err),
            }
            if !markers.is_none() {
                markers.del_item(rt::local(&markerid, "markerid")?)?;
            }
        }
        Ok(rt::none(py))
    }
    _iterencode.set(rt::function!(
        frame,
        gen _iterencode_ as "_iterencode"[isinstance, str, _encoder, int, _intstr, float, _floatstr, list, tuple, &_iterencode_list, dict, &_iterencode_dict, markers, id, ValueError, _default, &_iterencode](o, _current_indent_level)
    )?)?;
    _iterencode.get()
}

/// Extensible JSON <https://json.org> encoder for Python data structures.
///
/// Supports the following objects and types by default:
///
/// +-------------------+---------------+
/// | Python            | JSON          |
/// +===================+===============+
/// | dict              | object        |
/// +-------------------+---------------+
/// | list, tuple       | array         |
/// +-------------------+---------------+
/// | str               | string        |
/// +-------------------+---------------+
/// | int, float        | number        |
/// +-------------------+---------------+
/// | True              | true          |
/// +-------------------+---------------+
/// | False             | false         |
/// +-------------------+---------------+
/// | None              | null          |
/// +-------------------+---------------+
///
/// To extend this to recognize other objects, subclass and implement a
/// ``.default()`` method with another method that returns a serializable
/// object for ``o`` if possible, otherwise it should call the superclass
/// implementation (to raise ``TypeError``).
///
pub mod json_encoder {
    use super::*;

    // json/encoder.py:77
    /// The body of `class JSONEncoder`, run in its namespace `ns`.
    #[rt::frame(ns, "JSONEncoder", 77)]
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        ns.set(
            "__doc__",
            "Extensible JSON <https://json.org> encoder for Python data structures.\n\
                \n\
                Supports the following objects and types by default:\n\
                \n\
                +-------------------+---------------+\n\
                | Python            | JSON          |\n\
                +===================+===============+\n\
                | dict              | object        |\n\
                +-------------------+---------------+\n\
                | list, tuple       | array         |\n\
                +-------------------+---------------+\n\
                | str               | string        |\n\
                +-------------------+---------------+\n\
                | int, float        | number        |\n\
                +-------------------+---------------+\n\
                | True              | true          |\n\
                +-------------------+---------------+\n\
                | False             | false         |\n\
                +-------------------+---------------+\n\
                | None              | null          |\n\
                +-------------------+---------------+\n\
                \n\
                To extend this to recognize other objects, subclass and implement a\n\
                ``.default()`` method with another method that returns a serializable\n\
                object for ``o`` if possible, otherwise it should call the superclass\n\
                implementation (to raise ``TypeError``).\n\
                \n",
        )?;
        ns.set("item_separator", ", ")?;
        ns.set("key_separator", ": ")?;
        rt::def!(
            ns,
            __init__(slf as "self", *, skipkeys = false, ensure_ascii = true, check_circular = true, allow_nan = true, sort_keys = false, indent = py.None(), separators = py.None(), default = py.None()),
            doc = "Constructor for JSONEncoder, with sensible defaults.\n\
                \n\
                If skipkeys is false, then it is a TypeError to attempt\n\
                encoding of keys that are not str, int, float, bool or None.\n\
                If skipkeys is True, such items are simply skipped.\n\
                \n\
                If ensure_ascii is true, the output is guaranteed to be str objects\n\
                with all incoming non-ASCII and non-printable characters escaped.\n\
                If ensure_ascii is false, the output can contain non-ASCII and\n\
                non-printable characters.\n\
                \n\
                If check_circular is true, then lists, dicts, and custom encoded\n\
                objects will be checked for circular references during encoding to\n\
                prevent an infinite recursion (which would cause an RecursionError).\n\
                Otherwise, no such check takes place.\n\
                \n\
                If allow_nan is true, then NaN, Infinity, and -Infinity will be\n\
                encoded as such.  This behavior is not JSON specification compliant,\n\
                but is consistent with most JavaScript based encoders and decoders.\n\
                Otherwise, it will be a ValueError to encode such floats.\n\
                \n\
                If sort_keys is true, then the output of dictionaries will be\n\
                sorted by key; this is useful for regression tests to ensure\n\
                that JSON serializations can be compared on a day-to-day basis.\n\
                \n\
                If indent is a non-negative integer, then JSON array\n\
                elements and object members will be pretty-printed with that\n\
                indent level.  An indent level of 0 will only insert newlines.\n\
                None is the most compact representation.\n\
                \n\
                If specified, separators should be an (item_separator,\n\
                key_separator) tuple.  The default is (', ', ': ') if *indent* is\n\
                ``None`` and (',', ': ') otherwise.  To get the most compact JSON\n\
                representation, you should specify (',', ':') to eliminate\n\
                whitespace.\n\
                \n\
                If specified, default is a function that gets called for objects\n\
                that can't otherwise be serialized.  It should return a JSON\n\
                encodable version of the object or raise a ``TypeError``.\n\
                \n"
        )?;
        rt::def!(
            ns,
            default(slf as "self", o),
            doc = "Implement this method in a subclass such that it returns\n\
                a serializable object for ``o``, or calls the base implementation\n\
                (to raise a ``TypeError``).\n\
                \n\
                For example, to support arbitrary iterators, you could\n\
                implement default like this::\n\
                \n\
                \x20   def default(self, o):\n\
                \x20       try:\n\
                \x20           iterable = iter(o)\n\
                \x20       except TypeError:\n\
                \x20           pass\n\
                \x20       else:\n\
                \x20           return list(iterable)\n\
                \x20       # Let the base class default method raise the TypeError\n\
                \x20       return super().default(o)\n\
                \n"
        )?;
        rt::def!(
            ns,
            encode(slf as "self", o),
            doc = "Return a JSON string representation of a Python data structure.\n\
                \n\
                >>> from json.encoder import JSONEncoder\n\
                >>> JSONEncoder().encode({\"foo\": [\"bar\", \"baz\"]})\n\
                '{\"foo\": [\"bar\", \"baz\"]}'\n\
                \n"
        )?;
        rt::def!(
            ns,
            iterencode(slf as "self", o, _one_shot = false),
            doc = "Encode the given object and yield each string\n\
                representation as available.\n\
                \n\
                For example::\n\
                \n\
                \x20   for chunk in JSONEncoder().iterencode(bigobject):\n\
                \x20       mysocket.write(chunk)\n\
                \n"
        )?;
        Ok(())
    }

    // json/encoder.py:108
    /// Constructor for JSONEncoder, with sensible defaults.
    ///
    /// If skipkeys is false, then it is a TypeError to attempt
    /// encoding of keys that are not str, int, float, bool or None.
    /// If skipkeys is True, such items are simply skipped.
    ///
    /// If ensure_ascii is true, the output is guaranteed to be str objects
    /// with all incoming non-ASCII and non-printable characters escaped.
    /// If ensure_ascii is false, the output can contain non-ASCII and
    /// non-printable characters.
    ///
    /// If check_circular is true, then lists, dicts, and custom encoded
    /// objects will be checked for circular references during encoding to
    /// prevent an infinite recursion (which would cause an RecursionError).
    /// Otherwise, no such check takes place.
    ///
    /// If allow_nan is true, then NaN, Infinity, and -Infinity will be
    /// encoded as such.  This behavior is not JSON specification compliant,
    /// but is consistent with most JavaScript based encoders and decoders.
    /// Otherwise, it will be a ValueError to encode such floats.
    ///
    /// If sort_keys is true, then the output of dictionaries will be
    /// sorted by key; this is useful for regression tests to ensure
    /// that JSON serializations can be compared on a day-to-day basis.
    ///
    /// If indent is a non-negative integer, then JSON array
    /// elements and object members will be pretty-printed with that
    /// indent level.  An indent level of 0 will only insert newlines.
    /// None is the most compact representation.
    ///
    /// If specified, separators should be an (item_separator,
    /// key_separator) tuple.  The default is (', ', ': ') if *indent* is
    /// ``None`` and (',', ': ') otherwise.  To get the most compact JSON
    /// representation, you should specify (',', ':') to eliminate
    /// whitespace.
    ///
    /// If specified, default is a function that gets called for objects
    /// that can't otherwise be serialized.  It should return a JSON
    /// encodable version of the object or raise a ``TypeError``.
    ///
    #[rt::frame(m, "JSONEncoder.__init__", 108)]
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        skipkeys: &Bound<'py, PyAny>,
        ensure_ascii: &Bound<'py, PyAny>,
        check_circular: &Bound<'py, PyAny>,
        allow_nan: &Bound<'py, PyAny>,
        sort_keys: &Bound<'py, PyAny>,
        indent: &Bound<'py, PyAny>,
        separators: &Bound<'py, PyAny>,
        default: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        slf.set_attr("skipkeys", skipkeys)?;
        slf.set_attr("ensure_ascii", ensure_ascii)?;
        slf.set_attr("check_circular", check_circular)?;
        slf.set_attr("allow_nan", allow_nan)?;
        slf.set_attr("sort_keys", sort_keys)?;
        slf.set_attr("indent", indent)?;
        if !separators.is_none() {
            let [item_separator, key_separator] = rt::unpack(separators)?;
            slf.set_attr("item_separator", item_separator)?;
            slf.set_attr("key_separator", key_separator)?;
        } else if !indent.is_none() {
            slf.set_attr("item_separator", ",")?;
        }
        if !default.is_none() {
            slf.set_attr("default", default)?;
        }
        Ok(rt::none(m.py()))
    }

    // json/encoder.py:166
    /// Implement this method in a subclass such that it returns
    /// a serializable object for ``o``, or calls the base implementation
    /// (to raise a ``TypeError``).
    ///
    /// For example, to support arbitrary iterators, you could
    /// implement default like this::
    ///
    ///     def default(self, o):
    ///         try:
    ///             iterable = iter(o)
    ///         except TypeError:
    ///             pass
    ///         else:
    ///             return list(iterable)
    ///         # Let the base class default method raise the TypeError
    ///         return super().default(o)
    ///
    #[rt::frame(m, "JSONEncoder.default", 166, frame)]
    pub fn default<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        o: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        Err(frame.raise(PyTypeError::new_err((rt::fstring(
            m.py(),
            (
                "Object of type ",
                rt::format(&o.attr("__class__")?.attr("__name__")?, "")?,
                " is not JSON serializable",
            ),
        )?
        .unbind(),))))
    }

    // json/encoder.py:188
    /// Return a JSON string representation of a Python data structure.
    ///
    /// >>> from json.encoder import JSONEncoder
    /// >>> JSONEncoder().encode({"foo": ["bar", "baz"]})
    /// '{"foo": ["bar", "baz"]}'
    ///
    #[rt::frame(m, "JSONEncoder.encode", 188)]
    pub fn encode<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        o: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        // This is for extremely simple cases and benchmarks.
        if o.is_instance(&rt::builtin(py, "str")?)? {
            if slf.attr("ensure_ascii")?.is_truthy()? {
                return m.global("encode_basestring_ascii")?.call1((o,));
            } else {
                return m.global("encode_basestring")?.call1((o,));
            }
        }
        // This doesn't pass the iterator directly to ''.join() because the
        // exceptions aren't as detailed.  The list call should be roughly
        // equivalent to the PySequence_Fast that ''.join() would do.
        let mut chunks = rt::call!(slf.attr("iterencode")?, o, _one_shot = true)?;
        if !chunks.is_instance(
            PyTuple::new(py, [&rt::builtin(py, "list")?, &rt::builtin(py, "tuple")?])?.as_any(),
        )? {
            chunks = rt::builtins::list(py, (&chunks,))?;
        }
        rt::str(py, "").method1("join", (&chunks,))
    }

    // json/encoder.py:210
    /// Encode the given object and yield each string
    /// representation as available.
    ///
    /// For example::
    ///
    ///     for chunk in JSONEncoder().iterencode(bigobject):
    ///         mysocket.write(chunk)
    ///
    #[rt::frame(m, "JSONEncoder.iterencode", 210, frame)]
    pub fn iterencode<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        o: &Bound<'py, PyAny>,
        _one_shot: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        let markers = if slf.attr("check_circular")?.is_truthy()? {
            PyDict::new(py).into_any()
        } else {
            rt::none(py)
        };
        let _encoder = if slf.attr("ensure_ascii")?.is_truthy()? {
            m.global("encode_basestring_ascii")?
        } else {
            m.global("encode_basestring")?
        };
        // json/encoder.py:229
        #[rt::frame(m, "JSONEncoder.iterencode.<locals>.floatstr", 229, frame)]
        fn floatstr<'py>(
            m: &rt::Module<'py>,
            o: &Bound<'py, PyAny>,
            allow_nan: &Bound<'py, PyAny>,
            _repr: &Bound<'py, PyAny>,
            _inf: &Bound<'py, PyAny>,
            _neginf: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            // Check for specials.  Note that this type of test is processor
            // and/or platform-specific, so do tests which don't depend on the
            // internals.
            let text = if o.ne(o)? {
                rt::str(py, "NaN")
            } else if o.eq(_inf)? {
                rt::str(py, "Infinity")
            } else if o.eq(_neginf)? {
                rt::str(py, "-Infinity")
            } else {
                return _repr.call1((o,));
            };
            if !allow_nan.is_truthy()? {
                return Err(frame.raise(PyValueError::new_err((rt::str(
                    py,
                    "Out of range float values are not JSON compliant: ",
                )
                .add(o.repr()?)?
                .unbind(),))));
            }
            Ok(text)
        }
        let floatstr = rt::function!(
            frame,
            floatstr(
                o,
                allow_nan = slf.attr("allow_nan")?,
                _repr = rt::builtin(py, "float")?.attr("__repr__")?,
                _inf = m.global("INFINITY")?,
                _neginf = m.global("INFINITY")?.neg()?
            )
        )?;
        let indent = if slf.attr("indent")?.is_none()
            || slf.attr("indent")?.is_instance(&rt::builtin(py, "str")?)?
        {
            slf.attr("indent")?
        } else {
            rt::str(py, " ").mul(slf.attr("indent")?)?
        };
        let _iterencode = if _one_shot.is_truthy()? && !m.global("c_make_encoder")?.is_none() {
            m.global("c_make_encoder")?.call1((
                &markers,
                slf.attr("default")?,
                &_encoder,
                &indent,
                slf.attr("key_separator")?,
                slf.attr("item_separator")?,
                slf.attr("sort_keys")?,
                slf.attr("skipkeys")?,
                slf.attr("allow_nan")?,
            ))?
        } else {
            _make_iterencode(
                m,
                &markers,
                &slf.attr("default")?,
                &_encoder,
                &indent,
                &floatstr,
                &slf.attr("key_separator")?,
                &slf.attr("item_separator")?,
                &slf.attr("sort_keys")?,
                &slf.attr("skipkeys")?,
                _one_shot,
                &m.default("_make_iterencode", "ValueError")?,
                &m.default("_make_iterencode", "dict")?,
                &m.default("_make_iterencode", "float")?,
                &m.default("_make_iterencode", "id")?,
                &m.default("_make_iterencode", "int")?,
                &m.default("_make_iterencode", "isinstance")?,
                &m.default("_make_iterencode", "list")?,
                &m.default("_make_iterencode", "str")?,
                &m.default("_make_iterencode", "tuple")?,
                &m.default("_make_iterencode", "_intstr")?,
            )?
        };
        _iterencode.call1((o, 0))
    }
}

/// Executes the body of `json/encoder.py` in the module `m`.
#[rt::frame(m, "<module>", 1, frame)]
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    m.set_global("__doc__", "Implementation of JSONEncoder\n")?;
    // Copied unmodified (below this header) from Lib/json/encoder.py of CPython 3.14.9,
    // licensed under the PSF License Agreement. A steelix test project:
    // tests/test_projects.py runs it transpiled and with CPython and compares.
    m.set_global("re", rt::import(m, "re")?)?;
    match (|| -> PyResult<()> {
        let _json = rt::import_names(m, "_json", &["encode_basestring_ascii"])?;
        m.set_global(
            "c_encode_basestring_ascii",
            rt::import_from(&_json, "encode_basestring_ascii")?,
        )?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyImportError>(py) => {
            let _caught = frame.catch(err);
            m.set_global("c_encode_basestring_ascii", py.None())?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<()> {
        let _json = rt::import_names(m, "_json", &["encode_basestring"])?;
        m.set_global(
            "c_encode_basestring",
            rt::import_from(&_json, "encode_basestring")?,
        )?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyImportError>(py) => {
            let _caught = frame.catch(err);
            m.set_global("c_encode_basestring", py.None())?;
        }
        Err(err) => return Err(err),
    }
    match (|| -> PyResult<()> {
        let _json = rt::import_names(m, "_json", &["make_encoder"])?;
        m.set_global("c_make_encoder", rt::import_from(&_json, "make_encoder")?)?;
        Ok(())
    })() {
        Ok(()) => {}
        Err(err) if err.is_instance_of::<PyImportError>(py) => {
            let _caught = frame.catch(err);
            m.set_global("c_make_encoder", py.None())?;
        }
        Err(err) => return Err(err),
    }
    m.set_global(
        "ESCAPE",
        m.global("re")?
            .method1("compile", ("[\\x00-\\x1f\\\\\"\\b\\f\\n\\r\\t]",))?,
    )?;
    m.set_global(
        "ESCAPE_ASCII",
        m.global("re")?
            .method1("compile", ("([\\\\\"]|[^\\ -~])",))?,
    )?;
    m.set_global(
        "HAS_UTF8",
        m.global("re")?
            .method1("compile", (rt::bytes(py, b"[\x80-\xff]"),))?,
    )?;
    m.set_global(
        "ESCAPE_DCT",
        [
            ("\\", "\\\\"),
            ("\"", "\\\""),
            ("\u{8}", "\\b"),
            ("\u{c}", "\\f"),
            ("\n", "\\n"),
            ("\r", "\\r"),
            ("\t", "\\t"),
        ]
        .into_py_dict(py)?,
    )?;
    for item in rt::iter(&rt::builtins::range(py, (32,))?)? {
        m.set_global("i", item?)?;
        m.global("ESCAPE_DCT")?.attr("setdefault")?.call1((
            rt::builtins::chr(py, (m.global("i")?,))?,
            rt::str(py, "\\u{0:04x}")
                .attr("format")?
                .call1((m.global("i")?,))?,
        ))?;
    }
    // ESCAPE_DCT.setdefault(chr(i), '\\u%04x' % (i,))
    m.del_global("i")?;
    m.set_global("INFINITY", rt::builtins::float(py, ("inf",))?)?;
    rt::def!(
        m,
        py_encode_basestring(s),
        doc = "Return a JSON representation of a Python string\n\
            \n\
            \x20   "
    )?;
    {
        let c_encode_basestring = m.global("c_encode_basestring")?;
        m.set_global(
            "encode_basestring",
            if c_encode_basestring.is_truthy()? {
                c_encode_basestring
            } else {
                m.global("py_encode_basestring")?
            },
        )?;
    }
    rt::def!(
        m,
        py_encode_basestring_ascii(s),
        doc = "Return an ASCII-only JSON representation of a Python string\n\
            \n\
            \x20   "
    )?;
    {
        let c_encode_basestring_ascii = m.global("c_encode_basestring_ascii")?;
        m.set_global(
            "encode_basestring_ascii",
            if c_encode_basestring_ascii.is_truthy()? {
                c_encode_basestring_ascii
            } else {
                m.global("py_encode_basestring_ascii")?
            },
        )?;
    }
    m.set_global(
        "JSONEncoder",
        rt::class!(
            m,
            "JSONEncoder"(rt::builtin(py, "object")?),
            json_encoder::body,
            line = 77,
            attributes = [
                "allow_nan",
                "check_circular",
                "default",
                "ensure_ascii",
                "indent",
                "item_separator",
                "key_separator",
                "skipkeys",
                "sort_keys"
            ]
        )?,
    )?;
    rt::def!(
        m,
        _make_iterencode(
            markers,
            _default,
            _encoder,
            _indent,
            _floatstr,
            _key_separator,
            _item_separator,
            _sort_keys,
            _skipkeys,
            _one_shot,
            ValueError = rt::builtin(py, "ValueError")?,
            dict = rt::builtin(py, "dict")?,
            float = rt::builtin(py, "float")?,
            id = rt::builtin(py, "id")?,
            int = rt::builtin(py, "int")?,
            isinstance = rt::builtin(py, "isinstance")?,
            list = rt::builtin(py, "list")?,
            str = rt::builtin(py, "str")?,
            tuple = rt::builtin(py, "tuple")?,
            _intstr = rt::builtin(py, "int")?.attr("__repr__")?
        )
    )?;
    Ok(())
}
