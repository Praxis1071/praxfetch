# praxfetch

A fast, zero-subprocess Linux system-information fetch utility written in Rust.

## What praxfetch does

praxfetch reads Linux kernel and system metadata directly instead of spawning external commands. It is designed around three principles:

- **Fast startup:** keep detection small and avoid shelling out.
- **Honest output:** unavailable data is reported as unavailable instead of being guessed.
- **Portable Linux behavior:** use standard Linux interfaces such as /proc, /sys, /etc/os-release, and POSIX APIs.

## Current modules

- **User / Hostname** — $USER + /proc/sys/kernel/hostname
- **OS** — /etc/os-release, falling back to /usr/lib/os-release; PRETTY_NAME is preferred for display
- **Host** — DMI vendor/product when available
- **Kernel** — /proc/sys/kernel/osrelease
- **Uptime** — /proc/uptime; malformed or unavailable values are reported as unknown rather than a fabricated zero
- **Load Average** — /proc/loadavg
- **Packages** — Pacman, dpkg, APK, and RPM detection, including the modern /usr/lib/sysimage/rpm database location
- **Shell** — $SHELL and exported shell-version variables when available
- **DE/WM** — XDG_CURRENT_DESKTOP / DESKTOP_SESSION
- **Terminal** — TERM_PROGRAM / TERM
- **CPU** — model from /proc/cpuinfo; online logical threads and physical cores from kernel topology; frequency prefers hardware-reported CPUFreq measurements, weights policy values by online CPUs, and clearly falls back to scaling_cur_freq only when needed
- **CPU Temperature** — prefers CPU package/die readings, ignores hwmon channels marked faulty, excludes AMD Tctl-only control readings from physical temperature, and falls back to matching CPU thermal zones
- **Memory** — /proc/meminfo
- **Swap** — /proc/meminfo
- **Mounted Storage** — parses /proc/self/mountinfo as raw bytes, decodes the kernel-defined escaped mount paths, and preserves non-UTF-8 Unix path bytes, excludes pseudo, remote, FUSE, and overlay filesystems, deduplicates bind mounts by major:minor, and uses POSIX statvfs() to aggregate accessible filesystem capacity without subprocesses
- **Battery** — /sys/class/power_supply/BAT* when available; preserves charging, discharging, full, and not-charging states, and avoids mixing incompatible energy/charge units and rejects invalid capacity weights when aggregating multiple batteries
- **GPU** — Linux DRM/sysfs card devices and kernel driver; resolves PCI vendor/device IDs through the locally installed pci.ids database when available, with readable vendor/ID fallback
- **Display** — connected DRM connector names; active resolution is not inferred from the list of supported modes exposed by sysfs
- **Network** — interface count, operational link state, and wireless interface count

## ASCII logo

praxfetch always displays the same Tux ASCII mascot on every Linux distribution. The logo is intentionally independent of distro detection, keeping the visual identity consistent. Distribution-specific color accents may still be applied.

## Design constraints

- **Zero subprocesses:** no Command::new, shell execution, uname, pacman, dpkg, rpm, or similar external command calls.
- **No network access:** normal operation is completely local.
- **No privileged access required:** missing or unreadable system files simply fall back to an unavailable value.
- **No guessed shell versions:** praxfetch never launches a shell to ask for its version.
- **Native hardware detection:** GPU, display, battery, and network data come from Linux sysfs/proc interfaces rather than external utilities.
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

- src/info.rs — current Linux detection, parsing, validation, and formatting implementation; modular extraction is planned in ROADMAP.md.
- src/ascii.rs — the single Tux logo, terminal-width-safe padding, distro-aware color accents, and color blocks.
- src/main.rs — CLI parsing and final presentation.

This mirrors a principle used by mature fetch tools such as Fastfetch: detection logic should not depend on presentation logic.

## Development roadmap

The prioritized implementation strategy, completed work, acceptance criteria, and research references are maintained in [ROADMAP.md](ROADMAP.md). Update that file alongside implementation work; it is the project's development source of truth.

## License

MIT
