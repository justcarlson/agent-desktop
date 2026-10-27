#!/usr/bin/env bash
set -euo pipefail

FORMULA=${1:?Usage: homebrew.sh FORMULA_PATH}
test -f "$FORMULA"
TAP=lahfir/agent-desktop-ci
TAP_PATH="$(brew --repository)/Library/Taps/lahfir/homebrew-agent-desktop-ci"
test ! -e "$TAP_PATH"
test ! -e "$(brew --cellar)/agent-desktop"
mkdir -p "$TAP_PATH/Formula"
git -C "$TAP_PATH" init
git -C "$TAP_PATH" -c user.name=Lahfir -c user.email=70215676+lahfir@users.noreply.github.com \
  commit --allow-empty -m 'test: initialize native installation tap'
cp "$FORMULA" "$(brew --repo "$TAP")/Formula/agent-desktop.rb"
brew install "$TAP/agent-desktop"
brew test "$TAP/agent-desktop"
PREFIX=$(brew --prefix)
BINARY="$PREFIX/bin/agent-desktop"
KEG=$(brew --prefix "$TAP/agent-desktop")
test -L "$BINARY"
test "$BINARY" -ef "$KEG/bin/agent-desktop"
case "$(uname -s)" in
  Darwin) HELPER=agent-desktop-macos-helper ;;
  Linux) HELPER=agent-desktop-linux-clipboard-owner ;;
esac
test "$PREFIX/bin/$HELPER" -ef "$KEG/bin/$HELPER"
test "$(head -c 2 "$BINARY")" != '#!'
env -i HOME="$HOME" PATH="$KEG/bin" agent-desktop version
env -i HOME="$HOME" PATH="$KEG/bin" agent-desktop permissions
brew unlink agent-desktop
brew link agent-desktop
test "$BINARY" -ef "$KEG/bin/agent-desktop"
brew uninstall "$TAP/agent-desktop"
test ! -e "$BINARY"
test ! -L "$BINARY"
test ! -e "$PREFIX/bin/$HELPER"
brew untap "$TAP"
