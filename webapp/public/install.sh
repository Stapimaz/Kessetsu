#!/bin/sh
# Kessetsu per-user installer; no sudo, telemetry, or simulator installation.
set -eu

fail() { printf '\nKessetsu: %s\nHelp: https://kessetsu.com/install/\n' "$*" >&2; exit 1; }
hash_file() {
    if command -v sha256sum >/dev/null 2>&1; then sha256sum "$1" | cut -d ' ' -f 1
    else shasum -a 256 "$1" | cut -d ' ' -f 1; fi
}
json_field() { sed -n 's/^[[:space:]]*"'"$1"'"[[:space:]]*:[[:space:]]*"\([^"]*\)"[,]\{0,1\}[[:space:]]*$/\1/p' "$2"; }

install_kessetsu() {
    version=latest
    custom_root=''
    no_path=false
    while [ "$#" -gt 0 ]; do
        case "$1" in
            --install-dir) [ "$#" -ge 2 ] || fail '--install-dir needs a dedicated absolute directory.'; custom_root=$2; shift 2 ;;
            --no-path) no_path=true; shift ;;
            latest|[0-9]*) version=$1; shift ;;
            *) fail "Unknown option: $1" ;;
        esac
    done
    case "$version" in latest) ;; *) printf '%s\n' "$version" | grep -Eq '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$' || fail 'Use latest or a stable version such as 1.0.0.' ;; esac
    system=$(uname -s)
    architecture=$(uname -m)
    case "$system/$architecture" in
        Linux/x86_64) target=linux-x86_64 ;;
        Darwin/x86_64) target=macos-x86_64 ;;
        Darwin/arm64|Darwin/aarch64) target=macos-aarch64 ;;
        *) fail "Unsupported platform: $system/$architecture. Use the Web Hub or manual build." ;;
    esac
    [ -n "${HOME:-}" ] || fail 'A user home directory is required.'
    case "$HOME" in /*) ;; *) fail 'Home directory must be absolute.' ;; esac
    for tool in curl tar sed grep mktemp; do command -v "$tool" >/dev/null 2>&1 || fail "Required tool missing: $tool"; done
    command -v sha256sum >/dev/null 2>&1 || command -v shasum >/dev/null 2>&1 || fail 'Install sha256sum or shasum first.'
    root="$HOME/.local/share/kessetsu"
    bin="$HOME/.local/bin"
    if [ -n "$custom_root" ]; then
        case "$custom_root" in /*) ;; *) fail 'Install directory must be absolute.' ;; esac
        root=${custom_root%/}; bin="$root/bin"
        [ -n "$root" ] && [ "$root" != "$HOME" ] || fail 'Choose a dedicated install directory.'
    fi
    [ ! -L "$root" ] || fail 'Install directory must not be a link.'
    if [ -e "$root" ]; then
        [ -f "$root/.kessetsu-install" ] && [ "$(cat "$root/.kessetsu-install")" = kessetsu.install.v1 ] || fail "Unmanaged directory: $root. Nothing was overwritten."
    else
        mkdir -p "$root"
        printf 'kessetsu.install.v1\n' > "$root/.kessetsu-install"
    fi
    mkdir "$root/.install-lock" 2>/dev/null || fail 'Another installation may be running. Do not remove its lock until it finishes.'
    scratch=''
    # The exact mktemp directory and installer-owned empty lock are the only cleanup targets.
    trap '[ -z "$scratch" ] || rm -rf -- "$scratch"; rmdir "$root/.install-lock"' EXIT
    trap 'exit 1' HUP INT TERM
    scratch=$(mktemp -d "${TMPDIR:-/tmp}/kessetsu-install.XXXXXXXX")
    printf 'Finding the official Kessetsu release...\n'
    if [ "$version" = latest ]; then
        effective=$(curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL --connect-timeout 20 --max-time 60 -o /dev/null -w '%{url_effective}' https://github.com/Stapimaz/Kessetsu/releases/latest) || fail 'Could not find the latest release. Check your internet connection.'
        case "$effective" in https://github.com/Stapimaz/Kessetsu/releases/tag/v*) version=${effective##*/v} ;; *) fail 'Unexpected release redirect.' ;; esac
        printf '%s\n' "$version" | grep -Eq '^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$' || fail 'Latest release is not a stable version.'
    fi
    bundle="kessetsu-v$version-$target"
    archive="$bundle.tar.gz"
    base="https://github.com/Stapimaz/Kessetsu/releases/download/v$version"
    for name in "$archive" "$archive.sha256"; do
        curl --proto '=https' --proto-redir '=https' --tlsv1.2 -fsSL --retry 2 --connect-timeout 20 --max-time 180 "$base/$name" -o "$scratch/$name" || fail "Could not download $name. Previous installation was not changed."
    done
    read -r expected recorded < "$scratch/$archive.sha256"
    printf '%s\n' "$expected" | grep -Eq '^[a-fA-F0-9]{64}$' || fail 'Invalid checksum record.'
    [ "$recorded" = "$archive" ] || fail 'Checksum filename mismatch.'
    [ "$(hash_file "$scratch/$archive")" = "$expected" ] || fail 'Archive checksum mismatch. Previous installation was not changed.'
    tar -tzf "$scratch/$archive" > "$scratch/entries"
    while IFS= read -r entry; do
        case "$entry" in "$bundle/"*) ;; *) fail 'Unsafe archive root.' ;; esac
        case "/$entry/" in */../*|*\\*) fail 'Unsafe archive path.' ;; esac
    done < "$scratch/entries"
    tar -tvzf "$scratch/$archive" | grep -Ev '^[-d]' > "$scratch/links" || true
    [ ! -s "$scratch/links" ] || fail 'Archive contains unsupported links or special files.'
    mkdir "$scratch/unpacked"
    tar -xzf "$scratch/$archive" -C "$scratch/unpacked"
    stage="$scratch/unpacked/$bundle"
    manifest="$stage/release-manifest.json"
    [ "$(json_field schema_version "$manifest")" = kessetsu.release.v1 ] &&
        [ "$(json_field version "$manifest")" = "$version" ] &&
        [ "$(json_field target "$manifest")" = "$target" ] &&
        [ "$(json_field executable "$manifest")" = kess ] || fail 'Release manifest identity mismatch.'
    [ "$(hash_file "$stage/kess")" = "$(json_field executable_sha256 "$manifest")" ] || fail 'Executable checksum mismatch.'
    chmod +x "$stage/kess"
    [ "$("$stage/kess" --version)" = "kess $version" ] || fail 'CLI version probe failed.'
    [ ! -L "$root/releases" ] || fail 'Managed releases directory must not be a link.'
    mkdir -p "$root/releases" "$bin"
    if [ -e "$bin/kess" ] || [ -L "$bin/kess" ]; then
        [ -L "$bin/kess" ] || fail 'Existing ~/.local/bin/kess is not managed by this installer.'
        [ ! -d "$bin/kess" ] || fail 'Existing kess points to a directory.'
        case "$(readlink "$bin/kess")" in "$root/releases/"*/kess) ;; *) fail 'Existing kess link is not managed by this installer.' ;; esac
    fi
    install_id="v$version-$(basename "$scratch")"
    destination="$root/releases/$install_id"
    [ ! -e "$destination" ] || fail 'Installation destination already exists.'
    mv "$stage" "$destination"
    # Never replace an unrelated executable. Atomic rename preserves the old link on failure.
    pending="$bin/.kess-$(basename "$scratch")"
    ln -s "$destination/kess" "$pending"
    bin_quoted=$(printf '%s' "$bin" | sed "s/'/'\\\\''/g")
    path_line="export PATH='$bin_quoted':\"\$PATH\" # Kessetsu installer"
    case "${SHELL:-}" in
        */zsh) profiles="$HOME/.zshrc:$HOME/.zprofile" ;;
        */bash) profiles="$HOME/.bashrc:$HOME/.bash_profile" ;;
        */fish) profiles='' ;;
        *) profiles="$HOME/.profile" ;;
    esac
    if [ -n "$profiles" ] && [ "$no_path" = false ]; then
        old_ifs=$IFS; IFS=:
        for profile in $profiles; do
            if [ ! -e "$profile" ] || ! grep -Fxq "$path_line" "$profile"; then
                printf '\n%s\n' "$path_line" >> "$profile"
            fi
        done
        IFS=$old_ifs
    fi
    if [ "$no_path" = false ]; then
        case "${SHELL:-}" in */fish)
            command -v fish >/dev/null 2>&1 || fail 'The configured fish shell is unavailable.'
            fish -c 'fish_add_path $argv[1]' "$bin"
            ;; esac
    fi
    mv -f "$pending" "$bin/kess"
    printf '\nInstalled Kessetsu %s in %s\nOpen a NEW terminal, then run: kess --version\n' "$version" "$root"
    if [ "$no_path" = true ]; then printf 'PATH was not changed. Run: "%s/kess" --version\n' "$bin"; fi
    if ! command -v ngspice >/dev/null 2>&1; then
        printf '\nCompile and export work now. Simulation also needs Ngspice:\n'
        if [ "$system" = Darwin ]; then
            printf '  With Homebrew: brew install ngspice\n  Without Homebrew: https://brew.sh (install it first)\n'
        else
            printf '  Ubuntu/Debian: sudo apt-get update && sudo apt-get install ngspice\n  Other Linux: install ngspice with your distribution package manager.\n'
        fi
    fi
    printf '\nUpdate: run the same installation command again. Old bundles are kept for recovery.\n'
}

install_kessetsu "$@"
