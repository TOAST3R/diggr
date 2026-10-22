#!/bin/sh
# Installs Diggr for this user only (no root): run it from the extracted release folder.
# Running it again replaces the installed copy. To uninstall, delete the three files it prints.
set -eu
cd "$(dirname "$0")"

data=${XDG_DATA_HOME:-$HOME/.local/share}
bin=$HOME/.local/bin/diggr
icon=$data/icons/hicolor/256x256/apps/diggr.png
entry=$data/applications/diggr.desktop

mkdir -p "$(dirname "$bin")" "$(dirname "$icon")" "$(dirname "$entry")"
# Copy then rename, so a running copy keeps its file.
cp diggr "$bin.new" && chmod 755 "$bin.new" && mv -f "$bin.new" "$bin"
cp diggr.png "$icon"
# A desktop session's PATH may not include ~/.local/bin, so the entry names the binary in full.
sed "s|^Exec=diggr|Exec=$bin|" diggr.desktop >"$entry"

# Menus pick the entry up without these too; they only make it show sooner.
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database "$data/applications" || true
fi
if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    gtk-update-icon-cache -q -t "$data/icons/hicolor" || true
fi

echo "Installed Diggr:"
echo "  $bin"
echo "  $icon"
echo "  $entry"
