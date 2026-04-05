#!/usr/bin/env bash
set -e

echo "=> Buscando dependencias (Rust/Cargo)..."
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: 'cargo' no está instalado."
    echo "Instala Rust primero desde https://rustup.rs (ejecutando: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh)"
    exit 1
fi

echo "=> Instalando Git-Lore globalmente en tu sistema desde el repositorio remoto..."
cargo install --git https://github.com/JussMor/git-lore.git --package git-lore "$@"

if [[ "$*" == *"semantic-search"* ]]; then
    echo ""
    echo "=> Flag 'semantic-search' detectado. Descargando modelos ONNX de IA locales (~120MB)..."
    mkdir -p ~/.cache/memvid/text-models
    curl -# -L 'https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/onnx/model.onnx' -o ~/.cache/memvid/text-models/bge-small-en-v1.5.onnx
    curl -# -L 'https://huggingface.co/BAAI/bge-small-en-v1.5/resolve/main/tokenizer.json' -o ~/.cache/memvid/text-models/bge-small-en-v1.5_tokenizer.json
    echo "✅ Modelos ONNX configurados en ~/.cache/memvid/text-models/"
fi

echo ""
echo "✅ Git-Lore instalado con éxito!"
echo "Puedes probarlo ejecutando: git-lore --help"
