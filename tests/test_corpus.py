"""Real-world shaped scripts (`tests/corpus/`) using the supported subset.

Unlike `tests/cases/`, which each exercise one feature, these combine many
features the way small applications do, to find what breaks in combination.
They must compile without warnings and, being deterministic, behave like
CPython.
"""

from __future__ import annotations

from pathlib import Path

import pytest
from conftest import CORPUS_DIR, E2E_DIR, PYTHON, Workspace, compiler_warnings, make_workspace
from test_e2e import run

from steelix.emit.crate import Mode

pytestmark = [pytest.mark.e2e, pytest.mark.cargo]

SCRIPTS = sorted(p.stem for p in CORPUS_DIR.glob("*.py"))


@pytest.fixture(scope="session")
def corpus_workspace() -> Workspace:
    workspace = make_workspace(
        "corpus", SCRIPTS, Mode.BIN, from_golden=False, source_dir=CORPUS_DIR
    )
    result = workspace.cargo("build", "--workspace", "--message-format=json")
    assert result.returncode == 0, result.stderr + result.stdout
    warnings = compiler_warnings(result.stdout, workspace)
    assert not warnings, "\n".join(warnings)
    return workspace


@pytest.mark.parametrize("script", SCRIPTS)
def test_corpus_matches_python(script: str, corpus_workspace: Workspace, tmp_path: Path) -> None:
    expected = run([str(PYTHON), str(CORPUS_DIR / f"{script}.py")], tmp_path)
    actual = run([str(E2E_DIR / "target" / "debug" / script)], tmp_path)
    assert expected.returncode == 0, expected.stderr
    assert (actual.stdout, actual.stderr, actual.returncode) == (expected.stdout, "", 0)
