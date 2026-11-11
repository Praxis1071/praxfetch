#!/bin/sh
set -eu

repo_root=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)

if ! command -v cargo >/dev/null 2>&1; then
    printf '%s\n' "Error: Rust Cargo is required. Install Rust from https://rustup.rs/." >&2
    exit 127
fi

cargo_home=${CARGO_HOME:-"$HOME/.cargo"}
install_dir="$cargo_home/bin"

printf '%s\n' "Building praxfetch (release)..."
cargo build --manifest-path "$repo_root/Cargo.toml" --release

binary="$repo_root/target/release/praxfetch"
if [ ! -x "$binary" ]; then
    printf 'Error: release binary was not created: %s\n' "$binary" >&2
    exit 1
fi

mkdir -p "$install_dir"
install -m 755 "$binary" "$install_dir/praxfetch"

case ":$PATH:" in
    *":$install_dir:"*)
        path_configured=true
        ;;
    *)
        path_configured=false
        ;;
esac

if [ "$path_configured" = false ]; then
    shell_name=${SHELL##*/}
    case "$shell_name" in
        zsh)
            profile="$HOME/.zshrc"
            ;;
        fish)
            profile="$HOME/.config/fish/config.fish"
            ;;
        bash)
            profile="$HOME/.bashrc"
            ;;
        *)
            profile="$HOME/.profile"
            ;;
    esac

    mkdir -p "$(dirname -- "$profile")"
    touch "$profile"

    if [ "$shell_name" = fish ]; then
        if ! grep -Fq "fish_add_path \"$install_dir\"" "$profile"; then
            printf '\n# Added by the praxfetch installer\nfish_add_path "%s"\n' "$install_dir" >> "$profile"
        fi
    else
        if ! grep -Fq "$install_dir" "$profile"; then
            printf '\n# Added by the praxfetch installer\nexport PATH="%s:$PATH"\n' "$install_dir" >> "$profile"
        fi
    fi

    printf 'Added %s to PATH in %s. Open a new terminal to use the command.\n' "$install_dir" "$profile"
fi

printf '\nInstalled successfully: %s/praxfetch\n' "$install_dir"
printf '%s\n' "Run: praxfetch"
