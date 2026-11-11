#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$repo_root"

if ! command -v hyperfine >/dev/null 2>&1; then
    printf '%s\n' "hyperfine is required for repeatable startup measurements." >&2
    printf '%s\n' "Install it on Arch Linux with: sudo pacman -S hyperfine" >&2
    exit 127
fi

cargo build --release

binary="$repo_root/target/release/praxfetch"
if [[ ! -x "$binary" ]]; then
    printf 'Release binary not found: %s\n' "$binary" >&2
    exit 1
fi

output_dir="$repo_root/target/benchmarks"
mkdir -p "$output_dir"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
result_file="$output_dir/startup-$timestamp.json"

printf '%s\n' "Praxfetch startup benchmark"
printf 'Date (UTC): %s\n' "$timestamp"
printf 'Kernel: %s\n' "$(uname -sr)"
printf 'Architecture: %s\n' "$(uname -m)"
printf 'Rust: %s\n' "$(rustc --version)"
if [[ -r /proc/cpuinfo ]]; then
    cpu_model="$(awk -F ': ' '/^model name[[:space:]]*:/ { print $2; exit }' /proc/cpuinfo)"
    printf 'CPU: %s\n' "${cpu_model:-unknown}"
fi
if [[ -r /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor ]]; then
    printf 'CPU governor: %s\n' "$(cat /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor)"
else
    printf '%s\n' "CPU governor: unavailable"
fi
printf 'Binary: %s\n' "$binary"
printf 'JSON result: %s\n\n' "$result_file"

benchmark_command="\\"$binary\\" --no-color"
hyperfine \
    --warmup 10 \
    --runs 100 \
    --export-json "$result_file" \
    "$benchmark_command"
