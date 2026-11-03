//! Parts of `json/encoder.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 268, `def _make_iterencode`: yield, yield from

use pyo3::exceptions::PyImportError;
use pyo3::exceptions::PyKeyError;
use pyo3::exceptions::PyTypeError;
use pyo3::exceptions::PyValueError;
use pyo3::prelude::*;
use pyo3::types::IntoPyDict;
use pyo3::types::PyDict;
use pyo3::types::PyTuple;
use steelix_runtime as rt;

// json/encoder.py:40
/// Return a JSON representation of a Python string
///
///
pub fn py_encode_basestring<'py>(
    m: &rt::Module<'py>,
    s: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "py_encode_basestring", 40, |frame| {
        // json/encoder.py:44
        fn replace<'py>(
            m: &rt::Module<'py>,
            r#match: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            rt::frame!(m, "py_encode_basestring.<locals>.replace", 44, |frame| {
                frame.line(45);
                m.global("ESCAPE_DCT")?
                    .get_item(r#match.call_method1("group", (0,))?)
            })
        }
        let replace = rt::function!(
            m,
            replace(r#match),
            qualname = "py_encode_basestring.<locals>.replace"
        )?;
        frame.line(46);
        rt::str(m.py(), "\"")
            .add(m.global("ESCAPE")?.call_method1("sub", (&replace, s))?)?
            .add("\"")
    })
}

