#!/bin/bash
set -e

# ============================================================
# WaLiOffice Docker 镜像构建 + 推送
# 用法: ./build.sh
# 前提: Docker Desktop 已启动，已登录阿里云镜像仓库
# ============================================================

# 镜像配置
ALIYUN_REGISTRY="registry.cn-hangzhou.aliyuncs.com"
NAMESPACE="fuzhengwei"
IMAGE_NAME="walioffice"
IMAGE_TAG="1.0"
FULL_IMAGE="${ALIYUN_REGISTRY}/${NAMESPACE}/${IMAGE_NAME}:${IMAGE_TAG}"

echo "🔧 开始构建 WaLiOffice Docker 镜像..."
echo "   镜像地址: ${FULL_IMAGE}"
echo "   支持平台: linux/amd64, linux/arm64"
echo ""

# 读取阿里云登录凭证
if [ -f ".local-config" ]; then
  source .local-config
else
  echo "❌ .local-config 文件不存在，请创建并填写："
  echo '   ALIYUN_USERNAME="你的阿里云账号"'
  echo '   ALIYUN_PASSWORD="你的阿里云密码"'
  exit 1
fi

# 登录阿里云镜像仓库
echo "🔐 登录阿里云镜像仓库..."
docker login --username="${ALIYUN_USERNAME}" --password="${ALIYUN_PASSWORD}" ${ALIYUN_REGISTRY}

# 确保 buildx builder 可用
if ! docker buildx inspect moyu-builder >/dev/null 2>&1; then
  echo "📦 创建 buildx builder..."
  docker buildx create --name moyu-builder --use
else
  docker buildx use moyu-builder
fi

# 构建并推送多平台镜像
echo ""
echo "🏗️  构建并推送镜像（多平台）..."
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t "${FULL_IMAGE}" \
  -f ./Dockerfile \
  --push \
  .

echo ""
echo "✅ 构建推送完成！"
echo ""
echo "📋 云服务器部署步骤："
echo "   1. 保存以下文件到服务器: docker-compose.yml + .env"
echo "   2. 修改 .env 中的 API Key 和 JWT_SECRET"
echo "   3. 执行: docker compose pull && docker compose up -d"
echo ""
echo "🔗 镜像拉取地址: docker pull ${FULL_IMAGE}"
echo ""

# 登出
docker logout ${ALIYUN_REGISTRY}
