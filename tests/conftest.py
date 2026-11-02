"""Shared fixtures: golden updating and building generated crates with cargo."""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

import pytest

from steelix.emit.crate import DEV_PROFILE, CrateOptions, Mode, emit_crate, rust_module_name
from steelix.frontend.parse import parse_file

ROOT = Path(__file__).resolve().parents[1]
CASES_DIR = ROOT / "tests" / "cases"
CORPUS_DIR = ROOT / "tests" / "corpus"
GOLDEN_DIR = ROOT / "tests" / "golden"
RUNTIME = ROOT / "crates" / "steelix-runtime"
PYTHON = ROOT / ".venv" / "bin" / "python"
E2E_DIR = ROOT / "target" / "e2e"

CASES = sorted(p.stem for p in CASES_DIR.glob("*.py"))


def pytest_addoption(parser: pytest.Parser) -> None:
    parser.addoption(
        "--update-golden",
        action="store_true",
        default=False,
        help="rewrite tests/golden/*.rs from the current transpiler output",
    )


@pytest.fixture
def update_golden(request: pytest.FixtureRequest) -> bool:
    return bool(request.config.getoption("--update-golden"))


def cargo_env(*, extension: bool) -> dict[str, str]:
    """Environment for building generated crates.

    `PYO3_PYTHON` must be explicit: `.cargo/config.toml` of this repository does
    not apply to crates generated elsewhere. Extension modules and executables
    use separate target directories because `PYO3_BUILD_EXTENSION_MODULE`
    changes how pyo3-ffi links, which would otherwise rebuild PyO3 each time.
    """
    env = dict(os.environ)
    env["PYO3_PYTHON"] = str(PYTHON)
    env["CARGO_TARGET_DIR"] = str(E2E_DIR / ("target-ext" if extension else "target"))
    env.pop("PYO3_BUILD_EXTENSION_MODULE", None)
    if extension:
        env["PYO3_BUILD_EXTENSION_MODULE"] = "1"
    return env


@dataclass
class Workspace:
    root: Path
    members: list[str]

    def cargo(self, *args: str, extension: bool = False) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            ["cargo", *args],
            cwd=self.root,
            env=cargo_env(extension=extension),
            capture_output=True,
            text=True,
            check=False,
        )


def make_workspace(
    name: str,
    cases: list[str],
    mode: Mode,
    *,
    from_golden: bool,
    source_dir: Path = CASES_DIR,
) -> Workspace:
    """Generate one crate per case under a shared cargo workspace.

    With `from_golden` the generated module file is replaced by the golden
    snapshot, so that the snapshots themselves are what gets compiled.
    """
    root = E2E_DIR / name
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    options = CrateOptions(mode=mode, runtime_path=RUNTIME, standalone=False)
    for case in cases:
        parsed = parse_file(source_dir / f"{case}.py")
        crate = root / case
        emit_crate(parsed, crate, options)
        if from_golden:
            module_file = rust_module_name(case).removeprefix("r#") + ".rs"
            shutil.copyfile(GOLDEN_DIR / f"{case}.rs", crate / "src" / module_file)
    members = ", ".join(f'"{case}"' for case in cases)
    manifest = f'[workspace]\nresolver = "3"\nmembers = [{members}]\n\n{DEV_PROFILE}'
    (root / "Cargo.toml").write_text(manifest)
    # Reuse the repository lock file so PyO3 resolves to the same version offline.
    shutil.copyfile(ROOT / "Cargo.lock", root / "Cargo.lock")
    return Workspace(root, cases)


def compiler_warnings(cargo_json_output: str, workspace: Workspace) -> list[str]:
    """Rendered warnings that rustc reported for the workspace's own crates."""
    warnings = []
    for line in cargo_json_output.splitlines():
        if not line.startswith("{"):
            continue
        message = json.loads(line)
        if message.get("reason") != "compiler-message":
            continue
        manifest = message.get("manifest_path", "")
        if not manifest.startswith(str(workspace.root)):
            continue
        diagnostic = message["message"]
        if diagnostic["level"] == "warning":
            warnings.append(diagnostic["rendered"])
    return warnings
