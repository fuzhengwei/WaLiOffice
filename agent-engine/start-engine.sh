#!/bin/bash
# DSH Agent Engine 启动脚本
# 以 HTTP 服务模式启动 DSH，监听 port 3780

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# 环境变量
export LLM_TEXT_BASE_URL="${LLM_TEXT_BASE_URL:-https://apihub.agnes-ai.com/v1}"
export LLM_TEXT_API_KEY="${LLM_TEXT_API_KEY:-}"
export LLM_TEXT_MODELS="${LLM_TEXT_MODELS:-agnes-2.5-flash}"
export LLM_TEXT_MODEL_DEFAULT="${LLM_TEXT_MODEL_DEFAULT:-agnes-2.5-flash}"
export RUST_BACKEND_URL="${RUST_BACKEND_URL:-http://127.0.0.1:8000}"
export DSH_PORT="${DSH_PORT:-3780}"

echo "🚀 DSH Agent Engine 启动中..."
echo "   Port: $DSH_PORT"
echo "   Rust Backend: $RUST_BACKEND_URL"
echo "   LLM: $LLM_TEXT_BASE_URL ($LLM_TEXT_MODELS)"

# 用 node 直接启动，保持 HTTP 服务常驻
exec npx dsh --profile walioffice "服务已启动，等待请求" 2>&1
