# praxfetch

A fast, zero-subprocess Linux system-information fetch utility written in Rust.

## What praxfetch does

praxfetch reads Linux kernel and system metadata directly instead of spawning external commands. It is designed around three principles:

- **Fast startup:** keep detection small and avoid shelling out.
- **Honest output:** unavailable data is reported as unavailable instead of being guessed.
- **Portable Linux behavior:** use standard Linux interfaces such as /proc, /sys, /etc/os-release, and POSIX APIs.

## Current modules

- **User / Hostname** — $USER + /proc/sys/kernel/hostname
- **OS** — /etc/os-release
- **Host** — DMI vendor/product when available
- **Kernel** — /proc/sys/kernel/osrelease
- **Uptime** — /proc/uptime
- **Load Average** — /proc/loadavg
- **Packages** — Pacman, dpkg, APK, and RPM detection
- **Shell** — $SHELL and exported shell-version variables when available
- **DE/WM** — XDG_CURRENT_DESKTOP / DESKTOP_SESSION
- **Terminal** — TERM_PROGRAM / TERM
- **CPU** — /proc/cpuinfo with sysfs CPU-range fallback
- **Memory** — /proc/meminfo
- **Swap** — /proc/meminfo
- **Root Disk** — POSIX statvfs("/") with no subprocess
- **Battery** — /sys/class/power_supply/BAT* when available

## Distro logos

CachyOS, Arch Linux, Ubuntu, Debian, Fedora, NixOS, Gentoo, Alpine, Void Linux and a generic Linux fallback are currently supported.

Distro family selection uses both ID and ID_LIKE, so derivatives can inherit a sensible family logo/color without requiring a dedicated logo for every distribution.

## Design constraints

- **Zero subprocesses:** no Command::new, shell execution, uname, pacman, dpkg, rpm, or similar external command calls.
- **No network access:** normal operation is completely local.
- **No privileged access required:** missing or unreadable system files simply fall back to an unavailable value.
- **No guessed shell versions:** praxfetch never launches a shell to ask for its version.
- **No misleading package count:** RPM is detected but intentionally reports count unavailable until a native parser is implemented.
- **No ANSI alignment bugs:** widths are calculated before color escape sequences are applied.

## CLI

~~~bash
praxfetch
praxfetch --no-color
NO_COLOR=1 praxfetch
praxfetch --help
praxfetch --version
~~~

--no-color and the standard NO_COLOR environment convention disable ANSI colors.

## Build and test

~~~bash
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
./target/release/praxfetch
~~~

The GitHub Actions workflow runs formatting, compilation, tests, and Clippy on pushes to main and pull requests.

## Performance

The project targets millisecond-level startup. **5 ms is a target, not a guaranteed bound.** Actual time depends on filesystem cache state, terminal rendering, package database size, CPU, and the runtime environment.

Package counting is one of the potentially expensive operations because some package databases require directory traversal or text parsing. This is deliberately preferred over executing a package-manager command, preserving the zero-subprocess guarantee.

The release profile uses optimization, ThinLTO, one codegen unit, abort-on-panic, and symbol stripping. These settings favor a compact, optimized release binary; actual startup performance must still be measured on real hardware.

## Architecture

The code is intentionally small, but follows a useful detection/formatting boundary:

- src/info.rs — Linux detection, parsing, validation, and formatting helpers.
- src/ascii.rs — distro logo selection, logo coloring, and color blocks.
- src/main.rs — CLI parsing and final presentation.

This mirrors a principle used by mature fetch tools such as Fastfetch: detection logic should not depend on presentation logic.

## Roadmap

### 0.1.x — Stabilization
- Real build/test verification on supported Linux environments
- Startup benchmark
- More parser tests
- Better package-manager coverage without subprocesses
- Native GPU, display and network detection

### 0.2.x — Hardware
- GPU identification through Linux sysfs/DRM
- Display/resolution information
- More detailed battery/power information
- Network interface summary

### 0.3.x — UX
- Configurable module selection
- More logo families
- Better terminal detection/version reporting
- Optional JSON output for scripting

### 1.0.x — Release quality
- Release binaries for common Linux architectures
- Reproducible release process
- Expanded CI matrix
- Documented compatibility and performance measurements

## License

MIT
