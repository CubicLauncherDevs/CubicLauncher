#!/usr/bin/env bash
# Script auxiliar para crear los usuarios/grupo de build que Nix necesita
# en una instalación multi-usuario. Ejecutar como root.
set -euo pipefail

NIX_GROUP="nixbld"
NIX_USERS=32
# GID estándar de NixOS para build users. Si colisiona, comentar la línea
# siguiente y dejar que groupadd elija uno automáticamente.
NIX_GID="${NIX_GID:-30000}"

if (( EUID != 0 )); then
  echo "[ERROR] Ejecutar como root (p. ej. con sudo)." >&2
  exit 1
fi

store_conf_value() {
  local file="$1" key="$2" val="$3"
  if [[ -f "$file" ]]; then
    if grep -qE "^\s*#?\s*${key}\s*=" "$file"; then
      sed -i -E "s|^\s*#?\s*(${key}\s*=).*|\1 ${val}|" "$file"
    else
      echo "${key} = ${val}" >> "$file"
    fi
  else
    echo "${key} = ${val}" > "$file"
  fi
}

if ! getent group "$NIX_GROUP" >/dev/null; then
  echo "Creando grupo ${NIX_GROUP}..."
  if [[ -n "${NIX_GID}" ]]; then
    groupadd -r -g "$NIX_GID" "$NIX_GROUP" || groupadd -r "$NIX_GROUP"
  else
    groupadd -r "$NIX_GROUP"
  fi
else
  echo "Grupo ${NIX_GROUP} ya existe."
fi

for i in $(seq 1 "$NIX_USERS"); do
  user="nixbld${i}"
  if ! id "$user" >/dev/null 2>&1; then
    uid=$((NIX_GID + i))
    useradd -r -g "$NIX_GROUP" -d /var/empty -s /usr/sbin/nologin \
      -u "$uid" "$user" 2>/dev/null || useradd -r -g "$NIX_GROUP" \
        -d /var/empty -s /usr/sbin/nologin "$user"
    echo "Creado ${user}."
  fi
done

echo "Configurando /etc/nix/nix.conf..."
store_conf_value /etc/nix/nix.conf build-users-group "$NIX_GROUP"

echo "Reiniciando nix-daemon..."
if systemctl is-active nix-daemon >/dev/null 2>&1; then
  systemctl restart nix-daemon
elif command -v rc-service >/dev/null 2>&1 && rc-service nix-daemon status >/dev/null 2>&1; then
  rc-service nix-daemon restart
elif pgrep -x nix-daemon >/dev/null; then
  pkill -HUP nix-daemon
fi

echo "Hecho. Verificá con: getent group ${NIX_GROUP}"
