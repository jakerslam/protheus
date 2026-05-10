#!/usr/bin/env sh
# SPDX-License-Identifier: Apache-2.0
# Layer ownership: installer/modules

infring_install_status() {
  printf '[infring install] %s\n' "$*"
}

infring_install_mkdir() {
  mkdir -p "$1"
}

infring_install_completion_card() {
  version="$1"
  location="$2"
  command="${3:-infring --help}"
  printf 'Setting up InfRing...\n\n'
  printf '\033[32m%s\033[0m\n\n' '✔ InfRing successfully installed!'
  printf '  Version: \033[33m%s\033[0m\n' "$version"
  printf '  Location: %s\n\n' "$location"
  printf '  Next: Run \033[33m%s\033[0m to get started.\n\n' "$command"
  printf '\033[32m%s\033[0m\n' 'Installation complete!'
}
