#!/bin/bash
# Read-only report about this Mac for tuning Trufi Cleaner's macOS layer.
# Nothing is changed or deleted. Paths in the report include your user
# name; look through it before sharing.
#
#   bash scripts/mac-diagnostics.sh            # quick
#   bash scripts/mac-diagnostics.sh --updates  # also ask Software Update (slow, goes online)
#
# Writes trufi-mac-report.txt in the current folder.

OUT="trufi-mac-report.txt"
exec > >(tee "$OUT") 2>&1

section() { printf '\n===== %s =====\n' "$1"; }
size() { [ -e "$1" ] && du -sh "$1" 2>/dev/null | cut -f1 || echo "-"; }

section "System"
sw_vers
uname -m
sysctl -n hw.memsize | awk '{printf "RAM: %.1f GB\n", $1/1073741824}'
echo "memorystatus_level (free %): $(sysctl -n kern.memorystatus_level 2>/dev/null)"
echo "vm_pressure_level (1 normal, 2 warn, 4 critical): $(sysctl -n kern.memorystatus_vm_pressure_level 2>/dev/null)"
memory_pressure 2>/dev/null | tail -1

section "Volumes"
df -h | grep -E '^/dev|Filesystem'

section "Full Disk Access of this terminal"
if ls ~/Library/Safari >/dev/null 2>&1; then echo "yes"; else echo "no (Trash, Safari, Mail sizes below may be '-')"; fi

section "Sizes"
for p in ~/.Trash ~/Library/Caches ~/Library/Logs "$TMPDIR" ~/Library/Developer/Xcode/DerivedData \
  ~/Library/Developer/Xcode/Archives "$HOME/Library/Developer/Xcode/iOS DeviceSupport" \
  ~/Library/Developer/CoreSimulator ~/Library/Caches/Homebrew ~/.npm ~/Library/Caches/pip ~/.cargo/registry \
  ~/.gradle/caches "$HOME/Library/Application Support/MobileSync/Backup" "$HOME/Library/Containers/com.apple.mail/Data/Library/Mail Downloads" \
  ~/Library/Caches/Google/Chrome "$HOME/Library/Application Support/Google/Chrome" ~/Library/Caches/com.apple.Safari \
  "$HOME/Library/Application Support/Telegram Desktop" ~/Library/Group\ Containers/*.ru.keepcoder.Telegram \
  ~/Library/Containers/com.docker.docker; do
  printf '%8s  %s\n' "$(size "$p")" "${p/#$HOME/~}"
done

section "Biggest folders in ~/Library/Caches"
du -sh ~/Library/Caches/* 2>/dev/null | sort -rh | head -15

section "Biggest folders in ~/Library/Application Support"
du -sh ~/Library/Application\ Support/* 2>/dev/null | sort -rh | head -20

section "Applications"
ls /Applications ~/Applications 2>/dev/null | grep -c '\.app$' | sed 's/^/count: /'

section "Launch agents and daemons"
ls ~/Library/LaunchAgents /Library/LaunchAgents /Library/LaunchDaemons 2>/dev/null
echo "--- launchctl print-disabled gui/$(id -u)"
launchctl print-disabled "gui/$(id -u)" 2>/dev/null | head -40

section "Time Machine local snapshots"
tmutil listlocalsnapshots / 2>&1 | head -20

section "Xcode"
xcode-select -p 2>&1
if xcode-select -p 2>/dev/null | grep -q '\.app/'; then
  xcrun simctl list devices unavailable 2>&1 | head -20
fi

section "Docker"
for d in /usr/local/bin/docker /opt/homebrew/bin/docker ~/.docker/bin/docker; do
  if [ -x "$d" ]; then "$d" system df 2>&1; break; fi
done

section "Last system updates"
plutil -convert json -o - /Library/Receipts/InstallHistory.plist 2>/dev/null |
  python3 -c 'import json,sys; [print(e.get("date"), e.get("processName"), e.get("contentType",""), e.get("displayName")) for e in json.load(sys.stdin)[-15:]]' 2>/dev/null

if [ "$1" = "--updates" ]; then
  section "softwareupdate --list"
  softwareupdate --list 2>&1
fi

echo
echo "Report saved to $(pwd)/$OUT"
