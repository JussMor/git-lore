#!/usr/bin/env bash
set -e

echo "=> Buscando dependencias (Rust/Cargo)..."
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: 'cargo' no está instalado."
    echo "Instala Rust primero desde https://rustup.rs (ejecutando: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)"
    exit 1
fi

echo "=> Instalando Git-Lore globalmente en tu sistema desde el repositorio remoto..."
cargo install --git https://github.com/JussMor/git-lore.git "$@"

echo ""
echo "✅ Git-Lore instalado con éxito!"
echo "Puedes probarlo ejecutando: git-lore --help"
