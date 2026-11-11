# praxfetch Development Roadmap

> **Purpose:** Build praxfetch into a dependable, fast, maintainable Linux system-information CLI without sacrificing its defining constraints.
>
> **Last reviewed:** 2026-11-11  
> **Current baseline:** `0.1.0` · Rust 2021 · MIT · 48 automated tests; latest verified implementation CI: [run #160](https://github.com/Praxis1071/praxfetch/actions/runs/43094431295), commit `698dd800caeafcffc22fee39da6ddf509ec342c5`.

This is the project's source of truth for development priorities. Keep it updated as work lands. A checkbox is only marked complete after the implementation is present, relevant tests pass, documentation is updated, and the latest applicable CI run succeeds.

## 1. Product direction and non-negotiable rules

praxfetch should be a small, trustworthy, Linux-native alternative in the fastfetch/neofetch category: quick to launch, useful on real machines, predictable in scripts, and easy to maintain.

### Invariants

- [x] **Linux-native detection:** read kernel and system interfaces such as `/proc`, `/sys`, `/etc/os-release`, and POSIX APIs directly.
- [x] **Zero subprocesses at runtime:** never invoke shells, system utilities, package managers, `Command::new`, or external programs from praxfetch.
- [x] **No network access at runtime:** hardware and system discovery stays local.
- [x] **No root requirement:** inaccessible or absent information must degrade gracefully.
- [x] **Honest reporting:** distinguish unavailable data from zero, and do not invent hardware details.
- [x] **Consistent Tux identity:** use Tux on every distribution; distro detection may influence accents, not the mascot.
- [x] **MIT license.** Keep MIT for now: Fastfetch is also MIT-licensed; changing Praxfetch to GPLv3 is a project-policy choice, not a technical requirement. Revisit only if copyleft becomes an explicit project goal.
- [ ] **Arch-first compatibility contract:** define and test Arch Linux x86_64 as the initial target, with CachyOS as a first-class validation environment; document architecture, libc, toolchain, and kernel-interface assumptions.
- [ ] **Machine-readable output contract:** define and test stable JSON field names, types, nullability, and versioning before calling JSON output stable.

### Engineering rules

1. Correctness and truthful output come before adding more modules.
2. Preserve the zero-subprocess and no-network guarantees in every phase.
3. Prefer documented kernel interfaces and standard library/POSIX APIs over parsing human-oriented command output.
4. Keep detection, data models, formatting, and CLI parsing separate.
5. Keep dependencies small. Add a dependency only when it materially improves correctness, safety, or maintainability; record why it is needed.
6. Make parsing functions deterministic and independently testable. Treat kernel files as unavailable, malformed, partial, or changing at read time.
7. Never claim a performance target is met without repeatable measurements on identified hardware.
8. Do not make broad refactors and feature changes in one unreviewable step. Work in small, buildable commits.

## 2. Engineering priorities — mandatory workstreams

These three workstreams are the project's central quality goals. They take precedence over feature count, cosmetic changes, packaging, and release expansion. Work on them incrementally; do not mark a workstream complete merely because its checklist has been added.

### A. Code quality

**Goal:** make the code easy to reason about, test, review, and safely extend.

- [ ] Audit the entire current codebase before large refactors; record concrete findings and severity instead of rewriting by taste.
- [ ] Separate detection, parsing, validation, data models, and rendering with clear ownership and minimal coupling.
- [ ] Keep parsers deterministic and testable using supplied input; isolate filesystem access at narrow boundaries.
- [ ] Replace ambiguous sentinel values with explicit types such as Option and small enums where they improve correctness.
- [ ] Review integer conversions, arithmetic overflow, indexing, UTF-8 assumptions, path handling, and panic paths.
- [ ] Remove dead code, duplicated parsing, unnecessary clones/allocations, and dependencies only when evidence confirms they are unnecessary.
- [ ] Use idiomatic, documented Rust; keep unsafe code absent unless a demonstrated need exists. Any future unsafe block requires a written safety argument and tests.
- [ ] Keep functions and modules focused; avoid overengineering, global mutable state, hidden caching, and abstractions without a concrete use.
- [ ] Add regression tests for every discovered bug and focused tests for each parser's boundary conditions.
- [ ] Keep the user-facing default output unchanged unless the user explicitly approves a change.

**Acceptance:** understandable module boundaries, no known high-severity maintainability defects, no unexplained dead code or dependency, and all quality gates pass.

### B. Performance

**Goal:** make the actual program fast, not merely make performance-oriented code changes.

- [ ] Establish a reproducible release-binary baseline on the user's Arch/CachyOS x86_64 systems before optimizing.
- [ ] Measure warm and cold execution separately where feasible; run enough repetitions to report median and spread/percentiles.
- [ ] Record hardware, CPU governor/power mode, kernel, Rust compiler, build flags, binary size, command, and measurement tool.
- [ ] Measure end-to-end startup and identify the cost of each detector; pay particular attention to package database traversal, sysfs/procfs enumeration, PCI database reads, and filesystem metadata calls.
- [ ] Profile before optimizing. Use Linux perf/sampling or allocation profilers when available; do not infer bottlenecks from source appearance.
- [ ] Reduce work before making individual operations clever: avoid collecting information not needed by the selected output, repeated reads/parses, unnecessary directory scans, and needless allocations.
- [ ] Evaluate lazy collection and selective detector execution while preserving a clear architecture and predictable output.
- [ ] Compare every meaningful optimization against the same baseline and rerun correctness tests.
- [ ] Track startup latency, peak/resident memory where practical, and release binary size; do not optimize one metric while silently damaging another.
- [ ] Keep the <5 ms warm-start objective as an aspirational, hardware-specific target, never a universal promise or noisy CI gate.

**Acceptance:** checked-in benchmark instructions and reproducible measurements on identified hardware; every claimed improvement has before/after evidence and no correctness regression.

### C. Reliability and correctness

**Goal:** every displayed value must be defensible, and one broken or unavailable source must not break the whole application.

- [ ] Audit every detector against the actual Linux kernel ABI/documentation and the relevant userspace specification.
- [ ] Prioritize Arch Linux's real filesystem layout and package database behavior; test CachyOS as a real-world Arch-based environment.
- [ ] Test missing, unreadable, malformed, empty, truncated, changing, duplicated, reordered, oversized, and non-UTF-8 input where applicable.
- [ ] Distinguish unavailable, unsupported, permission-denied, and genuinely zero values; never fabricate a fallback.
- [ ] Validate units, ranges, integer arithmetic, float finiteness, aggregation rules, and semantic meaning before formatting.
- [ ] Avoid blocking or remote filesystem probes; preserve the no-network, no-subprocess, and no-root runtime invariants.
- [ ] Add fixture-driven tests for parser edge cases and integration tests for the observable CLI contract.
- [ ] Exercise multiple-device and unusual-host cases: multi-CPUFreq policy, multi-GPU, multi-display, multiple batteries, containers, VMs, and minimal installations.
- [ ] Run tests under current stable Rust and an explicitly chosen MSRV once the project defines one.
- [ ] Keep CI checks strict, deterministic, and useful; investigate warnings rather than suppressing them broadly.
- [ ] Maintain a known-limitations list. An unsupported datum should be reported as unknown rather than guessed.

**Acceptance:** no known high-severity correctness issue, robust fixture coverage for audited interfaces, runtime invariants retained, and final CI green.

### Current platform scope

- **Primary target:** Arch Linux x86_64.
- **First validation system:** CachyOS x86_64.
- **Not a current release promise:** Debian, Fedora, Alpine, other distributions, non-x86_64 architectures, BSD, macOS, and Windows.
- Keep detection code Linux-native and standards-based so future portability remains possible, but do not spend this phase claiming or building a broad compatibility matrix.
- Package creation, AUR publication, .deb/.rpm work, and release automation are deliberately deferred until the three engineering workstreams and baseline audit are in good shape.

---

## 3. Current baseline — completed

### Core identity and system information

- [x] Rust 2021 command-line application with a release profile configured for optimized builds.
- [x] Tux-only ASCII logo, distro-aware color accents, and ANSI-safe alignment.
- [x] OS identification from `/etc/os-release`, with `/usr/lib/os-release` fallback.
- [x] Host, kernel, uptime, load average, shell, desktop/window manager, and terminal information.
- [x] CPU model, logical thread count, and physical-core topology detection.
- [x] CPU frequency detection through CPUFreq policies, weighted by online CPUs; hardware-reported values are preferred and `scaling_cur_freq` is a last resort.
- [x] CPU temperature detection using CPU-specific hwmon sensors, ignoring faulted channels and non-physical Tctl-only readings, with a matching thermal-zone fallback.
- [x] Memory and swap information from `/proc/meminfo`.
- [x] Local mounted-filesystem capacity using `/proc/self/mountinfo` and `statvfs()`, with escaped-path parsing, pseudo/remote/FUSE/overlay filtering, bind-mount deduplication, and no root fallback when mount metadata is unavailable.
- [x] Battery percentage and charging-state aggregation without mixing incompatible energy/charge units.
- [x] GPU and driver discovery through DRM/sysfs, with local PCI ID database lookup when available.
- [x] DRM display connector status and name discovery; active resolution is not guessed from the connector's supported-mode list.
- [x] Network interface, link-state, and wireless-interface discovery.
- [x] Pacman, dpkg, APK, and RPM database detection; RPM package count remains explicitly unavailable until a reliable native implementation exists.

### CLI, documentation, and quality gates

- [x] `--no-color`, `NO_COLOR`, `--help`, and `--version`.
- [x] Keep the default human output limited to username@hostname, OS, host product, kernel, uptime, package count, and used/total memory; render memory in whole MiB and remove the color-block footer.
- [x] README documents the runtime constraints, build commands, current modules, and the 5 ms goal as a target rather than a guarantee.
- [x] GitHub Actions checks formatting, compilation, tests, Clippy warnings, and release compilation.
- [x] Automated tests cover important parsing and edge cases, including OS metadata, CPU topology/frequency, CPU temperature selection, PCI IDs, mountinfo, battery aggregation, and formatting.
- [x] Latest verified baseline: 43 tests passed; format, compile, source invariant audit, Clippy, and release build passed in [Actions run #142](https://github.com/Praxis1071/praxfetch/actions/runs/43090814311).

## 4. Prioritized implementation roadmap

The phases are ordered by dependency and risk, not by how visually impressive a feature looks. Finish the acceptance criteria for a phase before moving its unfinished work into the completed column.

### Initial repository audit — findings to address

The first source review after setting these priorities found the following concrete work. These are observations, not claims that the whole audit is complete.

- [x] **P0 / Performance:** the default `collect()` path no longer executes CPU, temperature, battery, disk, GPU, display, network, shell, desktop, terminal, or load-average detection because none of those fields appear in the approved output. The detector implementations remain available for future explicitly requested modules.
- [ ] **P0 / Performance:** benchmark the change on Arch/CachyOS hardware and record before/after startup latency; the code change is CI-verified, but its real-device speedup has not yet been measured.
- [ ] **P0 / Code structure:** `src/info.rs` currently owns a large mixture of filesystem access, parsing, aggregation, and formatting. Extract modules incrementally with behavior-preserving tests; do not perform a one-shot rewrite.
- [ ] **P1 / Reproducibility:** the repository currently has no `Cargo.lock`; introduce and commit it before relying on `--locked` CI or release builds.
- [x] **P1 / CI security:** pin all current workflow actions to verified full commit SHAs and restrict `GITHUB_TOKEN` to `contents: read`; review and verify every action update.
- [x] **P1 / Test coverage:** add executable-level regression tests for the exact seven-line default contract, field order, `--no-color`, `NO_COLOR`, `--help`, `--version`, and invalid-argument exit behavior; add a filesystem fixture test for Arch pacman local-database counting.
- [ ] **Scope review:** package database readers for non-Arch managers remain in the code. Decide whether to retain them as low-cost parser portability or remove them if they materially increase maintenance burden; do not remove functionality without measurements and a documented decision.

### Phase 1 — Stabilize and establish a trustworthy baseline

**Priority: P0 · Do before major feature work**

- [ ] Review every current detector for missing-file, permission-denied, malformed-data, empty-file, integer-overflow, and partial-read behavior.
- [ ] Audit CPU frequency semantics across drivers and virtual machines; do not present a requested frequency as a guaranteed instantaneous hardware measurement.
- [ ] Revisit CPU temperature selection and validation against actual hwmon driver conventions, including package/die/core labels and drivers that expose non-temperature channels.
- [ ] Revisit storage aggregation semantics: distinguish filesystems/mounts from physical disks; avoid double-counting subvolumes, overlay mounts, network mounts, and unusual device mappings.
- [ ] Add fixture-based parser tests for malformed, truncated, duplicate, reordered, and missing fields.
- [x] Add a filesystem-backed regression test for the Arch pacman local database counter, including ignored non-directory entries and an unavailable database path.
- [ ] Add regression tests for multiple GPUs, displays, batteries, network interfaces, and CPUFreq policies.
- [x] Add a CI source audit that flags common subprocess APIs, socket APIs, and direct network-client dependencies; treat it as defense-in-depth rather than a formal proof.
- [ ] Verify output on the primary Arch Linux x86_64 target, including the user's CachyOS systems; expand to AMD, VM, and minimal/container environments only when available and useful.

**Audit fixes completed on 2026-11-05**

- [x] Stop presenting the first DRM connector `modes` entry as the active display resolution; sysfs exposes supported modes, not necessarily the currently selected mode.
- [x] Preserve missing `MemAvailable` and `SwapFree` values as unavailable instead of reporting zero available memory or 100% usage.
- [x] Ignore hwmon temperature channels marked faulty and include the documented Intel PECI CPU temperature driver.
- [x] Do not present AMD `Tctl`-only control readings as physical CPU die temperature; prefer physical die/package readings when available and allow CPU thermal-zone fallback when Tctl is the only hwmon reading.
- [x] Remove the DMI computer product name as a fallback CPU model, avoiding laptop/desktop model names being mislabeled as processors.
- [x] Follow os-release precedence precisely: fall back to `/usr/lib/os-release` only when `/etc/os-release` is missing, and parse quoted/unquoted backslash escapes according to their shell-style quoting context.
- [x] Make package counts unavailable when the database cannot be read or a directory scan is incomplete; parse dpkg status stanzas correctly with LF/CRLF line endings and validate APK package names.
- [x] Validate battery energy/charge weights as finite positive values and aggregate them in `f64` to avoid malformed or extreme readings producing NaN/Infinity output.
- [x] Centralize sysfs directory enumeration and reject incomplete directory iteration instead of presenting partial GPU, display, network, battery, or sensor inventories as complete.
- [x] Add fixture-based `/proc/meminfo` parsing tests and regression tests for missing values, hwmon faults, temperature boundaries, display connector-only output, and CPUFreq policy weighting/overflow.
- [x] Weight CPUFreq policy readings by online CPUs, ignore policies with no online CPUs, and use checked arithmetic for aggregation.
- [x] Exclude known remote, generic FUSE, and overlay mounts before calling `statvfs()`; do not probe `/` as a fallback when mount metadata is unavailable, preserving the no-network and non-blocking design.
- [x] Add a CI source audit for common runtime subprocess and network APIs.
- [x] Preserve non-UTF-8 mount-point bytes from `/proc/self/mountinfo` with Unix `OsString`; decode only the four kernel-defined path escapes and reject malformed escapes.
- [x] Keep missing or malformed `/proc/uptime` data unavailable instead of formatting it as a real zero-second uptime.
- [x] Parse the first three `/proc/loadavg` fields positionally; reject malformed or missing values instead of shifting later fields into earlier load averages.

**Exit criteria:** no known incorrect labels or silent parser panics; core detector behavior is covered by deterministic tests; runtime invariants are demonstrably preserved.

### Phase 2 — Modularize the code without changing behavior

**Priority: P0 · Foundation for safe growth**

- [ ] Split the large `src/info.rs` into focused modules: `os`, `host`, `cpu`, `memory`, `storage`, `battery`, `gpu`, `display`, `network`, and `packages`.
- [ ] Move shared data structures into a small, clearly owned model layer; avoid circular module dependencies.
- [ ] Separate raw file reading, pure parsing/normalization, validated system models, and presentation formatting.
- [ ] Keep system paths and file reads injectable at parser/detector boundaries where this improves tests; do not build an elaborate abstraction framework.
- [ ] Keep collection orchestration explicit. Avoid global mutable state, hidden caches, plugin registries, and unnecessary async code.
- [ ] Preserve existing public output and CLI behavior during the refactor.
- [ ] Move tests alongside the module they protect; use integration tests for externally visible CLI behavior.
- [ ] Update README architecture notes and add a short contributor/development guide.

**Exit criteria:** all existing behavior remains intact; tests and CI pass after each extraction; module responsibilities are obvious to a new contributor.

### Phase 3 — CLI behavior and machine-readable output

**Priority: P1 · High user value**

- [ ] Design a small CLI contract around the existing `clap` dependency; do not add flags without a clear use case.
- [ ] Add `--format human|json` (and optionally `--json` as a convenience alias) with human output remaining the default.
- [ ] Define a typed, documented JSON schema. Use proper serialization rather than manually concatenating JSON strings.
- [ ] Represent unavailable values consistently as `null` or an explicitly documented status; never silently turn unknown into zero or an empty string.
- [ ] Keep JSON free of ANSI escapes, banners, decorative logo output, and progress messages.
- [ ] Send diagnostics and errors to stderr; reserve stdout for the selected output format.
- [ ] Add `--show` / `--hide` module selection only after module collection can skip unrequested and potentially expensive work.
- [ ] Add stable module identifiers, helpful invalid-module errors, and complete `--help` examples.
- [ ] Test JSON parsing, schema snapshots, no-color behavior, stdout/stderr separation, exit codes, and combinations of CLI flags.

**Design note:** Serde/serde_json are the preferred default if JSON is implemented; evaluate the binary-size and startup cost rather than hand-writing a fragile serializer.

**Exit criteria:** scripts can consume documented JSON reliably; human output remains backward-compatible unless a deliberate release note says otherwise.

### Phase 4 — Performance measurement and optimization

**Priority: P1 · Measure first, optimize second**

- [ ] Establish a reproducible startup benchmark baseline for the compiled release binary on the user's Arch/CachyOS hardware.
- [x] Add `scripts/benchmark-startup.sh`: 10 warm-up runs, 100 measured release-binary runs, environment metadata, and a JSON report via external Hyperfine.
- [ ] Measure cold-cache and warm-cache runs separately; record CPU, kernel, storage, build profile, compiler, and command used.
- [ ] Report distributions (median and tail/percentile where tooling supports it), not one best-case timing.
- [ ] Measure the cost of each detector, especially package database traversal, PCI database parsing, mount enumeration, and sensor discovery.
- [ ] Add microbenchmarks only for pure hot-path parsers where they answer a concrete question.
- [ ] Use profiling to locate actual bottlenecks before changing algorithms or adding caches.
- [ ] Consider lazy/selected-module collection to avoid unnecessary filesystem reads.
- [ ] Evaluate release profile settings empirically; retain LTO, codegen-unit, stripping, or panic settings only when their trade-offs are justified.
- [ ] Add a non-blocking performance report first. Introduce regression thresholds only after runner variance is understood.
- [ ] Keep external benchmark tools outside the application; the praxfetch binary itself must never spawn them.

**Performance target:** aim for a typical warm-start execution below 5 ms on a clearly identified reference system. This is a goal, not a universal guarantee. Do not gate correctness or claim success from GitHub-hosted runner timing alone.

**Exit criteria:** a repeatable benchmark procedure and published baseline exist; any optimization includes before/after evidence and no correctness regression.

### Phase 5 — CI, supply-chain, and compatibility hardening

**Priority: P1 · Required before stable releases**

- [x] Give GitHub Actions the minimum required permissions (`contents: read` for the current build-only workflow).
- [x] Pin third-party Actions to verified full commit SHAs; record the corresponding release tags in comments.
- [ ] Use `Cargo.lock` and `--locked` in CI for reproducible dependency resolution.
- [ ] Add dependency/advisory review (for example, `cargo audit`) with a documented policy for advisories and exceptions.
- [ ] Add a supported Rust toolchain/MSRV policy and test it deliberately; do not accidentally promise an untested MSRV.
- [ ] Keep CI aligned with the current Arch-first x86_64 scope; add aarch64 or other targets only after they become explicit support commitments.
- [ ] Keep formatting, check, tests, Clippy with warnings denied, and release build as required gates.
- [x] Add integration checks for the current CLI contract: `--help`, `--version`, `--no-color`, `NO_COLOR`, exact default fields, and invalid-argument exit status. JSON checks remain deferred until JSON output exists.
- [ ] Consider scheduled dependency update checks and a documented update/review cadence.
- [ ] Verify release artifacts are built from tagged commits and include checksums; consider provenance/attestation when the release workflow is mature.

**Exit criteria:** CI is least-privilege, dependency resolution is reproducible, supported targets are explicit, and every release candidate passes the same gates.

### Phase 6 — Hardware coverage and useful additions

**Priority: P2 · Only after the foundation is stable**

- [ ] Add GPU VRAM and additional driver information only through documented sysfs/DRM interfaces with safe fallbacks.
- [ ] Add GPU temperature only when the sensor can be attributed to the correct GPU; never label a generic hwmon sensor as GPU temperature by guesswork.
- [ ] Improve storage reporting to distinguish filesystem capacity, mount point, backing device, and physical disk where the kernel exposes enough information.
- [ ] Improve battery health/cycle information where supported by power-supply sysfs.
- [ ] Improve network classification without assuming interface names identify hardware type.
- [ ] Add optional locale/terminal-width robustness and test Unicode display alignment.
- [ ] Consider configuration files only if repeated user needs cannot be handled cleanly by CLI flags.
- [ ] Keep optional/rare modules isolated so a missing kernel driver never affects the rest of the output.

**Exit criteria:** each new field has a documented source, clear semantics, fallback behavior, and tests on both supported and unsupported hardware.

### Phase 7 — Release and maintenance

**Priority: P2 · 1.0 readiness**

- [ ] Define and publish the Arch Linux x86_64 support contract and its known limitations.
- [ ] Review the CLI and JSON schema for compatibility before 1.0.
- [ ] Document installation, source build, binary installation, troubleshooting, and known limitations.
- [ ] Produce release binaries for the explicitly supported Arch Linux x86_64 target from clean CI jobs.
- [ ] Publish checksums and release notes describing user-visible changes and limitations.
- [ ] Test upgrade behavior and ensure old command-line usage remains valid.
- [ ] Tag releases with semantic versioning and keep the changelog/README/ROADMAP synchronized.
- [ ] Create a maintenance checklist for dependency updates, CI failures, and security reports.

**1.0 release gate:** no known critical correctness or security issue; clean CI; supported-platform test evidence; documented CLI/JSON contracts; repeatable release artifacts; measured performance results with honest hardware context.

## 5. Explicit non-goals

- No runtime subprocesses, shell-outs, package-manager command execution, or network requests.
- No root-only detection path as a normal requirement.
- No speculative sensor values, fabricated hardware names, or misleading physical-disk counts.
- No large dependency stack, async runtime, daemon, background service, or plugin system without a demonstrated need.
- No premature micro-optimizations or hard CI timing thresholds based on noisy shared runners.
- No feature expansion that makes the default output slower or less reliable without a clear user benefit.

## 6. Definition of done for every change

A change is complete only when all applicable items below are satisfied:

- [ ] The implementation follows the project invariants.
- [ ] Missing, malformed, and unsupported system data are handled safely.
- [ ] Pure parsing/formatting logic has focused tests; externally visible behavior has integration coverage where appropriate.
- [ ] `cargo fmt --all -- --check` passes.
- [ ] `cargo check --all-targets` passes.
- [ ] `cargo test --all-targets` passes.
- [ ] `cargo clippy --all-targets -- -D warnings` passes.
- [ ] `cargo build --release` passes.
- [ ] README and this roadmap reflect the actual state.
- [ ] The latest GitHub Actions run for the final commit succeeds.
- [ ] Performance claims, if any, include reproducible measurements and hardware context.

## 7. Research references

- [Rust Performance Book — benchmarking](https://nnethercote.github.io/perf-book/benchmarking.html)
- [Rust Performance Book — profiling](https://nnethercote.github.io/perf-book/profiling.html)
- [Rust Performance Book — general optimization principles](https://nnethercote.github.io/perf-book/general-tips.html)
- [RustSec cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
- [Arch Linux packaging standards](https://wiki.archlinux.org/title/Arch_Packaging_Standards)
- [Arch makepkg manual](https://man.archlinux.org/man/makepkg.8)
- [Fastfetch upstream repository and MIT license](https://github.com/fastfetch-cli/fastfetch)
- [Arch Linux pacman local database and database paths](https://wiki.archlinux.org/title/Pacman)
- [pacman.conf(5) — DBPath and default database directory](https://man.archlinux.org/man/pacman.conf.5.en)

Use primary documentation first and revisit these links when implementing the related phase:

- [Linux procfs documentation — including mountinfo and meminfo formats](https://docs.kernel.org/7.0/filesystems/proc.html)
- [systemd os-release specification](https://www.freedesktop.org/software/systemd/man/latest/os-release.html)
- [Linux CPUFreq policy interface](https://docs.kernel.org/admin-guide/pm/cpufreq.html)
- [Linux thermal sysfs interface](https://docs.kernel.org/driver-api/thermal/sysfs-api.html)
- [Linux hwmon sysfs naming, temperature units, labels, and fault flags](https://docs.kernel.org/hwmon/sysfs-interface.html)
- [Linux Intel coretemp driver and package sensor semantics](https://docs.kernel.org/hwmon/coretemp.html)
- [Linux AMD k10temp driver and Tctl/Tdie semantics](https://docs.kernel.org/hwmon/k10temp.html)
- [Linux PECI CPU temperature driver](https://docs.kernel.org/hwmon/peci-cputemp.html)
- [Linux DRM/KMS connector mode semantics](https://docs.kernel.org/gpu/drm-kms.html)
- [Linux FUSE technical documentation](https://docs.kernel.org/filesystems/fuse/index.html)
- [Linux CIFS client documentation](https://docs.kernel.org/admin-guide/cifs/usage.html)
- [Linux NFS documentation](https://docs.kernel.org/filesystems/nfs/)
- [Linux /proc/meminfo field semantics](https://docs.kernel.org/7.0/filesystems/proc.html)
- [Linux hwmon coretemp driver and package/core sensor semantics](https://docs.kernel.org/hwmon/coretemp.html)
- [Linux hwmon sysfs naming and data formats](https://docs.kernel.org/hwmon/sysfs-interface.html)
- [Rust Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html)
- [Cargo test and target selection](https://doc.rust-lang.org/cargo/commands/cargo-test.html)
- [clap command-line parser documentation](https://docs.rs/clap/latest/clap/)
- [Serde serialization framework](https://serde.rs/)
- [serde_json API](https://docs.rs/serde_json/latest/serde_json/)
- [Command Line Interface Guidelines](https://clig.dev/)
- [Criterion.rs statistical benchmarking](https://criterion-rs.github.io/book/)
- [GitHub Actions secure use and SHA pinning](https://docs.github.com/en/actions/reference/security/secure-use)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [The Rust Programming Language — error handling](https://doc.rust-lang.org/book/ch09-00-error-handling.html)
- [Rust By Example — error handling and panic guidance](https://doc.rust-lang.org/rust-by-example/error.html)
- [Clippy lint catalogue — correctness, suspicious, and performance lints](https://rust-lang.github.io/rust-clippy/master/index.html)
- [Cargo profiles — optimization trade-offs](https://doc.rust-lang.org/cargo/reference/profiles.html)

---

**Maintenance rule:** update this file in the same change series as implementation work. Mark items complete only after the definition-of-done checks are met. If priorities change, explain why in the relevant phase instead of silently deleting unfinished work.
