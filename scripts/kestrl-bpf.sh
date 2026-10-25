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

GROUP=kestrl_bpf
LABEL=dev.kestrl.bpf
HELPER_DIR="/Library/Application Support/Kestrl"
HELPER="$HELPER_DIR/kestrl-bpf.sh"
PLIST=/Library/LaunchDaemons/dev.kestrl.bpf.plist
CHMODBPF_PLIST=/Library/LaunchDaemons/org.wireshark.ChmodBPF.plist

die() {
    printf 'kestrl-bpf: %s\n' "$*" >&2
    exit 1
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
    group_exists || die "group $GROUP does not exist"
    for dev in /dev/bpf[0-9]*; do
        [ -c "$dev" ] || continue
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

    group_exists || dseditgroup -o create -r "Kestrl BPF access" "$GROUP"
    dseditgroup -o edit -a "$2" -t user "$GROUP"

    install -d -o root -g wheel -m 0755 "$HELPER_DIR"
    install -o root -g wheel -m 0755 "$0" "$HELPER"

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
    rm -f "$PLIST" "$HELPER"
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
