"""Shortest-route planner over a small city road graph using BFS and Dijkstra (heapq)."""

import heapq
from collections import deque

ROADS = [
    ("A", "B", 4),
    ("A", "C", 2),
    ("B", "C", 5),
    ("B", "D", 10),
    ("C", "E", 3),
    ("E", "D", 4),
    ("D", "F", 11),
    ("E", "F", 20),
    ("G", "H", 1),
]


def build_graph(edges, directed=False):
    graph = {}
    for u, v, w in edges:
        graph.setdefault(u, []).append((v, w))
        if not directed:
            graph.setdefault(v, []).append((u, w))
        else:
            graph.setdefault(v, [])
    return graph


def bfs_hops(graph, start):
    hops = {start: 0}
    queue = deque([start])
    while queue:
        node = queue.popleft()
        for nxt, _ in graph[node]:
            if nxt not in hops:
                hops[nxt] = hops[node] + 1
                queue.append(nxt)
    return hops


def dijkstra(graph, source, target):
    dist = {source: 0}
    prev = {}
    heap = [(0, source)]
    visited = set()
    while heap:
        d, node = heapq.heappop(heap)
        if node in visited:
            continue
        visited.add(node)
        if node == target:
            break
        for nxt, w in graph[node]:
            nd = d + w
            if nd < dist.get(nxt, float("inf")):
                dist[nxt] = nd
                prev[nxt] = node
                heapq.heappush(heap, (nd, nxt))
    else:
        return None, []
    path = [target]
    while path[-1] != source:
        path.append(prev[path[-1]])
    path.reverse()
    return dist[target], path


def components(graph):
    seen = set()
    groups = []
    for node in sorted(graph):
        if node in seen:
            continue
        group = sorted(bfs_hops(graph, node))
        seen.update(group)
        groups.append(group)
    return groups


def main():
    graph = build_graph(ROADS)
    hops = bfs_hops(graph, "A")
    for node, h in sorted(hops.items()):
        print(f"{node}: {h} hop(s)")
    queries = [("A", "F"), ("A", "D"), ("B", "E"), ("A", "H")]
    for src, dst in queries:
        cost, path = dijkstra(graph, src, dst)
        if cost is None:
            print(f"{src} -> {dst}: unreachable")
        else:
            print(f"{src} -> {dst}: cost {cost:>3} via {' -> '.join(path)}")
    for i, group in enumerate(components(graph), start=1):
        print(f"component {i}: {group}")
    degree = {}
    for node, edges in graph.items():
        degree[node] = len(edges)
    hub = max(degree, key=lambda n: (degree[n], n))
    print(f"hub: {hub} with degree {degree[hub]}")


if __name__ == "__main__":
    main()
