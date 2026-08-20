#!/usr/bin/env bash
set -euo pipefail
HELPER="$1"; REQUEST_FILE="$2"; UNIT="${3:-vsn-agent.service}"
was_active=0; systemctl --user is-active --quiet "$UNIT" && was_active=1 || true
if [ "$was_active" = 1 ]; then systemctl --user stop "$UNIT"; fi
trap 'if [ "$was_active" = 1 ]; then systemctl --user start "$UNIT" || true; fi' EXIT
"$HELPER" < "$REQUEST_FILE"
