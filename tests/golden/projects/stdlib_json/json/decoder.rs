use pyo3::exceptions::PyImportError;
use pyo3::exceptions::PyIndexError;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyStopIteration;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyList;
use pyo3::types::PyTuple;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

// json/decoder.py:63
pub fn _decode_uXXXX<'py>(
    module: &rt::Module<'py>,
    s: &Bound<'py, PyAny>,
    pos: &Bound<'py, PyAny>,
    _m: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = module.py();
    rt::frame!(module, "_decode_uXXXX", 63, |frame| {
        let esc = _m.call1((s, pos.add(1)?))?;
        if !esc.is_none() {
            match (|| rt::builtin(py, "int")?.call1((esc.method0("group")?, 16)))() {
                Ok(value) => return Ok(value),
                Err(err) if err.is_instance_of::<PyValueError>(py) => {}
                Err(err) => return Err(err),
            }
        }
        let msg = rt::str(py, "Invalid \\uXXXX escape");
        Err(frame.raise(module.global("JSONDecodeError")?.call1((&msg, s, pos))?))
    })
}

// json/decoder.py:73
/// Scan the string s for a JSON string. End is the index of the
/// character in s after the quote that started the JSON string.
/// Unescapes all valid JSON string escape sequences and raises ValueError
/// on attempt to decode an invalid string. If strict is False then literal
/// control characters are allowed in the string.
///
/// Returns a tuple of the decoded string and the index of the character in s
/// after the end quote.
pub fn py_scanstring<'py>(
    module: &rt::Module<'py>,
    s: &Bound<'py, PyAny>,
    end: &Bound<'py, PyAny>,
    strict: &Bound<'py, PyAny>,
    _b: &Bound<'py, PyAny>,
    _m: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = module.py();
    rt::frame!(module, "py_scanstring", 73, |frame| {
        let mut end = end.clone();
        let chunks = PyList::empty(py).into_any();
        let _append = chunks.attr("append")?;
        let begin = end.sub(1)?;
        let mut chunk: Bound<'py, PyAny>;
        let mut content: Bound<'py, PyAny>;
        let mut terminator: Bound<'py, PyAny>;
        let msg: Bound<'py, PyAny>;
        let mut esc: Bound<'py, PyAny>;
        let mut char: Bound<'py, PyAny>;
        let mut uni: Bound<'py, PyAny>;
        let mut uni2: Bound<'py, PyAny>;
        loop {
            chunk = _m.call1((s, &end))?;
            if chunk.is_none() {
                return Err(frame.raise(module.global("JSONDecodeError")?.call1((
                    "Unterminated string starting at",
                    s,
                    &begin,
                ))?));
            }
            end = chunk.method0("end")?;
            [content, terminator] = rt::unpack(&chunk.method0("groups")?)?;
            // Content is contains zero or more unescaped string characters
            if content.is_truthy()? {
                _append.call1((&content,))?;
            }
            // Terminator is the end of string, a literal control character,
            // or a backslash denoting that an escape sequence follows
            if terminator.eq("\"")? {
                break;
            } else if terminator.ne("\\")? {
                if strict.is_truthy()? {
                    // msg = "Invalid control character %r at" % (terminator,)
                    msg = rt::str(py, "Invalid control character {0!r} at")
                        .method1("format", (&terminator,))?;
                    return Err(frame.raise(module.global("JSONDecodeError")?.call1((
                        &msg,
                        s,
                        end.sub(1)?,
                    ))?));
                } else {
                    _append.call1((&terminator,))?;
                    py.check_signals()?;
                    continue;
                }
            }
            match s.get_item(&end) {
                Ok(value) => esc = value,
                Err(err) if err.is_instance_of::<PyIndexError>(py) => {
                    let _caught = frame.catch(err);
                    return Err(frame.raise_from(
                        module.global("JSONDecodeError")?.call1((
                            "Unterminated string starting at",
                            s,
                            &begin,
                        ))?,
                        py.None(),
                    ));
                }
                Err(err) => return Err(err),
            }
            // If not a unicode escape sequence, must be in the lookup table
            if esc.ne("u")? {
                match _b.get_item(&esc) {
                    Ok(value) => char = value,
                    Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                        let _caught = frame.catch(err);
                        msg = rt::str(py, "Invalid \\escape: {0!r}").method1("format", (&esc,))?;
                        return Err(
                            frame.raise(module.global("JSONDecodeError")?.call1((&msg, s, &end))?)
                        );
                    }
                    Err(err) => return Err(err),
                }
                end = rt::iadd(&end, 1)?;
            } else {
                uni = _decode_uXXXX(module, s, &end, &module.default("_decode_uXXXX", "_m")?)?;
                end = rt::iadd(&end, 5)?;
                if rt::int(py, 55296).le(&uni)?
                    && uni.le(56319)?
                    && s.get_item(rt::slice(py, &end, end.add(2)?)?)?.eq("\\u")?
                {
                    uni2 = _decode_uXXXX(
                        module,
                        s,
                        &end.add(1)?,
                        &module.default("_decode_uXXXX", "_m")?,
                    )?;
                    if rt::int(py, 56320).le(&uni2)? && uni2.le(57343)? {
                        uni = rt::int(py, 65536)
                            .add(uni.sub(55296)?.lshift(10)?.bitor(uni2.sub(56320)?)?)?;
                        end = rt::iadd(&end, 6)?;
                    }
                }
                char = rt::builtin(py, "chr")?.call1((&uni,))?;
            }
            _append.call1((&char,))?;
            py.check_signals()?;
        }
        Ok(PyTuple::new(py, [&rt::str(py, "").method1("join", (&chunks,))?, &end])?.into_any())
    })
}

