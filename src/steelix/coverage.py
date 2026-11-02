"""How much of a module is native Rust and what keeps the rest Python.

`steelix transpile` prints a short summary, and `--report FILE` writes the
JSON form, which can be merged across modules to decide what to support next.
"""

from __future__ import annotations

import ast
from collections import Counter
from dataclasses import dataclass, field

from steelix.frontend.fallback import (
    FallbackPlan,
    first_line,
    last_line,
    outermost_functions,
)
from steelix.frontend.subset import Issue


@dataclass
class Unit:
    """A function or a module level statement, counted as native or not."""

    kind: str
    """`function` (a module level `def`, or a method) or `statement`."""
    name: str
    line: int
    native: bool
    blockers: list[str] = field(default_factory=list)
    """Distinct unsupported features that keep it Python, in source order."""


@dataclass
class FeatureStats:
    feature: str
    occurrences: int = 0
    functions: int = 0
    """Functions it appears in (or, for classes, that it contains)."""
    only: int = 0
    """Functions that only this feature keeps Python: supporting it makes them native."""
    statements: int = 0
    """Module level statements other than functions it appears in."""
    locations: list[str] = field(default_factory=list)


@dataclass
class Coverage:
    filename: str
    units: list[Unit]
    features: list[FeatureStats]
    lines_total: int
    lines_python: int

    def count(self, kind: str) -> tuple[int, int]:
        """`(native, total)` of the units of `kind`."""
        units = [u for u in self.units if u.kind == kind]
        return sum(u.native for u in units), len(units)

    def to_json(self) -> dict:
        functions = self.count("function")
        statements = self.count("statement")
        return {
            "module": self.filename,
            "functions": {"native": functions[0], "total": functions[1]},
            "statements": {"native": statements[0], "total": statements[1]},
            "lines": {
                "native": self.lines_total - self.lines_python,
                "total": self.lines_total,
            },
            "features": [
                {
                    "feature": f.feature,
                    "occurrences": f.occurrences,
                    "functions": f.functions,
                    "only": f.only,
                    "statements": f.statements,
                    "locations": f.locations,
                }
                for f in self.features
            ],
            "python": [
                {"kind": u.kind, "name": u.name, "line": u.line, "features": u.blockers}
                for u in self.units
                if not u.native
            ],
        }

    def summary(self, verbose: bool = False) -> str:
        native_fns, total_fns = self.count("function")
        native_stmts, total_stmts = self.count("statement")
        lines = [
            f"{self.filename}: native {_ratio(native_fns, total_fns)} functions,"
            f" {_ratio(native_stmts, total_stmts)} module statements,"
            f" {_ratio(self.lines_total - self.lines_python, self.lines_total)} lines"
        ]
        if self.features:
            width = max(len(f.feature) for f in self.features)
            lines.append(
                f"  {'kept as Python by':<{width}}  functions  only  statements  occurrences"
            )
            for f in self.features:
                lines.append(
                    f"  {f.feature:<{width}}  {f.functions:>9}  {f.only:>4}"
                    f"  {f.statements:>10}  {f.occurrences:>11}"
                )
                if verbose:
                    lines += [f"      {where}" for where in f.locations]
        return "\n".join(lines)


def _ratio(part: int, total: int) -> str:
    percent = f" ({100 * part / total:.0f}%)" if total else ""
    return f"{part}/{total}{percent}"


def measure(tree: ast.Module, plan: FallbackPlan, filename: str) -> Coverage:
    units: list[Unit] = []
    features: dict[str, FeatureStats] = {}

    def stats(feature: str) -> FeatureStats:
        return features.setdefault(feature, FeatureStats(feature))

    lines_python = 0
    for stmt in tree.body:
        issues = plan.python.get(stmt, [])
        native = stmt not in plan.python
        if not native:
            lines_python += last_line(stmt) - first_line(stmt) + 1
        for issue in issues:
            entry = stats(issue.feature)
            entry.occurrences += 1
            entry.locations.append(str(issue.location))
        functions = outermost_functions(stmt)
        for fn in functions:
            blockers = _blockers(fn, issues)
            units.append(Unit("function", _qualname(stmt, fn), fn.lineno, native, blockers))
            for feature in blockers:
                stats(feature).functions += 1
            if len(blockers) == 1:
                stats(blockers[0]).only += 1
        if not isinstance(stmt, ast.FunctionDef | ast.AsyncFunctionDef):
            blockers = list(dict.fromkeys(i.feature for i in issues if i.owner is None))
            name = stmt.name if isinstance(stmt, ast.ClassDef) else type(stmt).__name__
            units.append(Unit("statement", name, first_line(stmt), native, blockers))
            for feature in blockers:
                stats(feature).statements += 1

    ranked = sorted(features.values(), key=lambda f: (-f.functions, -f.only, -f.occurrences))
    return Coverage(filename, units, ranked, _code_lines(tree), lines_python)


def _blockers(fn: ast.FunctionDef | ast.AsyncFunctionDef, issues: list[Issue]) -> list[str]:
    """What keeps `fn` Python: its own unsupported constructs, and those of
    the statement around it (a class, a `try`) outside of any function."""
    return list(dict.fromkeys(i.feature for i in issues if i.owner is fn or i.owner is None))


def _qualname(stmt: ast.stmt, fn: ast.FunctionDef | ast.AsyncFunctionDef) -> str:
    """`f`, or `Class.method` for methods."""
    if fn is stmt:
        return fn.name
    path: list[str] = []

    def find(node: ast.AST, prefix: list[str]) -> bool:
        for child in ast.iter_child_nodes(node):
            if child is fn:
                path.extend(prefix)
                return True
            inner = [*prefix, child.name] if isinstance(child, ast.ClassDef) else prefix
            if isinstance(child, ast.stmt | ast.excepthandler | ast.match_case) and find(
                child, inner
            ):
                return True
        return False

    find(ast.Module([stmt], []), [])
    return ".".join([*path, fn.name])


def _code_lines(tree: ast.Module) -> int:
    return sum(last_line(stmt) - first_line(stmt) + 1 for stmt in tree.body)


def merge(reports: list[dict]) -> list[dict]:
    """Feature counts summed over several JSON reports, ranked like one."""
    totals: dict[str, Counter[str]] = {}
    for report in reports:
        for f in report["features"]:
            counts = totals.setdefault(f["feature"], Counter())
            for key in ("occurrences", "functions", "only", "statements"):
                counts[key] += f[key]
    ranked = sorted(
        totals.items(), key=lambda item: (-item[1]["functions"], -item[1]["only"], item[0])
    )
    return [{"feature": feature, **counts} for feature, counts in ranked]
