#!/usr/bin/env bash
set -euo pipefail
HELPER="$1"; REQUEST_FILE="$2"; LABEL="${3:-dev.vsn.agent}"; PLIST="${4:-/Library/LaunchAgents/dev.vsn.agent.plist}"
UID_NOW="$(id -u)"; DOMAIN="gui/$UID_NOW"; running=0
launchctl print "$DOMAIN/$LABEL" >/dev/null 2>&1 && running=1 || true
if [ "$running" = 1 ]; then launchctl bootout "$DOMAIN" "$PLIST" || true; fi
trap 'if [ "$running" = 1 ]; then launchctl bootstrap "$DOMAIN" "$PLIST" || true; fi' EXIT
"$HELPER" < "$REQUEST_FILE"
