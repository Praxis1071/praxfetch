# praxfetch

A fast Linux system-information fetch utility written entirely in Rust.

## Design goals

- Zero subprocesses: no `Command::new`, shell execution, or external system commands.
- Direct reads from `/proc`, `/sys`, `/etc/os-release`, and environment variables.
- Distribution-aware ASCII logos and ANSI 8/16-color palette.
- Release profile optimized for a small, fast executable.
- Graceful fallbacks when a platform does not expose a particular hardware or package source.

## Data sources

- User / hostname: `$USER`, `/proc/sys/kernel/hostname`
- OS: `/etc/os-release`
- Hardware: `/sys/class/dmi/id/*`
- Kernel: `/proc/sys/kernel/osrelease`
- Uptime: `/proc/uptime`
- Packages: Pacman local DB, dpkg status, APK DB, RPM DB detection
- Shell: `$SHELL` plus shell-specific version environment variables when available
- Desktop: `XDG_CURRENT_DESKTOP`, `DESKTOP_SESSION`
- Terminal: `TERM_PROGRAM`, `TERM`
- CPU: `/proc/cpuinfo`, with sysfs fallback
- Memory: `/proc/meminfo`

When a package database is detected but its package count cannot be calculated without an additional database parser, praxfetch reports the manager and explicitly says that the count is unavailable rather than displaying a misleading zero.

## Supported logos

CachyOS, Arch Linux, Ubuntu, Debian, Fedora, NixOS, Gentoo, Alpine, Void Linux and a generic Linux fallback.

## Build

```bash
cargo build --release
./target/release/praxfetch
```

Disable colors with:

```bash
./target/release/praxfetch --no-color
```

Run the test suite with:

```bash
cargo test
```

## Performance

The project targets millisecond-level startup by avoiding subprocesses and keeping detection lightweight. Exact startup time depends on filesystem cache, terminal rendering, CPU, and runtime environment; **5 ms is a target, not a guaranteed bound**.

Package database enumeration is intentionally performed directly through filesystem APIs to preserve the zero-subprocess design. Its cost can vary between machines.

## Implementation notes

Fastfetch's current architecture separates detection from formatting/modules, and its logo system uses distro identity information and per-logo color palettes. praxfetch follows the same general separation at a smaller scope while keeping its Linux implementation dependency-light.

The output layout calculates widths before ANSI color codes are added, preventing escape sequences from corrupting column alignment. Logos currently use fixed-width ASCII geometry so their terminal-column width is deterministic.

Shell versions are never guessed by launching the shell. If a shell-specific version environment variable is unavailable, praxfetch displays the shell name only.

## License

MIT
