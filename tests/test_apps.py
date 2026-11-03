"""Real applications we did not write (`tests/apps/`), in mixed mode.

They are command line tools from CPython's standard library, copied
unmodified. Whatever steelix can not lower yet stays Python source (design
§13), and each tool must behave exactly like CPython running the same copy.
"""

from __future__ import annotations

import pytest
from conftest import APPS_DIR, E2E_DIR, PYTHON, Workspace, compiler_warnings, make_workspace
from test_e2e import run, traceback_frames

from steelix.emit.crate import Mode

pytestmark = [pytest.mark.e2e, pytest.mark.cargo]

DATA = APPS_DIR / "data"

# Command lines per tool, run in `tests/apps/data`. Calendars name a year so
# that the output does not depend on today's date.
COMMANDS: dict[str, list[list[str]]] = {
    "stdlib_calendar": [
        ["2024"],
        ["2025", "2"],
        ["-f", "6", "-w", "3", "-l", "2", "-s", "4", "-m", "4", "2024"],
        ["-L", "C", "2025", "12"],
        ["-t", "html", "-c", "style.css", "2024"],
        ["-t", "html", "2024", "1"],
        ["2025", "13"],
        ["year"],
    ],
    "stdlib_base64": [
        ["-e", "hello.txt"],
        ["-d", "hello.b64"],
        ["-u", "hello.b64"],
        ["-t"],
        ["-d", "missing.b64"],
    ],
    "stdlib_tabnanny": [
        ["bad_indent.txt"],
        ["-v", "small.py"],
        ["-q", "bad_indent.txt", "small.py"],
        ["-v", "."],
        [],
    ],
    "stdlib_tokenize": [
        ["small.py"],
        ["-e", "small.py"],
        ["sample.json"],
        ["bad_indent.txt"],
    ],
    "stdlib_ast": [
        ["small.py"],
        ["-a", "--indent", "2", "small.py"],
        ["-m", "eval", "sample.json"],
        ["--feature-version", "3.8", "--optimize", "1", "--show-empty", "small.py"],
        ["bad_indent.txt"],
    ],
    "stdlib_json_tool": [
        ["sample.json"],
        ["--sort-keys", "--indent", "1", "sample.json"],
        ["--compact", "--no-ensure-ascii", "sample.json"],
        ["--json-lines", "--tab", "lines.jsonl"],
        ["--json-lines", "hello.txt"],
        ["missing.json"],
    ],
}

CASES = [(app, args) for app, commands in COMMANDS.items() for args in commands]


@pytest.fixture(scope="session")
def apps_workspace() -> Workspace:
    workspace = make_workspace(
        "apps",
        list(COMMANDS),
        Mode.BIN,
        from_golden=False,
        source_dir=APPS_DIR,
        traceback_lines=True,
    )
    result = workspace.cargo("build", "--workspace", "--message-format=json")
    assert result.returncode == 0, result.stderr + result.stdout
    warnings = compiler_warnings(result.stdout, workspace)
    assert not warnings, "\n".join(warnings)
    return workspace


@pytest.mark.parametrize(("app", "args"), CASES, ids=[f"{a} {' '.join(c)}" for a, c in CASES])
def test_app_matches_python(app: str, args: list[str], apps_workspace: Workspace) -> None:
    expected = run([str(PYTHON), str(APPS_DIR / f"{app}.py"), *args], DATA)
    actual = run([str(E2E_DIR / "target" / "debug" / app), *args], DATA)
    assert actual.stdout == expected.stdout
    assert actual.returncode == expected.returncode
    if "Traceback" not in expected.stderr:
        assert actual.stderr == expected.stderr
        return
    # Tracebacks have the frames of native functions too (design §9). A
    # native frame shows only the first line of a statement that spans
    # several, so the frames are compared by their first source line.
    assert traceback_frames(actual.stderr) == traceback_frames(expected.stderr)
    assert actual.last_stderr_line == expected.last_stderr_line
    before = expected.stderr.split("Traceback (most recent call last):")[0]
    assert actual.stderr.startswith(before)
