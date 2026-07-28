#!/bin/bash
# 快速修复脚本：更新模型名称映射并清除缓存
# 使用方法：chmod +x fix-model-names.sh && ./fix-model-names.sh

set -e

SSH_CMD="ssh -i ~/.ssh/michael_server root@154.44.13.133"
POSTGRES_UPDATE='
UPDATE models 
SET model_names = '\''{"claude-fable-5": "Claude Fable 5", 
                      "claude-opus-4-6": "Claude Opus 4.6", 
                      "claude-opus-4-7": "Claude Opus 4.7", 
                      "claude-opus-4-8": "Claude Opus 4.8", 
                      "claude-opus-5": "Claude Opus 5", 
                      "claude-sonnet-5": "Claude Sonnet 5"}'\''::jsonb
WHERE provider = '\''claude'\'';
'

echo "=========================================="
echo "🔧 Michael IDE 模型名称映射修复脚本"
echo "=========================================="
echo ""

echo "📡 连接到服务器..."
$SSH_CMD "echo ✅ 连接成功"
echo ""

echo "💾 更新数据库模型名称映射..."
$SSH_CMD "docker exec server-postgres-1 psql -U michael -d michael -c \"$POSTGRES_UPDATE\""
echo "✅ 数据库更新完成"
echo ""

echo "🔄 清理 Redis 缓存（如果有）..."
read -p "是否清空 Redis 缓存？[y/N]: " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    $SSH_CMD "docker exec server-redis-1 redis-cli FLUSHALL"
    echo "✅ Redis 已清空"
else
    echo "⏭️ 跳过 Redis 清理"
fi
echo ""

echo "🔁 重启后端服务以确保配置生效..."
read -p "是否重启后端容器？[Y/n]: " -n 1 -r
echo
if [[ -z $REPLY || $REPLY =~ ^[Yy]$ ]]; then
    $SSH_CMD "docker compose -p server restart backend"
    echo "✅ 后端容器已重启"
else
    echo "⏭️ 跳过重启（/api/models 会实时查询数据库，通常不需要重启）"
fi
echo ""

echo "=========================================="
echo "✨ 修复完成！"
echo "=========================================="
echo ""
echo " 请在 IDE 中执行以下操作："
echo "   1. 硬刷新：Cmd + Shift + R (macOS)"
echo "   2. 重新选择模型看看名称是否正确"
echo ""
echo "如果问题仍存在，请检查："
echo "   - localStorage 中的自定义模型配置"
echo "   - /api/models 端点的实际返回内容"
echo ""
echo "验证命令（在服务器上运行）："
echo "  curl http://127.0.0.1:8080/api/models | python3 -m json.tool"
echo ""
