"""Word frequency counter over an embedded text sample."""

import re
import string
from collections import Counter

TEXT = """
It was the best of times, it was the worst of times, it was the age of
wisdom, it was the age of foolishness, it was the epoch of belief, it was
the epoch of incredulity, it was the season of Light, it was the season of
Darkness, it was the spring of hope, it was the winter of despair.
"""

STOPWORDS = {"the", "of", "it", "was", "a", "an", "and"}


def tokenize(text, lower=True):
    words = []
    for raw in re.split(r"\s+", text):
        word = raw.strip(string.punctuation)
        if not word:
            continue
        if lower:
            word = word.lower()
        words.append(word)
    return words


def count_words(words, skip=None):
    counts = {}
    for w in words:
        if skip is not None and w in skip:
            continue
        counts[w] = counts.get(w, 0) + 1
    return counts


def top_n(counts, n=5):
    items = sorted(counts.items(), key=lambda kv: (-kv[1], kv[0]))
    return items[:n]


def bar_chart(pairs, width=20):
    lines = []
    peak = 1
    for _, c in pairs:
        peak = max(peak, c)
    for word, c in pairs:
        bar = "#" * (c * width // peak)
        lines.append(f"{word:<12}|{bar:<{width}}| {c:>3}")
    return lines


def main():
    words = tokenize(TEXT)
    print(f"total words: {len(words)}")
    print(f"unique words: {len(set(words))}")
    counts = count_words(words, skip=STOPWORDS)
    for line in bar_chart(top_n(counts, n=6), width=10):
        print(line)
    longest = max(words, key=len)
    print(f"longest word: {longest!r} ({len(longest)} chars)")
    c = Counter(words)
    for word, n in c.most_common(3):
        print(f"{word}: {n}")
    initials = {}
    for w in words:
        initials[w[0]] = initials.get(w[0], 0) + 1
    letters = "".join(sorted(initials))
    print("initial letters:", letters)
    avg = sum(map(len, words)) / len(words)
    print(f"average length: {avg:.3f}")


if __name__ == "__main__":
    main()
