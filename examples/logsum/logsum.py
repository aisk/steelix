"""Summarize a web server access log.

Each line of the log is in the common log format, followed by the time it
took to serve the request:

    203.0.113.9 - - [12/Mar/2024:10:15:32 +0000] "GET /api/users HTTP/1.1" 200 512 41ms
"""

import argparse
import sys
from pathlib import Path

STATUS_CLASSES = {2: "success", 3: "redirect", 4: "client error", 5: "server error"}
DEFAULT_TOP = 5
SLOW_MS = 500
PERCENTILES = (50, 90, 99)
BAR_WIDTH = 30


def parse_args(argv):
    parser = argparse.ArgumentParser(prog="logsum", description=__doc__.splitlines()[0])
    parser.add_argument("logfile", help="access log to read")
    parser.add_argument("--top", type=int, default=DEFAULT_TOP, help="paths to list")
    parser.add_argument("--slow", type=int, default=SLOW_MS, help="slow request threshold (ms)")
    return parser.parse_args(argv)


def parse_line(line):
    """Split one log line into a request record, or return None if it is malformed."""
    parts = line.split()
    if len(parts) < 10 or not parts[3].startswith("["):
        return None
    status = parts[8]
    if len(status) != 3 or not status.isdigit():
        return None
    size = parts[9]
    millis = None
    if len(parts) > 10 and parts[10].endswith("ms"):
        millis = int(parts[10][:-2])
    return {
        "ip": parts[0],
        "hour": parts[3][13:15],
        "method": parts[5].lstrip('"'),
        "path": parts[6].split("?")[0],
        "status": int(status),
        "bytes": 0 if size == "-" else int(size),
        "ms": millis,
    }


def load(path):
    """Parse every line of the log, counting the lines that are not requests."""
    requests = []
    malformed = 0
    for line in path.read_text(encoding="utf-8").splitlines():
        if not line.strip():
            continue
        request = parse_line(line)
        if request is None:
            malformed += 1
        else:
            requests.append(request)
    return requests, malformed


def percentile(sorted_values, pct):
    """Nearest-rank percentile of an ascending list of numbers."""
    if not sorted_values:
        return 0
    rank = (pct * len(sorted_values) + 99) // 100
    return sorted_values[max(rank, 1) - 1]


def bar(count, peak, width=BAR_WIDTH):
    if peak == 0:
        return ""
    return "#" * max(1, count * width // peak)


def tally(requests, field):
    counts = {}
    for request in requests:
        key = request[field]
        counts[key] = counts.get(key, 0) + 1
    return counts


def print_statuses(requests):
    classes = {}
    for request in requests:
        name = STATUS_CLASSES.get(request["status"] // 100, "other")
        classes[name] = classes.get(name, 0) + 1
    peak = max(classes.values(), default=0)
    print("status")
    for name in sorted(classes, key=lambda n: (-classes[n], n)):
        print(f"  {name:<13} {classes[name]:>5}  {bar(classes[name], peak)}")


def print_paths(requests, top):
    paths = tally(requests, "path")
    ranked = sorted(paths.items(), key=lambda item: (-item[1], item[0]))
    print(f"top {min(top, len(ranked))} of {len(ranked)} paths")
    for path, count in ranked[:top]:
        print(f"  {count:>5}  {path}")


def print_latency(requests, slow):
    timed = []
    for request in requests:
        if request["ms"] is not None:
            timed.append(request["ms"])
    timed.sort()
    print(f"latency ({len(timed)} timed requests)")
    for pct in PERCENTILES:
        print(f"  p{pct:<3} {percentile(timed, pct):>6} ms")
    slow_requests = []
    for request in requests:
        if request["ms"] is not None and request["ms"] >= slow:
            slow_requests.append(request)
    print(f"  {len(slow_requests)} slower than {slow} ms")
    for request in sorted(slow_requests, key=lambda r: -r["ms"])[:3]:
        print(f"    {request['ms']:>6} ms  {request['method']} {request['path']}")


def print_hours(requests):
    hours = tally(requests, "hour")
    peak = max(hours.values(), default=0)
    print("requests per hour")
    for hour in sorted(hours):
        print(f"  {hour}:00 {hours[hour]:>4}  {bar(hours[hour], peak, width=20)}")


def main(argv):
    args = parse_args(argv)
    path = Path(args.logfile)
    if not path.is_file():
        print(f"logsum: {path}: no such file", file=sys.stderr)
        return 2
    requests, malformed = load(path)
    total_bytes = 0
    clients = set()
    for request in requests:
        total_bytes += request["bytes"]
        clients.add(request["ip"])
    print(f"{path.name}: {len(requests)} requests", end="")
    print(f" from {len(clients)} clients, {total_bytes / 1024:.1f} KiB")
    if malformed:
        print(f"skipped {malformed} malformed line{'s' if malformed != 1 else ''}")
    print()
    print_statuses(requests)
    print()
    print_paths(requests, args.top)
    print()
    print_latency(requests, args.slow)
    print()
    print_hours(requests)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
