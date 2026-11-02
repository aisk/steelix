"""End-to-end differential tests.

Each case is transpiled into an executable (bin mode) and must behave like
`python case.py`: same stdout, same exit code, and for uncaught exceptions the
same final traceback line (the traceback frames themselves differ).
"""

from __future__ import annotations

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

    assert actual.stdout == expected.stdout
    assert actual.returncode == expected.returncode
    if not expected.stderr:
        assert actual.stderr == ""
    elif EXCEPTION_TYPE_ONLY in source.read_text():
        exception_type = expected.last_stderr_line.split(":")[0]
        assert actual.last_stderr_line.split(":")[0] == exception_type
    else:
        assert actual.last_stderr_line == expected.last_stderr_line


def test_ext_module_matches_python(tmp_path: Path) -> None:
    """The extension module mode still builds and imports."""
    workspace = make_workspace("ext", ["fib"], Mode.EXT, from_golden=False)
    result = workspace.cargo("build", "--workspace", extension=True)
    assert result.returncode == 0, result.stderr
    shutil.copyfile(E2E_DIR / "target-ext" / "debug" / "libfib.so", tmp_path / "fib.so")
    expected = run([str(PYTHON), str(CASES_DIR / "fib.py")], tmp_path)
    actual = run([str(PYTHON), "-c", "import fib"], tmp_path)
    assert (actual.stdout, actual.returncode) == (expected.stdout, expected.returncode)