// json/decoder.py:140
pub fn JSONObject<'py>(
    m: &rt::Module<'py>,
    s_and_end: &Bound<'py, PyAny>,
    strict: &Bound<'py, PyAny>,
    scan_once: &Bound<'py, PyAny>,
    object_hook: &Bound<'py, PyAny>,
    object_pairs_hook: &Bound<'py, PyAny>,
    memo: &Bound<'py, PyAny>,
    _w: &Bound<'py, PyAny>,
    _ws: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "JSONObject", 140, |frame| {
        let mut memo = memo.clone();
        let [s, mut end] = rt::unpack(s_and_end)?;
        let mut pairs = PyList::empty(py).into_any();
        let pairs_append = pairs.attr("append")?;
        // Backwards compatibility
        if memo.is_none() {
            memo = PyDict::new(py).into_any();
        }
        let memo_get = memo.attr("setdefault")?;
        // Use a slice to prevent IndexError from being raised, the following
        // check will raise a more specific ValueError if the string is empty
        let mut nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
        let result: Bound<'py, PyAny>;
        // Normally we expect nextchar == '"'
        if nextchar.ne("\"")? {
            if _ws.contains(&nextchar)? {
                end = _w.call1((&s, &end))?.method0("end")?;
                nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
            }
            // Trivial empty object
            if nextchar.eq("}")? {
                if !object_pairs_hook.is_none() {
                    result = object_pairs_hook.call1((&pairs,))?;
                    return Ok(PyTuple::new(py, [&result, &end.add(1)?])?.into_any());
                }
                pairs = PyDict::new(py).into_any();
                if !object_hook.is_none() {
                    pairs = object_hook.call1((&pairs,))?;
                }
                return Ok(PyTuple::new(py, [&pairs, &end.add(1)?])?.into_any());
            } else if nextchar.ne("\"")? {
                return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                    "Expecting property name enclosed in double quotes",
                    &s,
                    &end,
                ))?));
            }
        }
        end = rt::iadd(&end, 1)?;
        let mut key: Bound<'py, PyAny>;
        let mut value: Option<Bound<'py, PyAny>> = None;
        let mut comma_idx: Bound<'py, PyAny>;
        loop {
            [key, end] = rt::unpack(&m.global("scanstring")?.call1((&s, &end, strict))?)?;
            key = memo_get.call1((&key, &key))?;
            // To skip some function call overhead we optimize the fast paths where
            // the JSON key separator is ": " or just ":".
            if s.get_item(rt::slice(py, &end, end.add(1)?)?)?.ne(":")? {
                end = _w.call1((&s, &end))?.method0("end")?;
                if s.get_item(rt::slice(py, &end, end.add(1)?)?)?.ne(":")? {
                    return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                        "Expecting ':' delimiter",
                        &s,
                        &end,
                    ))?));
                }
            }
            end = rt::iadd(&end, 1)?;
            match (|| -> PyResult<()> {
                if _ws.contains(s.get_item(&end)?)? {
                    end = rt::iadd(&end, 1)?;
                    if _ws.contains(s.get_item(&end)?)? {
                        end = _w.call1((&s, end.add(1)?))?.method0("end")?;
                    }
                }
                Ok(())
            })() {
                Ok(()) => {}
                Err(err2) if err2.is_instance_of::<PyIndexError>(py) => {}
                Err(err2) => return Err(err2),
            }
            match (|| -> PyResult<()> {
                let [value2, end2] = rt::unpack(&scan_once.call1((&s, &end))?)?;
                value = Some(value2);
                end = end2;
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                    let err = frame.catch(err);
                    return Err(frame.raise_from(
                        m.global("JSONDecodeError")?.call1((
                            "Expecting value",
                            &s,
                            err.attr("value")?,
                        ))?,
                        py.None(),
                    ));
                }
                Err(err2) => return Err(err2),
            }
            pairs_append.call1((PyTuple::new(py, [&key, rt::local(&value, "value")?])?,))?;
            match (|| -> PyResult<()> {
                nextchar = s.get_item(&end)?;
                if _ws.contains(&nextchar)? {
                    end = _w.call1((&s, end.add(1)?))?.method0("end")?;
                    nextchar = s.get_item(&end)?;
                }
                Ok(())
            })() {
                Ok(()) => {}
                Err(err2) if err2.is_instance_of::<PyIndexError>(py) => nextchar = rt::str(py, ""),
                Err(err2) => return Err(err2),
            }
            end = rt::iadd(&end, 1)?;
            if nextchar.eq("}")? {
                break;
            } else if nextchar.ne(",")? {
                return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                    "Expecting ',' delimiter",
                    &s,
                    end.sub(1)?,
                ))?));
            }
            comma_idx = end.sub(1)?;
            end = _w.call1((&s, &end))?.method0("end")?;
            nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
            end = rt::iadd(&end, 1)?;
            if nextchar.ne("\"")? {
                if nextchar.eq("}")? {
                    return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                        "Illegal trailing comma before end of object",
                        &s,
                        &comma_idx,
                    ))?));
                }
                return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                    "Expecting property name enclosed in double quotes",
                    &s,
                    end.sub(1)?,
                ))?));
            }
            py.check_signals()?;
        }
        if !object_pairs_hook.is_none() {
            result = object_pairs_hook.call1((&pairs,))?;
            return Ok(PyTuple::new(py, [&result, &end])?.into_any());
        }
        pairs = rt::builtin(py, "dict")?.call1((&pairs,))?;
        if !object_hook.is_none() {
            pairs = object_hook.call1((&pairs,))?;
        }
        Ok(PyTuple::new(py, [&pairs, &end])?.into_any())
    })
}

