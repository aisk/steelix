"""Questions about `case` patterns that several analyses ask."""

from __future__ import annotations

import ast


def pattern_captures(pattern: ast.pattern) -> list[str]:
    """The names a `case` pattern binds, in the order CPython stores them:
    a capture after the captures inside its subpattern (`[x] as y`), the
    `**rest` of a mapping last."""
    match pattern:
        case ast.MatchAs(sub, name):
            inner = [] if sub is None else pattern_captures(sub)
            return inner + ([name] if name is not None else [])
        case ast.MatchStar(name):
            return [name] if name is not None else []
        case ast.MatchSequence(patterns):
            return [n for sub in patterns for n in pattern_captures(sub)]
        case ast.MatchMapping(_, patterns, rest):
            names = [n for sub in patterns for n in pattern_captures(sub)]
            return names + ([rest] if rest is not None else [])
        case ast.MatchClass(_, patterns, _, kwd_patterns):
            return [n for sub in [*patterns, *kwd_patterns] for n in pattern_captures(sub)]
        case ast.MatchOr(alternatives):
            return pattern_captures(alternatives[0])
    return []


def irrefutable(pattern: ast.pattern) -> bool:
    """Whether a pattern matches every subject: `_`, a capture, or an OR
    pattern with such an alternative."""
    match pattern:
        case ast.MatchAs(None):
            return True
        case ast.MatchAs(sub):
            return sub is not None and irrefutable(sub)
        case ast.MatchOr(alternatives):
            return any(irrefutable(alt) for alt in alternatives)
    return False
