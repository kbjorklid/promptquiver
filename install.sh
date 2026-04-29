#!/bin/bash
set -e

[[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

cargo build --release -p promptquiver
cp target/release/promptquiver ~/.local/bin/quiver
codesign --force --sign - ~/.local/bin/quiver

echo "Installed quiver to ~/.local/bin/quiver"
