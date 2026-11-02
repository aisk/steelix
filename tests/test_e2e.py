"""End-to-end differential tests.

Each case is transpiled into an executable (bin mode) and must behave like
`python case.py`: same stdout, same exit code, and for uncaught exceptions the
same final traceback line (the traceback frames themselves differ).
"""

from __future__ import annotations

import re
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

import pytest
from conftest import CASES, CASES_DIR, E2E_DIR, PYTHON, Workspace, make_workspace

from steelix.emit.crate import Mode

pytestmark = pytest.mark.e2e

# A case may relax the stderr comparison with this marker comment when the
# message is a documented difference (design §12).
EXCEPTION_TYPE_ONLY = "# steelix-test: compare-exception-type-only"
# A case for a documented difference where CPython succeeds and the binary
# raises: the binary prints what CPython prints up to that point, then fails
# with this message (`ADDRESS` stands for an object address).
EXPECT_ERROR = re.compile(r"^# steelix-test: expect-error (.*)$", re.MULTILINE)


@dataclass
class Outcome:
    stdout: str
    stderr: str
    returncode: int

    @property
    def last_stderr_line(self) -> str:
        lines = self.stderr.strip().splitlines()
        return lines[-1] if lines else ""


def run(args: list[str], cwd: Path) -> Outcome:
    proc = subprocess.run(args, cwd=cwd, capture_output=True, text=True, timeout=120, check=False)
    return Outcome(proc.stdout, proc.stderr, proc.returncode)


@pytest.fixture(scope="session")
def bin_workspace() -> Workspace:
    workspace = make_workspace("bin", CASES, Mode.BIN, from_golden=False)
    result = workspace.cargo("build", "--workspace")
    assert result.returncode == 0, result.stderr
    return workspace


@pytest.mark.parametrize("case", CASES)
def test_bin_matches_python(case: str, bin_workspace: Workspace, tmp_path: Path) -> None:
    source = CASES_DIR / f"{case}.py"
    expected = run([str(PYTHON), str(source)], tmp_path)
    binary = E2E_DIR / "target" / "debug" / case
    actual = run([str(binary)], tmp_path)

    text = source.read_text()
    if match := EXPECT_ERROR.search(text):
        assert expected.stdout.startswith(actual.stdout), actual.stderr
        assert actual.returncode == 1
        pattern = re.escape(match.group(1)).replace("ADDRESS", "0x[0-9a-f]+")
        assert re.fullmatch(pattern, actual.last_stderr_line), actual.stderr
        return
    assert actual.stdout == expected.stdout
    assert actual.returncode == expected.returncode
    if not expected.stderr:
        assert actual.stderr == ""
    elif EXCEPTION_TYPE_ONLY in text:
        exception_type = expected.last_stderr_line.split(":")[0]
        assert actual.last_stderr_line.split(":")[0] == exception_type
    else:
        assert actual.last_stderr_line == expected.last_stderr_line


UNCAUGHT = [case for case in CASES if "uncaught" in case or case == "typed_receivers"]


@pytest.mark.parametrize("case", UNCAUGHT)
def test_bin_output_order_matches_python(
    case: str, bin_workspace: Workspace, tmp_path: Path
) -> None:
    """With stdout and stderr going to the same file, everything the program
    printed comes before the traceback, as with CPython (which flushes
    `sys.stdout` before printing the exception)."""

    def combined(args: list[str]) -> list[str]:
        proc = subprocess.run(
            args, cwd=tmp_path, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True
        )
        assert proc.returncode != 0
        return proc.stdout.splitlines()

    expected = combined([str(PYTHON), str(CASES_DIR / f"{case}.py")])
    actual = combined([str(E2E_DIR / "target" / "debug" / case)])
    printed = expected[: expected.index("Traceback (most recent call last):")]
    assert printed
    assert actual[: len(printed)] == printed
    assert actual[-1] == expected[-1]


def traceback_frames(stderr: str) -> list[str]:
    """The `File "..", line N, in name` lines of a traceback and the source
    line after each."""
    lines = stderr.splitlines()
    return [
        f"{line.strip()} | {lines[i + 1].strip()}"
        for i, line in enumerate(lines)
        if line.startswith('  File "') and i + 1 < len(lines)
    ]


def test_python_frames_have_original_locations(bin_workspace: Workspace, tmp_path: Path) -> None:
    """Code kept as Python (mixed mode) runs with its file name and original
    line numbers: its traceback frames are CPython's. The frames of native
    functions are missing (design §12)."""
    case = "mixed_uncaught"
    expected = run([str(PYTHON), str(CASES_DIR / f"{case}.py")], tmp_path)
    actual = run([str(E2E_DIR / "target" / "debug" / case)], tmp_path)
    frames = traceback_frames(actual.stderr)
    assert frames
    assert frames == traceback_frames(expected.stderr)[-len(frames) :]
    assert "line 9, in python_kept" in frames[-1]


def test_panic_exits_like_an_uncaught_exception(tmp_path: Path) -> None:
    """A Rust panic, e.g. in a function rewritten by hand, is reported like a
    Python exception and exits with 1 rather than Rust's 101."""
    source = tmp_path / "panicking.py"
    source.write_text("print('before')\n")
    workspace = make_workspace(
        "panic", ["panicking"], Mode.BIN, from_golden=False, source_dir=tmp_path
    )
    module = workspace.root / "panicking" / "src" / "panicking.rs"
    module.write_text(
        "use pyo3::prelude::*;\n"
        "use steelix_runtime as rt;\n"
        "pub fn exec_module(m: &rt::Module<'_>) -> PyResult<()> {\n"
        '    rt::builtins::print(m.py(), ("before",))?;\n'
        "    let values: Vec<i32> = Vec::new();\n"
        "    rt::builtins::print(m.py(), (values[3],))?;\n"
        "    Ok(())\n"
        "}\n"
    )
    result = workspace.cargo("build", "--workspace")
    assert result.returncode == 0, result.stderr
    actual = run([str(E2E_DIR / "target" / "debug" / "panicking")], tmp_path)
    assert actual.returncode == 1
    assert actual.stdout == "before\n"
    assert actual.last_stderr_line.startswith("pyo3_runtime.PanicException: index out of bounds")


def test_ext_module_matches_python(tmp_path: Path) -> None:
    """The extension module mode still builds and imports."""
    workspace = make_workspace("ext", ["fib"], Mode.EXT, from_golden=False)
    result = workspace.cargo("build", "--workspace", extension=True)
    assert result.returncode == 0, result.stderr
    shutil.copyfile(E2E_DIR / "target-ext" / "debug" / "libfib.so", tmp_path / "fib.so")
    expected = run([str(PYTHON), str(CASES_DIR / "fib.py")], tmp_path)
    actual = run([str(PYTHON), "-c", "import fib"], tmp_path)
    assert (actual.stdout, actual.returncode) == (expected.stdout, expected.returncode)
