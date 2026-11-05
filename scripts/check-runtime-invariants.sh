#!/usr/bin/env bash
# Defense-in-depth source audit. This checks application code, not build scripts
# or CI helpers; praxfetch must not spawn child processes or open network sockets.
set -euo pipefail

if grep -RInE --include='*.rs' \
    'std::process::Command|Command[[:space:]]*::[[:space:]]*new|std::net::|TcpStream|UdpStream|UdpSocket|reqwest|ureq|hyper::Client|tokio::process' \
    src; then
    printf '%s\n' "Runtime invariant violation: subprocess or network API detected in src/." >&2
    exit 1
fi

if grep -nE '^(reqwest|ureq|hyper|isahc|surf|awc|tokio)[[:space:]]*=' Cargo.toml; then
    printf '%s\n' "Runtime invariant violation: network-capable direct dependency detected in Cargo.toml." >&2
    exit 1
fi

printf '%s\n' "Runtime invariant audit passed."
