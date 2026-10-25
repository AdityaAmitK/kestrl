#!/bin/sh
#
# One-time BPF access for Kestrl port capture on macOS (ChmodBPF-style).
#
#   sudo sh scripts/kestrl-bpf.sh install --user <name>
#       Creates the kestrl_bpf group, adds <name> to it, installs this script
#       at a fixed root-owned path, and loads a LaunchDaemon that gives
#       kestrl_bpf access to /dev/bpf* at every boot.
#
#   sudo sh scripts/kestrl-bpf.sh uninstall [--user <name>]
#       Removes <name> from kestrl_bpf. When the group has no members left,
#       also unloads the LaunchDaemon, removes the installed files and the
#       group, and restores the /dev/bpf* devices this script assigned.
#
#   apply   Run by the LaunchDaemon at boot. Root only.
#
# Membership in kestrl_bpf lets that user read every packet on every
# interface through BPF, not only lo0. Kestrl's capture filter limits what
# Kestrl reads; it does not limit what the group can read.
#
# Refuses to install while Wireshark's ChmodBPF daemon is present, because
# both would set the group on the same /dev/bpf* devices.

set -eu
PATH=/usr/bin:/bin:/usr/sbin:/sbin
export PATH
umask 077

GROUP=kestrl_bpf
LABEL=dev.kestrl.bpf
HELPER_DIR="/Library/Application Support/Kestrl"
HELPER="$HELPER_DIR/kestrl-bpf.sh"
PLIST=/Library/LaunchDaemons/dev.kestrl.bpf.plist
MARKER="$HELPER_DIR/installed-v1"
CHMODBPF_PLIST=/Library/LaunchDaemons/org.wireshark.ChmodBPF.plist

die() {
    printf 'kestrl-bpf: %s\n' "$*" >&2
    exit 1
}

# Refuse symlinks and writable root-run paths, including pre-existing directories.
check_owned_path() {
    [ ! -L "$1" ] || die "refusing symlink: $1"
    [ -e "$1" ] || return 0
    [ "$(stat -f %u "$1")" -eq 0 ] || die "not root-owned: $1"
    mode=$(stat -f %Lp "$1")
    [ "$((0$mode & 022))" -eq 0 ] || die "group/world-writable path: $1"
}

check_paths() {
    for path in /Library "/Library/Application Support" /Library/LaunchDaemons "$HELPER_DIR" "$HELPER" "$PLIST" "$MARKER"; do
        check_owned_path "$path"
    done
}

is_managed() {
    [ -f "$MARKER" ] && [ "$(cat "$MARKER")" = "Kestrl BPF setup v1" ]
}

usage() {
    printf '%s\n' \
        'usage: sudo sh kestrl-bpf.sh install --user <name>' \
        '       sudo sh kestrl-bpf.sh uninstall [--user <name>]' >&2
    exit 2
}

require_root() {
    [ "$(id -u)" -eq 0 ] || die "run with sudo"
}

# Syntax only: lowercase login name, 1 to 31 characters, not starting with a digit or hyphen.
check_name() {
    case $1 in
        ''|*[!a-z0-9_-]*|[!a-z_]*) die "invalid user name: $1" ;;
    esac
    [ "${#1}" -le 31 ] || die "user name too long: $1"
}

# Existence and class: a local account with a non-system uid. Never root.
check_user() {
    check_name "$1"
    uid=$(dscl . -read "/Users/$1" UniqueID 2>/dev/null | awk '{print $2}')
    [ -n "$uid" ] || die "no local user named $1"
    [ "$uid" -ge 500 ] || die "$1 is a system account (uid $uid)"
}

group_exists() {
    dscl . -read "/Groups/$GROUP" PrimaryGroupID >/dev/null 2>&1
}

is_member() {
    dseditgroup -o checkmember -m "$1" "$GROUP" >/dev/null 2>&1
}

has_members() {
    [ -n "$(dscl . -read "/Groups/$GROUP" GroupMembership 2>/dev/null \
        | sed 's/^GroupMembership://' | tr -d ' \t\n')" ]
}

