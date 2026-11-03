//! Parts of `module_annotations.py` kept as Python source and run in place by
//! `rt::exec_python!` (steelix does not lower them yet):
//!
//! - line 8, `annassign` statement: annotated assignment at module level
//! - line 16, `annassign` statement: annotated assignment at module level
//! - line 20, `annassign` statement: annotated assignment at module level
//! - line 21, `annassign` statement: annotated assignment at module level
//! - line 24, `annassign` statement: annotated assignment at module level

use pyo3::prelude::*;
use steelix_runtime as rt;
use steelix_runtime::Attributes;

pub mod box_ {
    use super::*;

    // module_annotations.py:12
    /// The body of `class Box`, run in its namespace `ns`.
    pub fn body<'py>(m: &rt::Module<'py>, ns: &rt::Namespace<'py>) -> PyResult<()> {
        Ok(())
    }
}

/// Executes the body of `module_annotations.py` in the module `m`.
pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {
    let py = m.py();
    rt::frame!(m, "<module>", 1, |_| {
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
        m.set_global("annotationlib", rt::import(m, "annotationlib")?)?;
        m.set_global("sys", rt::import(m, "sys")?)?;
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
        rt::builtins::print(py, (m.global("__annotate__")?.call1((1,))?,))?;
        m.set_global("Box", rt::class!(m, "Box"(), box_::body, line = 12)?)?;
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
        if m.global("sys")?.attr("argv")?.len()? > 5 {
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
        rt::builtins::print(
            py,
            (m.global("annotationlib")?
                .attr("get_annotations")?
                .call1((m
                    .global("sys")?
                    .attr("modules")?
                    .get_item(m.global("__name__")?)?,))?,),
        )?;
        rt::builtins::print(
            py,
            (rt::call!(
                m.global("annotationlib")?.attr("get_annotations")?,
                m.global("sys")?
                    .attr("modules")?
                    .get_item(m.global("__name__")?)?,
                format = m.global("annotationlib")?.attr("Format")?.attr("STRING")?
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
        rt::builtins::print(
            py,
            (rt::builtin(py, "sorted")?.call1((m.global("__conditional_annotations__")?,))?,),
        )?;
        rt::builtins::print(py, (m.global("__annotate__")?.call1((1,))?,))?;
        Ok(())
    })
}
