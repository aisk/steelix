//! Parts of `module_annotations.py` kept as Python source and run by
//! `rt::exec_python!` in `exec_module` (steelix does not lower them yet):
//!
//! - line 8, `annassign` statement: annotated assignment at module level
//! - line 12, `class Box`: class definition
//! - line 16, `annassign` statement: annotated assignment at module level
//! - line 20, `annassign` statement: annotated assignment at module level
//! - line 21, `annassign` statement: annotated assignment at module level
//! - line 24, `annassign` statement: annotated assignment at module level

use pyo3::prelude::*;
use steelix_runtime as rt;

/// Executes the body of `module_annotations.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |frame| {
        // The module's `__annotate__`, which CPython defines before running it
        rt::module_annotate(
            m,
            &[
                (8, "A: int"),
                (16, "B: list[\n    str\n]"),
                (20, "C: float"),
                (21, "D: \"Box\""),
                (24, "E: undefined_name"),
            ],
        )?;
        // Module level annotations in several blocks kept as Python make one
        // `__annotate__`, defined before the first statement as in CPython, which
        // only reports the annotations whose assignments have run.
        frame.line(4);
        m.set_global("annotationlib", rt::import(m, "annotationlib")?)?;
        frame.line(5);
        m.set_global("sys", rt::import(m, "sys")?)?;
        frame.line(7);
        rt::builtins::print(py, (m.dir()?.contains("__annotate__")?,))?;
        // module_annotations.py:8, kept as Python: annotated assignment at module level at 8:1
        rt::exec_python!(
            m,
            line = 8,
            first_annotation = 0,
            r#"
A: int = 1
"#
        )?;
        frame.line(9);
        rt::builtins::print(py, (m.global("__annotate__")?.call1((1,))?,))?;
        // module_annotations.py:12, kept as Python: class definition at 12:1
        rt::exec_python!(
            m,
            line = 12,
            r#"
class Box:
    pass
"#
        )?;
        // module_annotations.py:16, kept as Python: annotated assignment at module level at 16:1
        rt::exec_python!(
            m,
            line = 16,
            first_annotation = 1,
            r#"
B: list[
    str
] = []
"#
        )?;
        frame.line(19);
        if m.global("sys")?.getattr("argv")?.len()? > 5 {
            // module_annotations.py:20, kept as Python: annotated assignment at module level at 20:5
            rt::exec_python!(
                m,
                line = 20,
                first_annotation = 2,
                r#"
    C: float
"#
            )?;
        }
        // module_annotations.py:21, kept as Python: annotated assignment at module level at 21:1
        rt::exec_python!(
            m,
            line = 21,
            first_annotation = 3,
            r#"
D: "Box" = Box()
"#
        )?;
        frame.line(22);
        rt::builtins::print(
            py,
            (m.global("annotationlib")?
                .getattr("get_annotations")?
                .call1((m
                    .global("sys")?
                    .getattr("modules")?
                    .get_item(m.global("__name__")?)?,))?,),
        )?;
        frame.line(23);
        rt::builtins::print(
            py,
            (rt::call!(
                m.global("annotationlib")?.getattr("get_annotations")?,
                m.global("sys")?
                    .getattr("modules")?
                    .get_item(m.global("__name__")?)?,
                format = m
                    .global("annotationlib")?
                    .getattr("Format")?
                    .getattr("STRING")?
            )?,),
        )?;
        // module_annotations.py:24, kept as Python: annotated assignment at module level at 24:1
        rt::exec_python!(
            m,
            line = 24,
            first_annotation = 4,
            r#"
E: undefined_name
"#
        )?;
        frame.line(25);
        rt::builtins::print(
            py,
            (rt::builtin(py, "sorted")?.call1((m.global("__conditional_annotations__")?,))?,),
        )?;
        frame.line(26);
        rt::builtins::print(py, (m.global("__annotate__")?.call1((1,))?,))?;
        Ok(())
    })
}
