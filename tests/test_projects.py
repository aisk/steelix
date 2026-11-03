"""Multi-module applications (`tests/projects/`, design §7).

Each project is transpiled from its entry script (or `-m module`) together
with the project modules it imports. The generated crate is a golden
snapshot (`tests/golden/projects/<name>/`, every file under `src/`, with the
repository's path in `main.rs` replaced by `$ROOT`), builds without warnings
and clippy lints, and the program behaves like CPython running the same
sources: stdout, exit code, and full tracebacks with file names.
"""

from __future__ import annotations

import shutil
from dataclasses import dataclass, field
from pathlib import Path

import pytest
from conftest import (
    DEV_PROFILE,
    E2E_DIR,
    GOLDEN_DIR,
    PYTHON,
    ROOT,
    RUNTIME,
    Workspace,
    compiler_warnings,
)
from test_e2e import Outcome, run

from steelix.emit.crate import CrateOptions, Mode, emit_project
from steelix.project import Project, find_project

PROJECTS_DIR = ROOT / "tests" / "projects"
PROJECT_GOLDEN = GOLDEN_DIR / "projects"


@dataclass(frozen=True)
class Case:
    name: str
    directory: str
    script: str | None = None
    module: str | None = None
    include: tuple[str, ...] = ()
    exclude: tuple[str, ...] = ()
    commands: list[list[str]] = field(default_factory=lambda: [[]])

    @property
    def root(self) -> Path:
        return PROJECTS_DIR / self.directory

    def project(self) -> Project:
        return find_project(
            self.root / self.script if self.script else None,
            module=self.module,
            root=self.root,
            include=[str(self.root / item) for item in self.include],
            exclude=self.exclude,
        )

    def python_command(self) -> list[str]:
        if self.script is not None:
            return [str(PYTHON), str(self.root / self.script)]
        return [str(PYTHON), "-m", str(self.module)]


CASES = [
    Case(
        "inventory",
        "inventory",
        script="app.py",
        include=("plugins",),
        exclude=("legacy",),
        commands=[[], ["crash"]],
    ),
    Case("inventory_store", "inventory", module="store", commands=[[], ["crash"]]),
    Case(
        "stdlib_json",
        "stdlib_json",
        script="app.py",
        commands=[
            [],
            ["tool", "--sort-keys", "--indent", "1", "sample.json"],
            ["tool", "--json-lines", "--compact", "lines.jsonl"],
            ["tool", "missing.json"],
            ["crash"],
        ],
    ),
    Case(
        "stdlib_json_main",
        "stdlib_json",
        module="json",
        commands=[
            ["--indent", "1", "sample.json"],
            ["--no-ensure-ascii", "--tab", "sample.json"],
            ["--no-such-option"],
            ["sample.json", "lines.jsonl"],
        ],
    ),
]
BY_NAME = {case.name: case for case in CASES}


def generate(case: Case, out: Path) -> Project:
    project = case.project()
    options = CrateOptions(mode=Mode.BIN, runtime_path=RUNTIME, standalone=False)
    emit_project(project, out, options, name=case.name)
    return project


def generated_sources(crate: Path) -> dict[str, str]:
    """The files under `src/` of a crate, with this checkout's path in
    `main.rs` replaced, by path relative to `src/`."""
    files = {}
    for path in sorted((crate / "src").rglob("*.rs")):
        text = path.read_text()
        if path.name == "main.rs" and path.parent == crate / "src":
            text = text.replace(str(ROOT), "$ROOT")
        files[path.relative_to(crate / "src").as_posix()] = text
    return files


@pytest.mark.parametrize("name", list(BY_NAME))
def test_project_golden(name: str, update_golden: bool, tmp_path: Path) -> None:
    case = BY_NAME[name]
    generate(case, tmp_path / name)
    actual = generated_sources(tmp_path / name)
    golden_dir = PROJECT_GOLDEN / name
    if update_golden or not golden_dir.exists():
        if golden_dir.exists():
            shutil.rmtree(golden_dir)
        for relative, text in actual.items():
            (golden_dir / relative).parent.mkdir(parents=True, exist_ok=True)
            (golden_dir / relative).write_text(text)
        if not update_golden:
            pytest.fail(f"created missing golden {golden_dir}; review and commit it")
        return
    expected = {
        p.relative_to(golden_dir).as_posix(): p.read_text() for p in golden_dir.rglob("*.rs")
    }
    assert sorted(actual) == sorted(expected), "the crate layout changed"
    for relative, text in actual.items():
        assert text == expected[relative], (
            f"{name}/{relative} is out of date; rerun with --update-golden if intended"
        )


