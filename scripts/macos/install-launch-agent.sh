#!/usr/bin/env sh
set -eu
AGENT="${1:-$HOME/.local/bin/vsn-agent}"
PLIST="$HOME/Library/LaunchAgents/dev.vsn.agent.plist"
mkdir -p "$HOME/Library/LaunchAgents"
sed "s|__VSN_AGENT_PATH__|$AGENT|g" "$(dirname "$0")/dev.vsn.agent.plist" > "$PLIST"
launchctl bootout "gui/$(id -u)/dev.vsn.agent" 2>/dev/null || true
launchctl bootstrap "gui/$(id -u)" "$PLIST"
launchctl enable "gui/$(id -u)/dev.vsn.agent"
