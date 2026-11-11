#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
tmp_root="$(mktemp -d)"
trap 'rm -rf "$tmp_root"' EXIT

fake_bin="$tmp_root/fake-bin"
mkdir -p "$fake_bin"

cat > "$fake_bin/cargo" <<'FAKE_CARGO'
#!/bin/sh
set -eu

manifest=""
while [ "$#" -gt 0 ]; do
    case "$1" in
        --manifest-path)
            manifest=$2
            shift 2
            ;;
        *)
            shift
            ;;
    esac
done

[ -n "$manifest" ] || exit 2
root=$(dirname -- "$manifest")
mkdir -p "$root/target/release"
cat > "$root/target/release/praxfetch" <<'FAKE_BINARY'
#!/bin/sh
printf '%s\n' "praxfetch 0.1.0"
FAKE_BINARY
chmod 755 "$root/target/release/praxfetch"
FAKE_CARGO
chmod 755 "$fake_bin/cargo"

test_shell() {
    shell_name="$1"
    home="$tmp_root/home-$shell_name"
    test_repo="$tmp_root/repo-$shell_name"
    cargo_home="$home/.cargo"

    mkdir -p "$home" "$test_repo/scripts"
    cp "$repo_root/scripts/install.sh" "$test_repo/scripts/install.sh"

    HOME="$home" \
    CARGO_HOME="$cargo_home" \
    SHELL="/bin/$shell_name" \
    PATH="$fake_bin:/usr/bin:/bin" \
        sh "$test_repo/scripts/install.sh"

    installed="$cargo_home/bin/praxfetch"
    test -x "$installed"
    output="$("$installed" --version)"
    test "$output" = "praxfetch 0.1.0"

    case "$shell_name" in
        zsh)
            profile="$home/.zshrc"
            expected="export PATH=\"$cargo_home/bin:\$PATH\""
            ;;
        bash)
            profile="$home/.bashrc"
            expected="export PATH=\"$cargo_home/bin:\$PATH\""
            ;;
        fish)
            profile="$home/.config/fish/config.fish"
            expected="fish_add_path \"$cargo_home/bin\""
            ;;
    esac

    grep -Fq "$expected" "$profile"

    # Re-running the installer must not duplicate the PATH entry.
    HOME="$home" \
    CARGO_HOME="$cargo_home" \
    SHELL="/bin/$shell_name" \
    PATH="$fake_bin:/usr/bin:/bin" \
        sh "$test_repo/scripts/install.sh"

    count="$(grep -Fc "Added by the praxfetch installer" "$profile")"
    test "$count" -eq 1

    printf 'Installer smoke test passed: %s\n' "$shell_name"
}

test_shell zsh
test_shell bash
test_shell fish