// json/decoder.py:224
pub fn JSONArray<'py>(
    m: &rt::Module<'py>,
    s_and_end: &Bound<'py, PyAny>,
    scan_once: &Bound<'py, PyAny>,
    _w: &Bound<'py, PyAny>,
    _ws: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    let py = m.py();
    rt::frame!(m, "JSONArray", 224, |frame| {
        let [s, mut end] = rt::unpack(s_and_end)?;
        let values = PyList::empty(py).into_any();
        let mut nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
        if _ws.contains(&nextchar)? {
            end = _w.call1((&s, end.add(1)?))?.method0("end")?;
            nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
        }
        // Look-ahead for trivial empty array
        if nextchar.eq("]")? {
            return Ok(PyTuple::new(py, [&values, &end.add(1)?])?.into_any());
        }
        let _append = values.attr("append")?;
        let mut value: Option<Bound<'py, PyAny>> = None;
        let mut comma_idx: Bound<'py, PyAny>;
        loop {
            match (|| -> PyResult<()> {
                let [value2, end2] = rt::unpack(&scan_once.call1((&s, &end))?)?;
                value = Some(value2);
                end = end2;
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                    let err = frame.catch(err);
                    return Err(frame.raise_from(
                        m.global("JSONDecodeError")?.call1((
                            "Expecting value",
                            &s,
                            err.attr("value")?,
                        ))?,
                        py.None(),
                    ));
                }
                Err(err2) => return Err(err2),
            }
            _append.call1((rt::local(&value, "value")?,))?;
            nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
            if _ws.contains(&nextchar)? {
                end = _w.call1((&s, end.add(1)?))?.method0("end")?;
                nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
            }
            end = rt::iadd(&end, 1)?;
            if nextchar.eq("]")? {
                break;
            } else if nextchar.ne(",")? {
                return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                    "Expecting ',' delimiter",
                    &s,
                    end.sub(1)?,
                ))?));
            }
            comma_idx = end.sub(1)?;
            match (|| -> PyResult<()> {
                if _ws.contains(s.get_item(&end)?)? {
                    end = rt::iadd(&end, 1)?;
                    if _ws.contains(s.get_item(&end)?)? {
                        end = _w.call1((&s, end.add(1)?))?.method0("end")?;
                    }
                }
                nextchar = s.get_item(rt::slice(py, &end, end.add(1)?)?)?;
                Ok(())
            })() {
                Ok(()) => {}
                Err(err2) if err2.is_instance_of::<PyIndexError>(py) => {}
                Err(err2) => return Err(err2),
            }
            if nextchar.eq("]")? {
                return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                    "Illegal trailing comma before end of array",
                    &s,
                    &comma_idx,
                ))?));
            }
            py.check_signals()?;
        }
        Ok(PyTuple::new(py, [&values, &end])?.into_any())
    })
}

