"""The PoC of native coroutines and generators (design §8, roadmap M13).

`tests/poc_async/src/coroutines.rs` is `coroutines.py` with its coroutines
and generators written by hand the way steelix is meant to generate them
(the transpiler does not do it yet), on top of the runtime's
`rt::Coroutine`. The program must behave like CPython running the Python
file, apart from the differences listed here (design §12).
"""

from __future__ import annotations

import re
import shutil
from pathlib import Path

import pytest
from conftest import E2E_DIR, PYTHON, ROOT, RUNTIME, Workspace, cargo_env, compiler_warnings
from test_e2e import run

from steelix.emit.crate import DEV_PROFILE, CrateOptions, Mode, emit_crate
from steelix.frontend.parse import parse_file

POC = ROOT / "tests" / "poc_async"

# CPython's line on the left is printed as the line on the right, or not at
# all (`None`).
KNOWN_DIFFERENCES: dict[str, str | None] = {
    # Native coroutines are not `types.CoroutineType` (design §12).
    "inspect.iscoroutine True": "inspect.iscoroutine False",
}


@pytest.fixture(scope="module")
def poc_workspace() -> Workspace:
    root = E2E_DIR / "poc_async"
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    options = CrateOptions(mode=Mode.BIN, runtime_path=RUNTIME, standalone=False)
    emit_crate(parse_file(POC / "coroutines.py"), root / "coroutines", options)
    shutil.copytree(POC / "src", root / "coroutines" / "src", dirs_exist_ok=True)
    manifest = f'[workspace]\nresolver = "3"\nmembers = ["coroutines"]\n\n{DEV_PROFILE}'
    (root / "Cargo.toml").write_text(manifest)
    shutil.copyfile(ROOT / "Cargo.lock", root / "Cargo.lock")
    workspace = Workspace(root, ["coroutines"])
    result = workspace.cargo("clippy", "--message-format=json")
    assert result.returncode == 0, result.stderr + result.stdout
    warnings = compiler_warnings(result.stdout, workspace)
    assert not warnings, "\n".join(warnings)
    result = workspace.cargo("build")
    assert result.returncode == 0, result.stderr
    return workspace


def test_kept_python_is_the_source() -> None:
    """The drivers in `src/coroutines.rs` are copied from `coroutines.py` and
    must stay identical, line numbers included."""
    rust = (POC / "src" / "coroutines.rs").read_text()
    source = (POC / "coroutines.py").read_text().splitlines()
    for match in re.finditer(r'line = (\d+),\s*r#"\n(.*?)\n"#', rust, re.DOTALL):
        first = int(match.group(1))
        kept = match.group(2).splitlines()
        assert kept == source[first - 1 : first - 1 + len(kept)], f"line {first}"


@pytest.mark.e2e
@pytest.mark.cargo
def test_poc_matches_cpython(poc_workspace: Workspace) -> None:
    expected = run([str(PYTHON), "coroutines.py"], POC)
    binary = Path(cargo_env(extension=False)["CARGO_TARGET_DIR"]) / "debug" / "coroutines"
    actual = run([str(binary)], POC)
    assert actual.returncode == expected.returncode == 0, actual.stderr
    lines = [KNOWN_DIFFERENCES.get(line, line) for line in expected.stdout.splitlines()]
    assert actual.stdout.splitlines() == [line for line in lines if line is not None]
    for cpython in KNOWN_DIFFERENCES:
        assert cpython in expected.stdout.splitlines(), f"no longer printed: {cpython!r}"
    # The warning for the coroutine that is never awaited, with its location.
    assert actual.stderr == expected.stderr
    assert "coroutine 'echo' was never awaited" in actual.stderr
