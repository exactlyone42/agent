#!/bin/sh
set -eu
here=$(cd "$(dirname "$0")" && pwd)
repo=${1:-$(cd "$here/../.." && pwd)}
branch=$(git -C "$repo" rev-parse --abbrev-ref HEAD)
if [ "$branch" != "develop" ]; then
    echo "refusing to install: $repo is on branch $branch, not develop" >&2
    exit 1
fi
target=${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user
mkdir -p "$target"
sed "s|@REPO@|$repo|g" "$here/agent-drift.service" > "$target/agent-drift.service"
cp "$here/agent-drift.timer" "$target/agent-drift.timer"
systemctl --user daemon-reload
systemctl --user enable --now agent-drift.timer
systemctl --user list-timers agent-drift.timer --no-pager