@pytest.fixture(scope="module")
def projects_workspace() -> Workspace:
    root = E2E_DIR / "projects"
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    for case in CASES:
        generate(case, root / case.name)
    members = ", ".join(f'"{case.name}"' for case in CASES)
    manifest = f'[workspace]\nresolver = "3"\nmembers = [{members}]\n\n{DEV_PROFILE}'
    (root / "Cargo.toml").write_text(manifest)
    shutil.copyfile(ROOT / "Cargo.lock", root / "Cargo.lock")
    workspace = Workspace(root, [case.name for case in CASES])
    result = workspace.cargo("build", "--workspace", "--message-format=json")
    assert result.returncode == 0, result.stderr + result.stdout
    warnings = compiler_warnings(result.stdout, workspace)
    assert not warnings, "\n".join(warnings)
    return workspace


@pytest.mark.cargo
def test_projects_are_clippy_clean(projects_workspace: Workspace) -> None:
    result = projects_workspace.cargo(
        "clippy", "--workspace", "--keep-going", "--message-format=json"
    )
    lints = compiler_warnings(result.stdout, projects_workspace, ("warning", "error"))
    assert not lints, "\n".join(lints)
    assert result.returncode == 0, result.stderr


def binary(case: Case) -> Path:
    return E2E_DIR / "target" / "debug" / case.name


def without_runpy(outcome: Outcome) -> list[str]:
    """The traceback without the frames of `runpy`, which `python -m` has
    and the transpiled program does not (design §12)."""
    return [line for line in outcome.traceback if not line.startswith('  File "<frozen runpy>"')]


COMMANDS = [(case.name, args) for case in CASES for args in case.commands]


@pytest.mark.e2e
@pytest.mark.parametrize(
    ("name", "args"), COMMANDS, ids=[f"{n} {' '.join(a)}".strip() for n, a in COMMANDS]
)
def test_project_matches_python(
    name: str, args: list[str], projects_workspace: Workspace, monkeypatch
) -> None:
    case = BY_NAME[name]
    # Neither run writes `__pycache__` into the test sources.
    monkeypatch.setenv("PYTHONDONTWRITEBYTECODE", "1")
    expected = run([*case.python_command(), *args], case.root)
    actual = run([str(binary(case)), *args], case.root)
    assert actual.stdout == expected.stdout
    assert actual.returncode == expected.returncode
    assert without_runpy(actual) == without_runpy(expected)


@pytest.mark.e2e
def test_ext_package_matches_python(tmp_path: Path) -> None:
    """Extension module mode for a package: `PyInit_store` runs the package's
    `__init__`, and its submodules (and the project modules it imports) are
    provided by the importer, relative to the extension module's directory."""
    root = PROJECTS_DIR / "inventory"
    project = find_project(module="store", root=root, run_main=False)
    crate_root = E2E_DIR / "projects-ext"
    if crate_root.exists():
        shutil.rmtree(crate_root)
    options = CrateOptions(mode=Mode.EXT, runtime_path=RUNTIME, standalone=False)
    emit_project(project, crate_root / "store", options)
    manifest = f'[workspace]\nresolver = "3"\nmembers = ["store"]\n\n{DEV_PROFILE}'
    (crate_root / "Cargo.toml").write_text(manifest)
    shutil.copyfile(ROOT / "Cargo.lock", crate_root / "Cargo.lock")
    workspace = Workspace(crate_root, ["store"])
    result = workspace.cargo("build", "--workspace", "--message-format=json", extension=True)
    assert result.returncode == 0, result.stderr + result.stdout
    assert not compiler_warnings(result.stdout, workspace)
    shutil.copyfile(E2E_DIR / "target-ext" / "debug" / "libstore.so", tmp_path / "store.so")
    code = (
        "import store\n"
        "from store.reports import render\n"
        "inventory = store.Inventory()\n"
        "inventory.add(store.Item('kiwi', 2, 3))\n"
        "print(render(inventory))\n"
        "import store.items\n"
        "print(store.items.__name__, store.items.__package__, store.__package__)\n"
        "print(store.__path__[0].endswith('/store'), store.__spec__.parent)\n"
    )
    expected = run([str(PYTHON), "-B", "-c", code], root)
    actual = run([str(PYTHON), "-B", "-c", code], tmp_path)
    assert actual.returncode == expected.returncode == 0, actual.stderr
    assert actual.stdout.replace(str(tmp_path), "<dir>") == expected.stdout.replace(
        str(root), "<dir>"
    )
