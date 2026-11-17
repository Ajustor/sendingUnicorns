#!/usr/bin/env bash
# Fail when a binary links a library a user's machine may not have.
#
# macOS: only system libraries (/usr/lib, /System) are allowed; anything else
# (e.g. Homebrew's /opt/homebrew/opt/openssl@3) is missing on a bare system.
# WebKit is a system framework (the response preview).
# Linux: the preview's web view comes from the system (WebKitGTK, GTK), so
# those are expected; OpenSSL must not be linked directly (its soname differs
# between distributions); TLS is rustls, built in. Only the binary's own
# NEEDED entries are checked: ldd would also list what WebKitGTK pulls in.
#
# Usage: scripts/check-linked-libs.sh <binary>
set -euo pipefail

bin="$1"
case "$(uname -s)" in
  Darwin)
    otool -L "$bin"
    bad=$(otool -L "$bin" | tail -n +2 | grep -vE '^[[:space:]]*(/usr/lib/|/System/)' || true)
    ;;
  Linux)
    readelf -d "$bin" | grep NEEDED
    bad=$(readelf -d "$bin" | grep NEEDED | grep -E 'lib(ssl|crypto)\.' || true)
    ;;
  *)
    echo "nothing to check on $(uname -s)"
    exit 0
    ;;
esac

if [ -n "$bad" ]; then
  echo "::error::$bin links libraries users may not have:"
  echo "$bad"
  exit 1
fi
echo "OK: no unexpected library linked"
