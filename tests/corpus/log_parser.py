"""State-machine log parser: tracks job lifecycles from a server log using re."""

import re
from collections import defaultdict

LOG = """\
2024-03-01 10:00:00 INFO job=build-1 event=queued
2024-03-01 10:00:05 INFO job=build-1 event=started worker=w1
2024-03-01 10:00:07 INFO job=test-7 event=queued
2024-03-01 10:00:09 WARN job=test-7 event=started worker=w2
2024-03-01 10:01:00 INFO job=build-1 event=finished status=ok
2024-03-01 10:01:30 ERROR job=test-7 event=finished status=failed
2024-03-01 10:02:00 INFO job=deploy-3 event=started worker=w1
garbage line here
2024-03-01 10:03:00 INFO job=lint-2 event=queued
2024-03-01 10:03:10 INFO job=lint-2 event=cancelled
"""

LINE_RE = re.compile(r"^(\S+) (\d\d):(\d\d):(\d\d) (\w+) (.*)$")
KV_RE = re.compile(r"(\w+)=(\S+)")

TRANSITIONS = {
    None: {"queued"},
    "queued": {"started", "cancelled"},
    "started": {"finished"},
}


def parse_line(line):
    m = LINE_RE.match(line)
    if not m:
        return None
    date, hh, mm, ss, level, rest = m.groups()
    fields = dict(KV_RE.findall(rest))
    seconds = int(hh) * 3600 + int(mm) * 60 + int(ss)
    return seconds, level, fields


def run(log_text):
    state = {}
    started_at = {}
    durations = {}
    problems = []
    levels = defaultdict(int)
    for lineno, line in enumerate(log_text.splitlines(), 1):
        parsed = parse_line(line)
        if parsed is None:
            problems.append(f"line {lineno}: unparseable")
            continue
        ts, level, fields = parsed
        levels[level] += 1
        job, event = fields["job"], fields["event"]
        current = state.get(job)
        allowed = TRANSITIONS.get(current, set())
        if event not in allowed:
            problems.append(f"line {lineno}: {job} {current} -> {event} not allowed")
        state[job] = event
        if event == "started":
            started_at[job] = (ts, fields.get("worker", "?"))
        elif event == "finished" and job in started_at:
            begin, worker = started_at.pop(job)
            durations[job] = (ts - begin, worker, fields.get("status"))
    return state, durations, problems, levels, started_at


def main():
    state, durations, problems, levels, running = run(LOG)
    print("final states:")
    for job in sorted(state):
        print(f"  {job:<10} {state[job]}")
    print("durations:")
    for job, (secs, worker, status) in sorted(durations.items(), key=lambda kv: -kv[1][0]):
        mins, secs = divmod(secs, 60)
        print(f"  {job:<10} {mins}m{secs:02d}s on {worker} [{status}]")
    print("still running:", ", ".join(sorted(running)) or "none")
    print("levels:", dict(sorted(levels.items())))
    for p in problems:
        print("problem:", p)
    errors = 0
    for line in LOG.splitlines():
        if " ERROR " in line or " WARN " in line:
            errors += 1
    print(f"{errors} warning/error lines; ok ratio {1 - errors / len(LOG.splitlines()):.2%}")


if __name__ == "__main__":
    main()