// json/encoder.py:52
/// Return an ASCII-only JSON representation of a Python string
///
///
pub fn py_encode_basestring_ascii<'py>(
    m: &rt::Module<'py>,
    s: &Bound<'py, PyAny>,
) -> PyResult<Bound<'py, PyAny>> {
    rt::frame!(m, "py_encode_basestring_ascii", 52, |frame| {
        // json/encoder.py:56
        fn replace<'py>(
            m: &rt::Module<'py>,
            r#match: &Bound<'py, PyAny>,
        ) -> PyResult<Bound<'py, PyAny>> {
            let py = m.py();
            rt::frame!(
                m,
                "py_encode_basestring_ascii.<locals>.replace",
                56,
                |frame| {
                    let mut n: Bound<'py, PyAny>;
                    let s1: Bound<'py, PyAny>;
                    let s2: Bound<'py, PyAny>;
                    frame.line(57);
                    let s = r#match.call_method1("group", (0,))?;
                    frame.line(59);
                    match (|| m.global("ESCAPE_DCT")?.get_item(&s))() {
                        Ok(value) => Ok(value),
                        Err(err) if err.is_instance_of::<PyKeyError>(py) => {
                            let _handling = rt::Handling::new(&frame.catch(err));
                            frame.line(61);
                            n = rt::builtin(py, "ord")?.call1((&s,))?;
                            frame.line(62);
                            if n.lt(65536)? {
                                frame.line(63);
                                rt::str(py, "\\u{0:04x}").call_method1("format", (&n,))
                            } else {
                                // return '\\u%04x' % (n,)
                                // surrogate pair
                                frame.line(67);
                                n = rt::isub(&n, 65536)?;
                                frame.line(68);
                                s1 = rt::int(py, 55296).bitor(n.rshift(10)?.bitand(1023)?)?;
                                frame.line(69);
                                s2 = rt::int(py, 56320).bitor(n.bitand(1023)?)?;
                                frame.line(70);
                                rt::str(py, "\\u{0:04x}\\u{1:04x}")
                                    .call_method1("format", (&s1, &s2))
                            }
                        }
                        Err(err) => Err(err),
                    }
                }
            )
        }
        let replace = rt::function!(
            m,
            replace(r#match),
            qualname = "py_encode_basestring_ascii.<locals>.replace"
        )?;
        frame.line(71);
        rt::str(m.py(), "\"")
            .add(
                m.global("ESCAPE_ASCII")?
                    .call_method1("sub", (&replace, s))?,
            )?
            .add("\"")
    })
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
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        let py = m.py();
        rt::frame!(ns, "JSONEncoder", 77, |frame| {
            frame.line(78);
            ns.set("__doc__", "Extensible JSON <https://json.org> encoder for Python data structures.\n\nSupports the following objects and types by default:\n\n+-------------------+---------------+\n| Python            | JSON          |\n+===================+===============+\n| dict              | object        |\n+-------------------+---------------+\n| list, tuple       | array         |\n+-------------------+---------------+\n| str               | string        |\n+-------------------+---------------+\n| int, float        | number        |\n+-------------------+---------------+\n| True              | true          |\n+-------------------+---------------+\n| False             | false         |\n+-------------------+---------------+\n| None              | null          |\n+-------------------+---------------+\n\nTo extend this to recognize other objects, subclass and implement a\n``.default()`` method with another method that returns a serializable\nobject for ``o`` if possible, otherwise it should call the superclass\nimplementation (to raise ``TypeError``).\n\n")?;
            frame.line(106);
            ns.set("item_separator", ", ")?;
            frame.line(107);
            ns.set("key_separator", ": ")?;
            frame.line(108);
            ns.set("__init__", rt::function!(
    ns,
    __init__(slf as "self", *, skipkeys = false, ensure_ascii = true, check_circular = true, allow_nan = true, sort_keys = false, indent = py.None(), separators = py.None(), default = py.None()),
    qualname = "JSONEncoder.__init__",
    doc = "Constructor for JSONEncoder, with sensible defaults.\n\nIf skipkeys is false, then it is a TypeError to attempt\nencoding of keys that are not str, int, float, bool or None.\nIf skipkeys is True, such items are simply skipped.\n\nIf ensure_ascii is true, the output is guaranteed to be str objects\nwith all incoming non-ASCII and non-printable characters escaped.\nIf ensure_ascii is false, the output can contain non-ASCII and\nnon-printable characters.\n\nIf check_circular is true, then lists, dicts, and custom encoded\nobjects will be checked for circular references during encoding to\nprevent an infinite recursion (which would cause an RecursionError).\nOtherwise, no such check takes place.\n\nIf allow_nan is true, then NaN, Infinity, and -Infinity will be\nencoded as such.  This behavior is not JSON specification compliant,\nbut is consistent with most JavaScript based encoders and decoders.\nOtherwise, it will be a ValueError to encode such floats.\n\nIf sort_keys is true, then the output of dictionaries will be\nsorted by key; this is useful for regression tests to ensure\nthat JSON serializations can be compared on a day-to-day basis.\n\nIf indent is a non-negative integer, then JSON array\nelements and object members will be pretty-printed with that\nindent level.  An indent level of 0 will only insert newlines.\nNone is the most compact representation.\n\nIf specified, separators should be an (item_separator,\nkey_separator) tuple.  The default is (', ', ': ') if *indent* is\n``None`` and (',', ': ') otherwise.  To get the most compact JSON\nrepresentation, you should specify (',', ':') to eliminate\nwhitespace.\n\nIf specified, default is a function that gets called for objects\nthat can't otherwise be serialized.  It should return a JSON\nencodable version of the object or raise a ``TypeError``.\n\n"
)?)?;
            frame.line(166);
            ns.set("default", rt::function!(
    ns,
    default(slf as "self", o),
    qualname = "JSONEncoder.default",
    doc = "Implement this method in a subclass such that it returns\na serializable object for ``o``, or calls the base implementation\n(to raise a ``TypeError``).\n\nFor example, to support arbitrary iterators, you could\nimplement default like this::\n\n    def default(self, o):\n        try:\n            iterable = iter(o)\n        except TypeError:\n            pass\n        else:\n            return list(iterable)\n        # Let the base class default method raise the TypeError\n        return super().default(o)\n\n"
)?)?;
            frame.line(188);
            ns.set("encode", rt::function!(
    ns,
    encode(slf as "self", o),
    qualname = "JSONEncoder.encode",
    doc = "Return a JSON string representation of a Python data structure.\n\n>>> from json.encoder import JSONEncoder\n>>> JSONEncoder().encode({\"foo\": [\"bar\", \"baz\"]})\n'{\"foo\": [\"bar\", \"baz\"]}'\n\n"
)?)?;
            frame.line(210);
            ns.set("iterencode", rt::function!(
    ns,
    iterencode(slf as "self", o, _one_shot = false),
    qualname = "JSONEncoder.iterencode",
    doc = "Encode the given object and yield each string\nrepresentation as available.\n\nFor example::\n\n    for chunk in JSONEncoder().iterencode(bigobject):\n        mysocket.write(chunk)\n\n"
)?)?;
            Ok(())
        })
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
        rt::frame!(m, "JSONEncoder.__init__", 108, |frame| {
            frame.line(153);
            slf.setattr("skipkeys", skipkeys)?;
            frame.line(154);
            slf.setattr("ensure_ascii", ensure_ascii)?;
            frame.line(155);
            slf.setattr("check_circular", check_circular)?;
            frame.line(156);
            slf.setattr("allow_nan", allow_nan)?;
            frame.line(157);
            slf.setattr("sort_keys", sort_keys)?;
            frame.line(158);
            slf.setattr("indent", indent)?;
            if !separators.is_none() {
                frame.line(160);
                let [item_separator, key_separator] = rt::unpack(separators)?;
                slf.setattr("item_separator", item_separator)?;
                slf.setattr("key_separator", key_separator)?;
            } else if !indent.is_none() {
                frame.line(162);
                slf.setattr("item_separator", ",")?;
            }
            if !default.is_none() {
                frame.line(164);
                slf.setattr("default", default)?;
            }
            Ok(rt::none(m.py()))
        })
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
    pub fn default<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        o: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        rt::frame!(m, "JSONEncoder.default", 166, |frame| {
            frame.line(185);
            Err(frame.raise(PyTypeError::new_err((rt::fstring(
                m.py(),
                (
                    "Object of type ",
                    rt::format(&o.getattr("__class__")?.getattr("__name__")?, "")?,
                    " is not JSON serializable",
                ),
            )?
            .unbind(),))))
        })
    }

    // json/encoder.py:188
    /// Return a JSON string representation of a Python data structure.
    ///
    /// >>> from json.encoder import JSONEncoder
    /// >>> JSONEncoder().encode({"foo": ["bar", "baz"]})
    /// '{"foo": ["bar", "baz"]}'
    ///
    pub fn encode<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        o: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "JSONEncoder.encode", 188, |frame| {
            // This is for extremely simple cases and benchmarks.
            frame.line(197);
            if o.is_instance(&rt::builtin(py, "str")?)? {
                frame.line(198);
                if slf.getattr("ensure_ascii")?.is_truthy()? {
                    frame.line(199);
                    return m.global("encode_basestring_ascii")?.call1((o,));
                } else {
                    frame.line(201);
                    return m.global("encode_basestring")?.call1((o,));
                }
            }
            // This doesn't pass the iterator directly to ''.join() because the
            // exceptions aren't as detailed.  The list call should be roughly
            // equivalent to the PySequence_Fast that ''.join() would do.
            frame.line(205);
            let mut chunks = rt::call!(slf.getattr("iterencode")?, o, _one_shot = true)?;
            frame.line(206);
            if !chunks.is_instance(
                PyTuple::new(py, [&rt::builtin(py, "list")?, &rt::builtin(py, "tuple")?])?.as_any(),
            )? {
                frame.line(207);
                chunks = rt::builtin(py, "list")?.call1((&chunks,))?;
            }
            frame.line(208);
            rt::str(py, "").call_method1("join", (&chunks,))
        })
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
    pub fn iterencode<'py>(
        m: &rt::Module<'py>,
        slf: &Bound<'py, PyAny>,
        o: &Bound<'py, PyAny>,
        _one_shot: &Bound<'py, PyAny>,
    ) -> PyResult<Bound<'py, PyAny>> {
        let py = m.py();
        rt::frame!(m, "JSONEncoder.iterencode", 210, |frame| {
            let markers: Bound<'py, PyAny>;
            let _encoder: Bound<'py, PyAny>;
            let indent: Bound<'py, PyAny>;
            let _iterencode: Bound<'py, PyAny>;
            frame.line(220);
            if slf.getattr("check_circular")?.is_truthy()? {
                markers = PyDict::new(py).into_any();
            } else {
                markers = rt::none(py);
            }
            frame.line(224);
            if slf.getattr("ensure_ascii")?.is_truthy()? {
                frame.line(225);
                _encoder = m.global("encode_basestring_ascii")?;
            } else {
                frame.line(227);
                _encoder = m.global("encode_basestring")?;
            }
            // json/encoder.py:229
            fn floatstr<'py>(
                m: &rt::Module<'py>,
                o: &Bound<'py, PyAny>,
                allow_nan: &Bound<'py, PyAny>,
                _repr: &Bound<'py, PyAny>,
                _inf: &Bound<'py, PyAny>,
                _neginf: &Bound<'py, PyAny>,
            ) -> PyResult<Bound<'py, PyAny>> {
                let py = m.py();
                rt::frame!(
                    m,
                    "JSONEncoder.iterencode.<locals>.floatstr",
                    229,
                    |frame| {
                        let text: Bound<'py, PyAny>;
                        // Check for specials.  Note that this type of test is processor
                        // and/or platform-specific, so do tests which don't depend on the
                        // internals.
                        frame.line(235);
                        if o.ne(o)? {
                            text = rt::str(py, "NaN");
                        } else {
                            frame.line(237);
                            if o.eq(_inf)? {
                                text = rt::str(py, "Infinity");
                            } else {
                                frame.line(239);
                                if o.eq(_neginf)? {
                                    text = rt::str(py, "-Infinity");
                                } else {
                                    frame.line(242);
                                    return _repr.call1((o,));
                                }
                            }
                        }
                        frame.line(244);
                        if !allow_nan.is_truthy()? {
                            frame.line(245);
                            return Err(frame.raise(PyValueError::new_err((rt::str(
                                py,
                                "Out of range float values are not JSON compliant: ",
                            )
                            .add(o.repr()?)?
                            .unbind(),))));
                        }
                        Ok(text)
                    }
                )
            }
            frame.line(229);
            let floatstr = rt::function!(
                m,
                floatstr(
                    o,
                    allow_nan = slf.getattr("allow_nan")?,
                    _repr = rt::builtin(py, "float")?.getattr("__repr__")?,
                    _inf = m.global("INFINITY")?,
                    _neginf = m.global("INFINITY")?.neg()?
                ),
                qualname = "JSONEncoder.iterencode.<locals>.floatstr"
            )?;
            frame.line(252);
            if slf.getattr("indent")?.is_none()
                || slf
                    .getattr("indent")?
                    .is_instance(&rt::builtin(py, "str")?)?
            {
                frame.line(253);
                indent = slf.getattr("indent")?;
            } else {
                frame.line(255);
                indent = rt::str(py, " ").mul(slf.getattr("indent")?)?;
            }
            frame.line(256);
            if _one_shot.is_truthy()? && !m.global("c_make_encoder")?.is_none() {
                frame.line(257);
                _iterencode = m.global("c_make_encoder")?.call1((
                    &markers,
                    slf.getattr("default")?,
                    &_encoder,
                    &indent,
                    slf.getattr("key_separator")?,
                    slf.getattr("item_separator")?,
                    slf.getattr("sort_keys")?,
                    slf.getattr("skipkeys")?,
                    slf.getattr("allow_nan")?,
                ))?;
            } else {
                frame.line(262);
                _iterencode = m.global("_make_iterencode")?.call1((
                    &markers,
                    slf.getattr("default")?,
                    &_encoder,
                    &indent,
                    &floatstr,
                    slf.getattr("key_separator")?,
                    slf.getattr("item_separator")?,
                    slf.getattr("sort_keys")?,
                    slf.getattr("skipkeys")?,
                    _one_shot,
                ))?;
            }
            frame.line(266);
            _iterencode.call1((o, 0))
        })
    }
}

