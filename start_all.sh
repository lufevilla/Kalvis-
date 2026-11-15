#!/bin/bash
# Script para iniciar backend Rust + frontend Flutter automaticamente

set -e

BACKEND_DIR="/home/felipe/dev/hackatons/NASA SPACE APPS/back-end"
FRONTEND_DIR="/home/felipe/dev/hackatons/NASA SPACE APPS/front_end"

echo "🚀 Iniciando Space Health - Backend + Frontend"

# Função para cleanup ao sair
cleanup() {
    echo ""
    echo "🛑 Parando processos..."
    if [ ! -z "$BACKEND_PID" ]; then
        kill $BACKEND_PID 2>/dev/null || true
    fi
    if [ ! -z "$FRONTEND_PID" ]; then
        kill $FRONTEND_PID 2>/dev/null || true
    fi
    exit 0
}

trap cleanup SIGINT SIGTERM EXIT

# 1. Verificar/instalar dependências do backend
echo "📦 Verificando backend..."
cd "$BACKEND_DIR"
if ! command -v cargo &> /dev/null; then
    echo "❌ Rust/Cargo não encontrado. Instale: https://rustup.rs/"
    exit 1
fi

# 2. Build backend
echo "🔨 Compilando backend..."
cargo build --release 2>&1 | tail -5

# 3. Iniciar backend em background
echo "🟢 Iniciando backend na porta 3000..."
cd "$BACKEND_DIR"
./target/release/back-end &
BACKEND_PID=$!

# Aguardar backend subir
sleep 3

# Verificar se backend está respondendo
for i in {1..10}; do
    if curl -s http://localhost:3000/tracking?astronauta_id=AST001 > /dev/null 2>&1; then
        echo "✅ Backend respondendo em http://localhost:3000"
        break
    fi
    if [ $i -eq 10 ]; then
        echo "❌ Backend não respondeu após 10 tentativas"
        exit 1
    fi
    sleep 1
done

# 4. Verificar/instalar dependências do frontend
echo "📦 Verificando frontend..."
cd "$FRONTEND_DIR"
if ! command -v flutter &> /dev/null; then
    echo "❌ Flutter não encontrado. Instale: https://flutter.dev/docs/get-started/install"
    exit 1
fi

flutter pub get

# 5. Iniciar frontend
echo "🟢 Iniciando frontend..."
flutter run -d linux &
FRONTEND_PID=$!

echo ""
echo "✅ Ambos rodando!"
echo "   Backend:  http://localhost:3000 (PID: $BACKEND_PID)"
echo "   Frontend: Flutter Linux (PID: $FRONTEND_PID)"
echo ""
echo "Pressione Ctrl+C para parar ambos"

# Aguardar processos
wait $BACKEND_PID $FRONTEND_PID