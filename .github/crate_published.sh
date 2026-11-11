#!/usr/bin/env bash
# Prints `yes` if version $2 of the crate $1 is on crates.io and `no` if it
# is not, so that a rerun of the release workflow does not publish it again.
# Fails when crates.io gives no clear answer.
set -euo pipefail
status=$(curl --silent --output /dev/null --write-out '%{http_code}' \
  --user-agent "steelix-release-workflow" \
  "https://crates.io/api/v1/crates/$1/$2")
case "$status" in
  200) echo yes ;;
  404) echo no ;;
  *)
    echo "crates.io answered $status for $1 $2" >&2
    exit 2
    ;;
esac
