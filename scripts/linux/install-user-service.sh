#!/usr/bin/env sh
set -eu
mkdir -p "$HOME/.config/systemd/user"
cp "$(dirname "$0")/vsn-agent.service" "$HOME/.config/systemd/user/vsn-agent.service"
systemctl --user daemon-reload
systemctl --user enable --now vsn-agent.service
systemctl --user status vsn-agent.service --no-pager