# Assign each BPF device to kestrl_bpf with group read/write, as ChmodBPF does for access_bpf.
apply_devices() {
    require_root
    check_paths
    is_managed || die "no Kestrl-managed BPF setup"
    [ ! -e "$CHMODBPF_PLIST" ] || die "Wireshark now manages BPF; refusing to override it"
    group_exists || die "group $GROUP does not exist"
    for dev in /dev/bpf[0-9]*; do
        [ -c "$dev" ] || continue
        current_group=$(stat -f %Sg "$dev")
        current_mode=$(stat -f %Lp "$dev")
        if [ "$current_group" != "$GROUP" ] && { [ "$current_group" != wheel ] || [ "$current_mode" != 600 ]; }; then
            die "another setup owns $dev; refusing to replace its permissions"
        fi
        chgrp "$GROUP" "$dev"
        chmod 0660 "$dev"
    done
}

# Restore only devices that still belong to kestrl_bpf, to the macOS default root:wheel 0600.
restore_devices() {
    for dev in /dev/bpf[0-9]*; do
        [ -c "$dev" ] || continue
        if [ "$(stat -f %Sg "$dev")" = "$GROUP" ]; then
            chgrp wheel "$dev"
            chmod 0600 "$dev"
        fi
    done
}

install_access() {
    require_root
    [ "$#" -eq 2 ] && [ "$1" = --user ] || usage
    check_user "$2"
    [ -f "$0" ] || die "run this script from a file"
    if [ -e "$CHMODBPF_PLIST" ]; then
        die "Wireshark ChmodBPF manages /dev/bpf* for access_bpf; Kestrl does not reassign those devices"
    fi

    check_paths
    if group_exists && ! is_managed; then
        die "group $GROUP already exists without a Kestrl ownership marker"
    fi
    if { [ -e "$HELPER" ] || [ -e "$PLIST" ]; } && ! is_managed; then
        die "Kestrl setup paths already exist without an ownership marker"
    fi
    install -d -o root -g wheel -m 0755 "$HELPER_DIR"
    install -o root -g wheel -m 0755 "$0" "$HELPER"
    printf '%s\n' 'Kestrl BPF setup v1' > "$MARKER"
    chown root:wheel "$MARKER"
    chmod 0600 "$MARKER"
    group_exists || dseditgroup -o create -r "Kestrl BPF access" "$GROUP"
    dseditgroup -o edit -a "$2" -t user "$GROUP"

    tmp=$(mktemp)
    trap 'rm -f "$tmp"' EXIT
    cat > "$tmp" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>$LABEL</string>
    <key>ProgramArguments</key>
    <array>
        <string>/bin/sh</string>
        <string>$HELPER</string>
        <string>apply</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
</dict>
</plist>
PLIST
    rm -f "$PLIST"
    install -o root -g wheel -m 0644 "$tmp" "$PLIST"

    launchctl bootout "system/$LABEL" 2>/dev/null || true
    launchctl bootstrap system "$PLIST"
    apply_devices
    printf 'kestrl-bpf: %s may now capture after logging out and back in\n' "$2"
}

uninstall_access() {
    require_root
    check_paths
    is_managed || die "no Kestrl-managed installation; refusing to remove unowned artifacts"
    name=
    if [ "$#" -eq 2 ] && [ "$1" = --user ]; then
        check_name "$2"
        name=$2
    elif [ "$#" -ne 0 ]; then
        usage
    fi

    if [ -n "$name" ] && group_exists && is_member "$name"; then
        dseditgroup -o edit -d "$name" -t user "$GROUP"
    fi
    if group_exists && has_members; then
        printf 'kestrl-bpf: %s still has members; kept the LaunchDaemon and group\n' "$GROUP"
        exit 0
    fi

    launchctl bootout "system/$LABEL" 2>/dev/null || true
    restore_devices
    rm -f "$PLIST" "$HELPER" "$MARKER"
    rmdir "$HELPER_DIR" 2>/dev/null || true
    if group_exists; then
        dseditgroup -o delete "$GROUP"
    fi
    printf 'kestrl-bpf: removed Kestrl BPF access\n'
}

case ${1:-} in
    install)
        shift
        install_access "$@"
        ;;
    uninstall)
        shift
        uninstall_access "$@"
        ;;
    apply)
        apply_devices
        ;;
    *)
        usage
        ;;
esac
