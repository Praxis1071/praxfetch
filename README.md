# praxfetch

A fast, zero-subprocess Linux system-information fetch utility written in Rust.

**Build requirement:** Rust 1.88 or newer (required by Ratatui Core 0.1.2).

## What praxfetch does

praxfetch reads Linux kernel and system metadata directly instead of spawning external commands. It is designed around three principles:

- **Fast startup:** keep detection small and avoid shelling out.
- **Honest output:** unavailable data is reported as unavailable instead of being guessed.
- **Arch-first correctness:** validate behavior on Arch Linux x86_64 and CachyOS before making broader compatibility claims; use documented Linux interfaces such as /proc, /sys, /etc/os-release, and POSIX APIs.

## Current modules

- **User / Hostname** — $USER + /proc/sys/kernel/hostname
- **OS** — /etc/os-release, falling back to /usr/lib/os-release; PRETTY_NAME is preferred for display
- **Host** — DMI vendor/product when available
- **Kernel** — /proc/sys/kernel/osrelease
- **Uptime** — /proc/uptime; malformed or unavailable values are reported as unknown rather than a fabricated zero
- **Load Average** — /proc/loadavg; malformed or incomplete fields are reported as unavailable instead of shifting later values into the wrong columns
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
- **Network** — interface count, operational link state, and wireless interface count (detected internally; not shown in the minimal default output)

## ASCII logo

praxfetch always displays the same Tux ASCII mascot on every Linux distribution. The default output is intentionally minimal: username@hostname, OS, host, kernel, uptime, package count, and used/total memory in whole MiB. No additional system fields or color-block footer are printed by default. Distribution-specific color accents may still be applied.

## Design constraints

- **Zero subprocesses:** no Command::new, shell execution, uname, pacman, dpkg, rpm, or similar external command calls.
- **No network access:** normal operation is completely local.
- **No privileged access required:** missing or unreadable system files simply fall back to an unavailable value.
- **No guessed shell versions:** praxfetch never launches a shell to ask for its version.
- **Native hardware detection:** GPU, display, battery, and network data come from Linux sysfs/proc interfaces rather than external utilities.
- **No misleading package count:** Arch package counts use the default `/var/lib/pacman/local` database path; custom `DBPath` overrides in `pacman.conf` are not resolved yet. RPM is detected but intentionally reports count unavailable until a native parser is implemented.
- **No ANSI alignment bugs:** widths are calculated before color escape sequences are applied.

## CLI

~~~bash
praxfetch
praxfetch --no-color
NO_COLOR=1 praxfetch
praxfetch --3d
praxfetch --help
praxfetch --version
~~~

--no-color and the standard NO_COLOR environment convention disable ANSI colors.

### Interactive 3D Tux

Run `praxfetch --3d` inside [Ratty](https://github.com/orhun/ratty), Orhun Parmaksız's GPU-rendered terminal emulator. This mode keeps the exact classic seven-line Praxfetch layout: only the ASCII Tux in the left logo column is replaced by an original, procedurally generated multi-part 3D Tux. System-information fields remain in their usual positions. It uses the MIT-licensed `ratatui-ratty` widget and Ratty Graphics Protocol (RGP); it does **not** open an alternate screen, clear the terminal, or create a full-screen TUI. Drag the Tux with the left mouse button to rotate it, use the wheel to zoom, and press `Q` or `Esc` to exit. On exit, the 3D object is removed and the original ASCII Tux is restored. Other terminal emulators do not implement RGP and cannot display the 3D model.

Install Ratty on Arch Linux with `sudo pacman -S ratty`. Praxfetch does not install, launch, or manage the terminal emulator for you.

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

For a repeatable warm-start measurement on Arch Linux, install the optional external benchmark tool and run:

~~~bash
sudo pacman -S hyperfine
bash scripts/benchmark-startup.sh
~~~

The script builds the release binary, performs 10 warm-up runs and 100 measured runs, and saves Hyperfine's JSON report under `target/benchmarks/`. It also prints the kernel, architecture, CPU model, Rust version, and CPU governor when available. Do not compare results from different machines or power states as if they were equivalent.

The release profile uses optimization, ThinLTO, one codegen unit, abort-on-panic, and symbol stripping. These settings favor a compact, optimized release binary; actual startup performance must still be measured on real hardware.

## Architecture

The code is intentionally small, but follows a useful detection/formatting boundary:

- src/info.rs — current Linux detection, parsing, validation, and formatting implementation; further modular extraction is planned in ROADMAP.md.
- src/info/os_release.rs — isolated, deterministic parsing of os-release quoted and escaped values.
- src/info/packages.rs — package database detection and native package counts, prioritizing the Arch pacman local database.
- src/ascii.rs — the single Tux logo, terminal-width-safe padding, and distro-aware color accents.
- src/tux_3d.rs — inline Ratty-backed Tux model, procedural OBJ mesh generation, mouse rotation, and zoom controls without a full-screen interface.
- src/output.rs — shared classic output layout used by both ASCII and inline 3D modes.
- src/main.rs — CLI parsing and mode selection.

This mirrors a principle used by mature fetch tools such as Fastfetch: detection logic should not depend on presentation logic.

## Development roadmap

The prioritized implementation strategy, completed work, acceptance criteria, and research references are maintained in [ROADMAP.md](ROADMAP.md). Update that file alongside implementation work; it is the project's development source of truth.

## License

MIT
