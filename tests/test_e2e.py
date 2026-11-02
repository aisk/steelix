"""End-to-end differential tests.

Each case is transpiled into an executable (bin mode) and must behave like
`python case.py`: same stdout, same exit code, and for uncaught exceptions the
same traceback, native functions included (design §9). Native frames have no
column positions, so the `^^^^` lines under source lines are left out of the
comparison, and so are the `SyntaxWarning`s CPython prints when it compiles
the file.
"""

from __future__ import annotations

import re
import shutil
import signal
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
# Documented differences in stdout (design §12): CPython's line on the left
# is printed as the pattern on the right by the transpiled program. A change
# that removes the difference (such as frames for native functions) fails
# the test, so that the entry gets removed.
KNOWN_DIFFERENCES: dict[str, dict[str, str]] = {
    "frames_logging": {
        "WARNING frames_logging.py:16 step: native step 1": (
            r"WARNING __init__\.py:\d+ warning: native step 1"
        ),
        # No Python frame at all.
        "UserWarning frames_logging.py:36: from native code": (
            r"UserWarning <sys>:0: from native code"
        ),
        # The kept function's call of the native one.
        "UserWarning frames_logging.py:36: from native code, called by kept code": (
            r"UserWarning frames_logging\.py:48: from native code, called by kept code"
        ),
    },
}
# The lines marking the failing expression under a source line.
CARETS = re.compile(r"\s*[~^]+\s*")
# A case that runs until it is interrupted: the test sends SIGINT once the
# program has printed its first line.
INTERRUPT = "# steelix-test: interrupt after the first line of output"


@dataclass
class Outcome:
    stdout: str
    stderr: str
    returncode: int

    @property
    def traceback(self) -> list[str]:
        """stderr without caret lines and compile time `SyntaxWarning`s."""
        lines = self.stderr.splitlines()
        out = []
        skip = False
        for line in lines:
            if skip:
                skip = False
                continue
            if re.match(r"^\S.*:\d+: SyntaxWarning: ", line):
                skip = True  # and the source line after it
                continue
            if CARETS.fullmatch(line):
                continue
            out.append(line)
        return out

    @property
    def last_stderr_line(self) -> str:
        lines = self.stderr.strip().splitlines()
        return lines[-1] if lines else ""


def run(args: list[str], cwd: Path) -> Outcome:
    proc = subprocess.run(args, cwd=cwd, capture_output=True, text=True, timeout=120, check=False)
    return Outcome(proc.stdout, proc.stderr, proc.returncode)


def run_interrupted(args: list[str], cwd: Path) -> Outcome:
    """Run `args`, and press Ctrl-C (send SIGINT) after its first line of output."""
    proc = subprocess.Popen(
        args, cwd=cwd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True
    )
    assert proc.stdout is not None
    first = proc.stdout.readline()
    proc.send_signal(signal.SIGINT)
    try:
        stdout, stderr = proc.communicate(timeout=30)
    except subprocess.TimeoutExpired:
        proc.kill()
        proc.communicate()
        pytest.fail(f"{args[0]} kept running after SIGINT")
    return Outcome(first + stdout, stderr, proc.returncode)


@pytest.fixture(scope="session")
def bin_workspace() -> Workspace:
    workspace = make_workspace("bin", CASES, Mode.BIN, from_golden=False)
    result = workspace.cargo("build", "--workspace")
    assert result.returncode == 0, result.stderr
    return workspace


@pytest.mark.parametrize("case", CASES)
def test_bin_matches_python(case: str, bin_workspace: Workspace, tmp_path: Path) -> None:
    source = CASES_DIR / f"{case}.py"
    text = source.read_text()
    runner = run_interrupted if INTERRUPT in text else run
    expected = runner([str(PYTHON), str(source)], tmp_path)
    binary = E2E_DIR / "target" / "debug" / case
    actual = runner([str(binary)], tmp_path)
    if runner is run_interrupted:
        # Killed by SIGINT after the `KeyboardInterrupt` traceback, as CPython does.
        assert expected.returncode == -signal.SIGINT
        assert expected.last_stderr_line == "KeyboardInterrupt"
    if match := EXPECT_ERROR.search(text):
        assert expected.stdout.startswith(actual.stdout), actual.stderr
        assert actual.returncode == 1
        pattern = re.escape(match.group(1)).replace("ADDRESS", "0x[0-9a-f]+")
        assert re.fullmatch(pattern, actual.last_stderr_line), actual.stderr
        return
    if case in KNOWN_DIFFERENCES:
        assert_known_differences(expected.stdout, actual.stdout, KNOWN_DIFFERENCES[case])
    else:
        assert actual.stdout == expected.stdout
    assert actual.returncode == expected.returncode
    if EXCEPTION_TYPE_ONLY in text:
        exception_type = expected.last_stderr_line.split(":")[0]
        assert actual.last_stderr_line.split(":")[0] == exception_type
    else:
        assert actual.traceback == expected.traceback


def assert_known_differences(expected: str, actual: str, differences: dict[str, str]) -> None:
    """`actual` is `expected`, except for the lines in `differences`, which
    must be there and match their patterns instead."""
    expected_lines, actual_lines = expected.splitlines(), actual.splitlines()
    assert len(actual_lines) == len(expected_lines), actual
    assert set(differences) <= set(expected_lines)
    for want, got in zip(expected_lines, actual_lines, strict=True):
        if want in differences:
            assert re.fullmatch(differences[want], got), f"{got!r} (CPython: {want!r})"
        else:
            assert got == want


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


def test_tracebacks_have_native_and_python_frames(bin_workspace: Workspace, tmp_path: Path) -> None:
    """Code kept as Python (mixed mode) runs with its file name and original
    line numbers, and native functions add frames of their own (design §9):
    the traceback has CPython's frames, with their source lines."""
    case = "mixed_uncaught"
    expected = run([str(PYTHON), str(CASES_DIR / f"{case}.py")], tmp_path)
    actual = run([str(E2E_DIR / "target" / "debug" / case)], tmp_path)
    frames = traceback_frames(actual.stderr)
    assert frames == traceback_frames(expected.stderr)
    assert "line 9, in python_kept" in frames[-1]
    assert any(", in native |" in frame for frame in frames)


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