/// Subclass of ValueError with the following additional properties:
///
/// msg: The unformatted error message
/// doc: The JSON document being parsed
/// pos: The start index of doc where parsing failed
/// lineno: The line corresponding to pos
/// colno: The column corresponding to pos
///
pub mod json_decode_error {
    use super::*;

    // json/decoder.py:23
    /// The body of `class JSONDecodeError`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        rt::frame!(ns, "JSONDecodeError", 23, |_| {
            ns.set(
                "__doc__",
                "Subclass of ValueError with the following additional properties:\n\
                    \n\
                    msg: The unformatted error message\n\
                    doc: The JSON document being parsed\n\
                    pos: The start index of doc where parsing failed\n\
                    lineno: The line corresponding to pos\n\
                    colno: The column corresponding to pos\n\
                    \n",
            )?;
            rt::def!(ns, __init__(slf as "self", msg, doc, pos))?;
            rt::def!(ns, __reduce__(slf as "self"))?;
            Ok(())
        })
    }

    // Note that this exception is used from _json
    // json/decoder.py:34
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        msg: &Bound<'py, PyAny>,
        doc: &Bound<'py, PyAny>,
        pos: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "JSONDecodeError.__init__", 34, |_| {
            let lineno = doc.method1("count", ("\n", 0, pos))?.add(1)?;
            let colno = pos.sub(doc.method1("rfind", ("\n", 0, pos))?)?;
            let errmsg = rt::str(py, "%s: line %d column %d (char %d)")
                .rem(PyTuple::new(py, [msg, &lineno, &colno, pos])?)?;
            rt::builtin(py, "ValueError")?.method1("__init__", (slf, &errmsg))?;
            slf.set_attr("msg", msg)?;
            slf.set_attr("doc", doc)?;
            slf.set_attr("pos", pos)?;
            slf.set_attr("lineno", &lineno)?;
            slf.set_attr("colno", &colno)?;
            Ok(rt::none(py))
        })
    }

    // json/decoder.py:45
    pub fn __reduce__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "JSONDecodeError.__reduce__", 45, |_| {
            Ok(PyTuple::new(
                py,
                [
                    &slf.attr("__class__")?,
                    PyTuple::new(
                        py,
                        [&slf.attr("msg")?, &slf.attr("doc")?, &slf.attr("pos")?],
                    )?
                    .as_any(),
                ],
            )?
            .into_any())
        })
    }
}

