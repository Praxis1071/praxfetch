# praxfetch

A fast Linux system-information fetch utility written entirely in Rust.

## Design goals

- Zero subprocesses: no `Command::new`, shell execution, or external system commands.
- Direct reads from `/proc`, `/sys`, `/etc/os-release`, and environment variables.
- Distribution-aware ASCII logos and ANSI 8/16-color palette.
- Release profile optimized for a small, fast executable.

## Data sources

- User / hostname: `$USER`, `/proc/sys/kernel/hostname`
- OS: `/etc/os-release`
- Hardware: `/sys/class/dmi/id/*`
- Kernel: `/proc/sys/kernel/osrelease`
- Uptime: `/proc/uptime`
- Packages: Pacman local DB, dpkg status, APK DB
- Shell: `$SHELL` plus shell-specific version environment variables when available
- Desktop: `XDG_CURRENT_DESKTOP`, `DESKTOP_SESSION`
- Terminal: `TERM_PROGRAM`, `TERM`
- CPU: `/proc/cpuinfo`
- Memory: `/proc/meminfo`

## Supported logos

CachyOS, Arch Linux, Ubuntu, Debian, Fedora, NixOS, Gentoo, Alpine, Void Linux and a generic Linux fallback.

## Build

```bash
cargo build --release
./target/release/praxfetch
```

Disable colors with `./target/release/praxfetch --no-color`.

## Performance

The project targets millisecond-level startup by avoiding subprocesses and keeping detection lightweight. Exact startup time depends on filesystem cache, terminal rendering, CPU, and runtime environment; **5 ms is a target, not a guaranteed bound**.

## Implementation notes

Fastfetch's current architecture separates detection from formatting/modules, and its logo system uses distro identity information and per-logo color palettes. praxfetch follows the same general separation at a smaller scope while keeping its Linux implementation dependency-light.

Shell versions are never guessed by launching the shell. If a shell-specific version environment variable is unavailable, praxfetch displays the shell name only.

## License

MIT