/// Executes the body of `json/encoder.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        m.set_global("__doc__", "Implementation of JSONEncoder\n")?;
        // Copied unmodified (below this header) from Lib/json/encoder.py of CPython 3.14.9,
        // licensed under the PSF License Agreement. A steelix test project:
        // tests/test_projects.py runs it transpiled and with CPython and compares.
        frame.line(6);
        m.set_global("re", rt::import(m, "re")?)?;
        match (|| -> PyResult<()> {
            frame.line(9);
            let _json = rt::import_names(m, "_json", &["encode_basestring_ascii"])?;
            m.set_global(
                "c_encode_basestring_ascii",
                rt::import_from(&_json, "encode_basestring_ascii")?,
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                frame.line(11);
                m.set_global("c_encode_basestring_ascii", py.None())?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            frame.line(13);
            let _json = rt::import_names(m, "_json", &["encode_basestring"])?;
            m.set_global(
                "c_encode_basestring",
                rt::import_from(&_json, "encode_basestring")?,
            )?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                frame.line(15);
                m.set_global("c_encode_basestring", py.None())?;
            }
            Err(err) => return Err(err),
        }
        match (|| -> PyResult<()> {
            frame.line(17);
            let _json = rt::import_names(m, "_json", &["make_encoder"])?;
            m.set_global("c_make_encoder", rt::import_from(&_json, "make_encoder")?)?;
            Ok(())
        })() {
            Ok(()) => {}
            Err(err) if err.is_instance_of::<PyImportError>(py) => {
                let _handling = rt::Handling::new(&frame.catch(err));
                frame.line(19);
                m.set_global("c_make_encoder", py.None())?;
            }
            Err(err) => return Err(err),
        }
        frame.line(21);
        m.set_global(
            "ESCAPE",
            m.global("re")?
                .call_method1("compile", ("[\\x00-\\x1f\\\\\"\\b\\f\\n\\r\\t]",))?,
        )?;
        frame.line(22);
        m.set_global(
            "ESCAPE_ASCII",
            m.global("re")?
                .call_method1("compile", ("([\\\\\"]|[^\\ -~])",))?,
        )?;
        frame.line(23);
        m.set_global(
            "HAS_UTF8",
            m.global("re")?
                .call_method1("compile", (rt::bytes(py, b"[\x80-\xff]"),))?,
        )?;
        frame.line(24);
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
        frame.line(33);
        for item in rt::builtin(py, "range")?.call1((32,))?.try_iter()? {
            m.set_global("i", item?)?;
            frame.line(34);
            m.global("ESCAPE_DCT")?.getattr("setdefault")?.call1((
                rt::builtin(py, "chr")?.call1((m.global("i")?,))?,
                rt::str(py, "\\u{0:04x}")
                    .getattr("format")?
                    .call1((m.global("i")?,))?,
            ))?;
            py.check_signals()?;
            frame.line(33);
        }
        // ESCAPE_DCT.setdefault(chr(i), '\\u%04x' % (i,))
        frame.line(36);
        m.del_global("i")?;
        frame.line(38);
        m.set_global("INFINITY", rt::builtin(py, "float")?.call1(("inf",))?)?;
        rt::def!(
            m,
            py_encode_basestring(s),
            doc = "Return a JSON representation of a Python string\n\n    "
        )?;
        frame.line(49);
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
            doc = "Return an ASCII-only JSON representation of a Python string\n\n    "
        )?;
        frame.line(75);
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
        frame.line(77);
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
        // json/encoder.py:268, kept as Python: yield at 285:13, yield from at 329:21
        rt::exec_python!(
            m,
            line = 268,
            r#"
def _make_iterencode(markers, _default, _encoder, _indent, _floatstr,
        _key_separator, _item_separator, _sort_keys, _skipkeys, _one_shot,
        ## HACK: hand-optimized bytecode; turn globals into locals
        ValueError=ValueError,
        dict=dict,
        float=float,
        id=id,
        int=int,
        isinstance=isinstance,
        list=list,
        str=str,
        tuple=tuple,
        _intstr=int.__repr__,
    ):

    def _iterencode_list(lst, _current_indent_level):
        if not lst:
            yield '[]'
            return
        if markers is not None:
            markerid = id(lst)
            if markerid in markers:
                raise ValueError("Circular reference detected")
            markers[markerid] = lst
        buf = '['
        if _indent is not None:
            _current_indent_level += 1
            newline_indent = '\n' + _indent * _current_indent_level
            separator = _item_separator + newline_indent
            buf += newline_indent
        else:
            newline_indent = None
            separator = _item_separator
        for i, value in enumerate(lst):
            if i:
                buf = separator
            try:
                if isinstance(value, str):
                    yield buf + _encoder(value)
                elif value is None:
                    yield buf + 'null'
                elif value is True:
                    yield buf + 'true'
                elif value is False:
                    yield buf + 'false'
                elif isinstance(value, int):
                    # Subclasses of int/float may override __repr__, but we still
                    # want to encode them as integers/floats in JSON. One example
                    # within the standard library is IntEnum.
                    yield buf + _intstr(value)
                elif isinstance(value, float):
                    # see comment above for int
                    yield buf + _floatstr(value)
                else:
                    yield buf
                    if isinstance(value, (list, tuple)):
                        chunks = _iterencode_list(value, _current_indent_level)
                    elif isinstance(value, dict):
                        chunks = _iterencode_dict(value, _current_indent_level)
                    else:
                        chunks = _iterencode(value, _current_indent_level)
                    yield from chunks
            except GeneratorExit:
                raise
            except BaseException as exc:
                exc.add_note(f'when serializing {type(lst).__name__} item {i}')
                raise
        if newline_indent is not None:
            _current_indent_level -= 1
            yield '\n' + _indent * _current_indent_level
        yield ']'
        if markers is not None:
            del markers[markerid]

    def _iterencode_dict(dct, _current_indent_level):
        if not dct:
            yield '{}'
            return
        if markers is not None:
            markerid = id(dct)
            if markerid in markers:
                raise ValueError("Circular reference detected")
            markers[markerid] = dct
        yield '{'
        if _indent is not None:
            _current_indent_level += 1
            newline_indent = '\n' + _indent * _current_indent_level
            item_separator = _item_separator + newline_indent
        else:
            newline_indent = None
            item_separator = _item_separator
        first = True
        if _sort_keys:
            items = sorted(dct.items())
        else:
            items = dct.items()
        for key, value in items:
            if isinstance(key, str):
                pass
            # JavaScript is weakly typed for these, so it makes sense to
            # also allow them.  Many encoders seem to do something like this.
            elif isinstance(key, float):
                # see comment for int/float in _make_iterencode
                key = _floatstr(key)
            elif key is True:
                key = 'true'
            elif key is False:
                key = 'false'
            elif key is None:
                key = 'null'
            elif isinstance(key, int):
                # see comment for int/float in _make_iterencode
                key = _intstr(key)
            elif _skipkeys:
                continue
            else:
                raise TypeError(f'keys must be str, int, float, bool or None, '
                                f'not {key.__class__.__name__}')
            if first:
                first = False
                if newline_indent is not None:
                    yield newline_indent
            else:
                yield item_separator
            yield _encoder(key)
            yield _key_separator
            try:
                if isinstance(value, str):
                    yield _encoder(value)
                elif value is None:
                    yield 'null'
                elif value is True:
                    yield 'true'
                elif value is False:
                    yield 'false'
                elif isinstance(value, int):
                    # see comment for int/float in _make_iterencode
                    yield _intstr(value)
                elif isinstance(value, float):
                    # see comment for int/float in _make_iterencode
                    yield _floatstr(value)
                else:
                    if isinstance(value, (list, tuple)):
                        chunks = _iterencode_list(value, _current_indent_level)
                    elif isinstance(value, dict):
                        chunks = _iterencode_dict(value, _current_indent_level)
                    else:
                        chunks = _iterencode(value, _current_indent_level)
                    yield from chunks
            except GeneratorExit:
                raise
            except BaseException as exc:
                exc.add_note(f'when serializing {type(dct).__name__} item {key!r}')
                raise
        if not first and newline_indent is not None:
            _current_indent_level -= 1
            yield '\n' + _indent * _current_indent_level
        yield '}'
        if markers is not None:
            del markers[markerid]

    def _iterencode(o, _current_indent_level):
        if isinstance(o, str):
            yield _encoder(o)
        elif o is None:
            yield 'null'
        elif o is True:
            yield 'true'
        elif o is False:
            yield 'false'
        elif isinstance(o, int):
            # see comment for int/float in _make_iterencode
            yield _intstr(o)
        elif isinstance(o, float):
            # see comment for int/float in _make_iterencode
            yield _floatstr(o)
        elif isinstance(o, (list, tuple)):
            yield from _iterencode_list(o, _current_indent_level)
        elif isinstance(o, dict):
            yield from _iterencode_dict(o, _current_indent_level)
        else:
            if markers is not None:
                markerid = id(o)
                if markerid in markers:
                    raise ValueError("Circular reference detected")
                markers[markerid] = o
            newobj = _default(o)
            try:
                yield from _iterencode(newobj, _current_indent_level)
            except GeneratorExit:
                raise
            except BaseException as exc:
                exc.add_note(f'when serializing {type(o).__name__} object')
                raise
            if markers is not None:
                del markers[markerid]
    return _iterencode
"#
        )?;
        Ok(())
    })
}