/// Simple JSON <https://json.org> decoder
///
/// Performs the following translations in decoding by default:
///
/// +---------------+-------------------+
/// | JSON          | Python            |
/// +===============+===================+
/// | object        | dict              |
/// +---------------+-------------------+
/// | array         | list              |
/// +---------------+-------------------+
/// | string        | str               |
/// +---------------+-------------------+
/// | number (int)  | int               |
/// +---------------+-------------------+
/// | number (real) | float             |
/// +---------------+-------------------+
/// | true          | True              |
/// +---------------+-------------------+
/// | false         | False             |
/// +---------------+-------------------+
/// | null          | None              |
/// +---------------+-------------------+
///
/// It also understands ``NaN``, ``Infinity``, and ``-Infinity`` as
/// their corresponding ``float`` values, which is outside the JSON spec.
///
pub mod json_decoder {
    use super::*;

    // json/decoder.py:265
    /// The body of `class JSONDecoder`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "JSONDecoder", 265, |_| {
            ns.set(
                "__doc__",
                "Simple JSON <https://json.org> decoder\n\
                    \n\
                    Performs the following translations in decoding by default:\n\
                    \n\
                    +---------------+-------------------+\n\
                    | JSON          | Python            |\n\
                    +===============+===================+\n\
                    | object        | dict              |\n\
                    +---------------+-------------------+\n\
                    | array         | list              |\n\
                    +---------------+-------------------+\n\
                    | string        | str               |\n\
                    +---------------+-------------------+\n\
                    | number (int)  | int               |\n\
                    +---------------+-------------------+\n\
                    | number (real) | float             |\n\
                    +---------------+-------------------+\n\
                    | true          | True              |\n\
                    +---------------+-------------------+\n\
                    | false         | False             |\n\
                    +---------------+-------------------+\n\
                    | null          | None              |\n\
                    +---------------+-------------------+\n\
                    \n\
                    It also understands ``NaN``, ``Infinity``, and ``-Infinity`` as\n\
                    their corresponding ``float`` values, which is outside the JSON spec.\n\
                    \n",
            )?;
            rt::def!(
                ns,
                __init__(slf as "self", *, object_hook = py.None(), parse_float = py.None(), parse_int = py.None(), parse_constant = py.None(), strict = true, object_pairs_hook = py.None()),
                doc = "``object_hook``, if specified, will be called with the result\n\
                    of every JSON object decoded and its return value will be used in\n\
                    place of the given ``dict``.  This can be used to provide custom\n\
                    deserializations (e.g. to support JSON-RPC class hinting).\n\
                    \n\
                    ``object_pairs_hook``, if specified will be called with the result\n\
                    of every JSON object decoded with an ordered list of pairs.  The\n\
                    return value of ``object_pairs_hook`` will be used instead of the\n\
                    ``dict``.  This feature can be used to implement custom decoders.\n\
                    If ``object_hook`` is also defined, the ``object_pairs_hook`` takes\n\
                    priority.\n\
                    \n\
                    ``parse_float``, if specified, will be called with the string\n\
                    of every JSON float to be decoded. By default this is equivalent to\n\
                    float(num_str). This can be used to use another datatype or parser\n\
                    for JSON floats (e.g. decimal.Decimal).\n\
                    \n\
                    ``parse_int``, if specified, will be called with the string\n\
                    of every JSON int to be decoded. By default this is equivalent to\n\
                    int(num_str). This can be used to use another datatype or parser\n\
                    for JSON integers (e.g. float).\n\
                    \n\
                    ``parse_constant``, if specified, will be called with one of the\n\
                    following strings: -Infinity, Infinity, NaN.\n\
                    This can be used to raise an exception if invalid JSON numbers\n\
                    are encountered.\n\
                    \n\
                    If ``strict`` is false (true is the default), then control\n\
                    characters will be allowed inside strings.  Control characters in\n\
                    this context are those with character codes in the 0-31 range,\n\
                    including ``'\\t'`` (tab), ``'\\n'``, ``'\\r'`` and ``'\\0'``.\n"
            )?;
            rt::def!(
                ns,
                decode(slf as "self", s, _w = ns.get("WHITESPACE")?.attr("match")?),
                doc = "Return the Python representation of ``s`` (a ``str`` instance\n\
                    containing a JSON document).\n\
                    \n"
            )?;
            rt::def!(
                ns,
                raw_decode(slf as "self", s, idx = 0),
                doc = "Decode a JSON document from ``s`` (a ``str`` beginning with\n\
                    a JSON document) and return a 2-tuple of the Python\n\
                    representation and the index in ``s`` where the document ended.\n\
                    \n\
                    This can be used to decode a JSON document from a string that may\n\
                    have extraneous data at the end.\n\
                    \n"
            )?;
            Ok(())
        })
    }

    // json/decoder.py:295
    /// ``object_hook``, if specified, will be called with the result
    /// of every JSON object decoded and its return value will be used in
    /// place of the given ``dict``.  This can be used to provide custom
    /// deserializations (e.g. to support JSON-RPC class hinting).
    ///
    /// ``object_pairs_hook``, if specified will be called with the result
    /// of every JSON object decoded with an ordered list of pairs.  The
    /// return value of ``object_pairs_hook`` will be used instead of the
    /// ``dict``.  This feature can be used to implement custom decoders.
    /// If ``object_hook`` is also defined, the ``object_pairs_hook`` takes
    /// priority.
    ///
    /// ``parse_float``, if specified, will be called with the string
    /// of every JSON float to be decoded. By default this is equivalent to
    /// float(num_str). This can be used to use another datatype or parser
    /// for JSON floats (e.g. decimal.Decimal).
    ///
    /// ``parse_int``, if specified, will be called with the string
    /// of every JSON int to be decoded. By default this is equivalent to
    /// int(num_str). This can be used to use another datatype or parser
    /// for JSON integers (e.g. float).
    ///
    /// ``parse_constant``, if specified, will be called with one of the
    /// following strings: -Infinity, Infinity, NaN.
    /// This can be used to raise an exception if invalid JSON numbers
    /// are encountered.
    ///
    /// If ``strict`` is false (true is the default), then control
    /// characters will be allowed inside strings.  Control characters in
    /// this context are those with character codes in the 0-31 range,
    /// including ``'\t'`` (tab), ``'\n'``, ``'\r'`` and ``'\0'``.
    pub fn __init__<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        object_hook: &Bound<'py, PyAny>,
        parse_float: &Bound<'py, PyAny>,
        parse_int: &Bound<'py, PyAny>,
        parse_constant: &Bound<'py, PyAny>,
        strict: &Bound<'py, PyAny>,
        object_pairs_hook: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "JSONDecoder.__init__", 295, |_| {
            slf.set_attr("object_hook", object_hook)?;
            slf.set_attr(
                "parse_float",
                if parse_float.is_truthy()? {
                    parse_float.clone()
                } else {
                    rt::builtin(py, "float")?
                },
            )?;
            slf.set_attr(
                "parse_int",
                if parse_int.is_truthy()? {
                    parse_int.clone()
                } else {
                    rt::builtin(py, "int")?
                },
            )?;
            slf.set_attr(
                "parse_constant",
                if parse_constant.is_truthy()? {
                    parse_constant.clone()
                } else {
                    m.global("_CONSTANTS")?.attr("__getitem__")?
                },
            )?;
            slf.set_attr("strict", strict)?;
            slf.set_attr("object_pairs_hook", object_pairs_hook)?;
            slf.set_attr("parse_object", m.global("JSONObject")?)?;
            slf.set_attr("parse_array", m.global("JSONArray")?)?;
            slf.set_attr("parse_string", m.global("scanstring")?)?;
            slf.set_attr("memo", PyDict::new(py))?;
            slf.set_attr(
                "scan_once",
                m.global("scanner")?.method1("make_scanner", (slf,))?,
            )?;
            Ok(rt::none(py))
        })
    }

    // json/decoder.py:343
    /// Return the Python representation of ``s`` (a ``str`` instance
    /// containing a JSON document).
    ///
    pub fn decode<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        s: &Bound<'py, PyAny>,
        _w: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "JSONDecoder.decode", 343, |frame| {
            let [obj, mut end] = rt::unpack(&rt::call!(
                slf.attr("raw_decode")?,
                s,
                idx = _w.call1((s, 0))?.method0("end")?
            )?)?;
            end = _w.call1((s, &end))?.method0("end")?;
            if end.ne(s.len()?)? {
                return Err(frame.raise(m.global("JSONDecodeError")?.call1((
                    "Extra data",
                    s,
                    &end,
                ))?));
            }
            Ok(obj)
        })
    }

    // json/decoder.py:354
    /// Decode a JSON document from ``s`` (a ``str`` beginning with
    /// a JSON document) and return a 2-tuple of the Python
    /// representation and the index in ``s`` where the document ended.
    ///
    /// This can be used to decode a JSON document from a string that may
    /// have extraneous data at the end.
    ///
    pub fn raw_decode<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        s: &Bound<'py, PyAny>,
        idx: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "JSONDecoder.raw_decode", 354, |frame| {
            let mut obj: Option<Bound<'py, PyAny>> = None;
            let mut end: Option<Bound<'py, PyAny>> = None;
            match (|| -> PyResult<()> {
                let [obj2, end2] = rt::unpack(&slf.method1("scan_once", (s, idx))?)?;
                obj = Some(obj2);
                end = Some(end2);
                Ok(())
            })() {
                Ok(()) => {}
                Err(err) if err.is_instance_of::<PyStopIteration>(py) => {
                    let err = frame.catch(err);
                    return Err(frame.raise_from(
                        m.global("JSONDecodeError")?.call1((
                            "Expecting value",
                            s,
                            err.attr("value")?,
                        ))?,
                        py.None(),
                    ));
                }
                Err(err2) => return Err(err2),
            }
            Ok(PyTuple::new(py, [rt::local(&obj, "obj")?, rt::local(&end, "end")?])?.into_any())
        })
    }
}

