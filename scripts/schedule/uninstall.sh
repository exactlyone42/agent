#!/bin/sh
set -eu
target=${XDG_CONFIG_HOME:-$HOME/.config}/systemd/user
systemctl --user disable --now agent-drift.timer
rm -f "$target/agent-drift.timer" "$target/agent-drift.service"
systemctl --user daemon-reload