/// Executes the body of `json/decoder.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Implementation of JSONDecoder\n")?;
        // Copied unmodified (below this header) from Lib/json/decoder.py of CPython 3.14.9,
        // licensed under the PSF License Agreement. A steelix test project:
        // tests/test_projects.py runs it transpiled and with CPython and compares.
        m.set_global("re", rt::import(m, "re")?)?;
        let json = rt::import_names(m, "json", &["scanner"])?;
        m.set_global("scanner", rt::import_from(&json, "scanner")?)?;
        match (|| -> PyResult<()> {
            let _json = rt::import_names(m, "_json", &["scanstring"])?;
            m.set_global("c_scanstring", rt::import_from(&_json, "scanstring")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let _caught = frame.catch(err);
                m.set_global("c_scanstring", py.None())?;
            }
            Err(err) => return Err(err),
        }
        m.set_global(
            "__all__",
            PyList::new(py, ["JSONDecoder", "JSONDecodeError"])?,
        )?;
        m.set_global(
            "FLAGS",
            m.global("re")?
                .attr("VERBOSE")?
                .bitor(m.global("re")?.attr("MULTILINE")?)?
                .bitor(m.global("re")?.attr("DOTALL")?)?,
        )?;
        m.set_global("NaN", rt::builtin(py, "float")?.call1(("nan",))?)?;
        m.set_global("PosInf", rt::builtin(py, "float")?.call1(("inf",))?)?;
        m.set_global("NegInf", rt::builtin(py, "float")?.call1(("-inf",))?)?;
        m.set_global(
            "JSONDecodeError",
            rt::class!(
                m,
                "JSONDecodeError"(rt::builtin(py, "ValueError")?),
                json_decode_error::body,
                line = 23,
                attributes = ["colno", "doc", "lineno", "msg", "pos"]
            )?,
        )?;
        m.set_global(
            "_CONSTANTS",
            [
                ("-Infinity", &m.global("NegInf")?),
                ("Infinity", &m.global("PosInf")?),
                ("NaN", &m.global("NaN")?),
            ]
            .into_py_dict(py)?,
        )?;
        m.set_global(
            "HEXDIGITS",
            m.global("re")?
                .attr("compile")?
                .call1(("[0-9A-Fa-f]{4}", m.global("FLAGS")?))?,
        )?;
        m.set_global(
            "STRINGCHUNK",
            m.global("re")?
                .attr("compile")?
                .call1(("(.*?)([\"\\\\\\x00-\\x1f])", m.global("FLAGS")?))?,
        )?;
        m.set_global(
            "BACKSLASH",
            [
                ("\"", "\""),
                ("\\", "\\"),
                ("/", "/"),
                ("b", "\u{8}"),
                ("f", "\u{c}"),
                ("n", "\n"),
                ("r", "\r"),
                ("t", "\t"),
            ]
            .into_py_dict(py)?,
        )?;
        rt::def!(
            m,
            _decode_uXXXX(s, pos, _m = m.global("HEXDIGITS")?.attr("match")?)
        )?;
        rt::def!(
            m,
            py_scanstring(
                s,
                end,
                strict = true,
                _b = m.global("BACKSLASH")?,
                _m = m.global("STRINGCHUNK")?.attr("match")?
            ),
            doc = "Scan the string s for a JSON string. End is the index of the\n\
                character in s after the quote that started the JSON string.\n\
                Unescapes all valid JSON string escape sequences and raises ValueError\n\
                on attempt to decode an invalid string. If strict is False then literal\n\
                control characters are allowed in the string.\n\
                \n\
                Returns a tuple of the decoded string and the index of the character in s\n\
                after the end quote."
        )?;
        // Use speedup if available
        {
            let c_scanstring = m.global("c_scanstring")?;
            m.set_global(
                "scanstring",
                if c_scanstring.is_truthy()? {
                    c_scanstring
                } else {
                    m.global("py_scanstring")?
                },
            )?;
        }
        m.set_global(
            "WHITESPACE",
            m.global("re")?
                .attr("compile")?
                .call1(("[ \\t\\n\\r]*", m.global("FLAGS")?))?,
        )?;
        m.set_global("WHITESPACE_STR", " \t\n\r")?;
        rt::def!(
            m,
            JSONObject(
                s_and_end,
                strict,
                scan_once,
                object_hook,
                object_pairs_hook,
                memo = py.None(),
                _w = m.global("WHITESPACE")?.attr("match")?,
                _ws = m.global("WHITESPACE_STR")?
            )
        )?;
        rt::def!(
            m,
            JSONArray(
                s_and_end,
                scan_once,
                _w = m.global("WHITESPACE")?.attr("match")?,
                _ws = m.global("WHITESPACE_STR")?
            )
        )?;
        m.set_global(
            "JSONDecoder",
            rt::class!(
                m,
                "JSONDecoder"(rt::builtin(py, "object")?),
                json_decoder::body,
                line = 265,
                attributes = [
                    "memo",
                    "object_hook",
                    "object_pairs_hook",
                    "parse_array",
                    "parse_constant",
                    "parse_float",
                    "parse_int",
                    "parse_object",
                    "parse_string",
                    "scan_once",
                    "strict"
                ]
            )?,
        )?;
        Ok(())
    })
}
