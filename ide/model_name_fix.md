# 解决模型名称不一致问题

## 问题分析

**现象**:
- 服务器上实际开放的模型：`claude-opus-5`, `claude-sonnet-5` 等
- IDE 选择器中显示的模型名称：`claude-4.6-sonnect`（或其他不匹配的名称）

**根本原因**:
IDE 中的模型列表来自服务器的 `/api/models` 端点响应。服务器会返回每个模型的 `model_id`（唯一标识符）和 `name`（友好显示名称）。如果这两个值不一致，说明服务器数据库中的 `model_names` 映射配置有问题。

---

## 🔍 当前状态检查

### 1. 服务器模型配置（PostgreSQL 数据库）

```sql
SELECT id, label, provider, enabled_models, model_names FROM models WHERE active = true;
```

**结果**:
```
id: 764fe78b-69a3-49ca-9bc1-4b8ef3050778
label: Claude
provider: claude
enabled_models: {claude-fable-5,claude-opus-4-6,claude-opus-4-7,claude-opus-4-8,claude-opus-5,claude-sonnet-5}
model_names: {"claude-opus-5": "claude-opus-5"}  # ⚠️ 只配置了一个映射
```

### 2. `/api/models` 端点输出

```json
{
  "model_id": "claude-opus-5",
  "name": "claude-opus-5",
  "provider": "claude",
  "group": "Claude"
}
```

**✅ 服务器返回的数据是正确的！**

### 3. IDE 中显示的模型名称

根据截图中看到的模型选择器：
- ✅ 正常显示：`Claude Fable 5`, `Claude Opus 4.6`, `Claude Opus 4.7`, `Claude Opus 4.8`
- ⚠️ **问题来源**：您提到的"claude-4.6-sonnect"可能来自：
  1. **自定义模型配置**（Custom Models）
  2. **浏览器缓存的旧数据**
  3. **IDE 本地配置的遗留项**

---

## 🛠️ 解决方案

### 方案 A：修复服务器端的模型名称映射（推荐）

**目的**: 为所有模型添加友好的显示名称映射

**步骤**:

1. **连接到数据库**:
   ```bash
   ssh -i ~/.ssh/michael_server root@154.44.13.133
   docker exec -it server-postgres-1 psql -U michael -d michael
   ```

2. **查看当前模型配置**:
   ```sql
   -- 查看 Claude 连接的 model_names 字段
   SELECT id, label, model_names FROM models WHERE provider = 'claude';
   
   -- 查看完整的模型列表
   SELECT model_id, name FROM json_array_elements_text(enabled_models);
   ```

3. **更新 model_names 映射**:
   ```sql
   UPDATE models 
   SET model_names = '{"claude-fable-5": "Claude Fable 5", 
                        "claude-opus-4-6": "Claude Opus 4.6", 
                        "claude-opus-4-7": "Claude Opus 4.7", 
                        "claude-opus-4-8": "Claude Opus 4.8", 
                        "claude-opus-5": "Claude Opus 5", 
                        "claude-sonnet-5": "Claude Sonnet 5"}'::jsonb
   WHERE provider = 'claude';
   ```

4. **验证更新**:
   ```sql
   SELECT id, label, model_names FROM models WHERE provider = 'claude';
   ```

5. **重启后端容器** (可选，因为 `/api/models` 是实时查询数据库):
   ```bash
   docker compose -p server restart backend
   ```

---

### 方案 B：清理 IDE 中的自定义模型配置

如果您的 IDE 中有自定义的 OpenAI 兼容模型配置，它们可能会显示在主模型列表中。

**检查步骤**:

1. **打开 IDE**,进入开发者工具:
   - macOS: `Cmd + Option + I`
   - 或：右键 → 检查

2. **在 Console 中运行**:
   ```javascript
   // 查看本地存储的自定义模型配置
   console.log(localStorage.getItem('michael_custom_models_v1'));
   ```

3. **如果看到自定义模型配置，执行清理**:
   ```javascript
   localStorage.removeItem('michael_custom_models_v1');
   location.reload();  // 刷新页面
   ```

**注意**: 这会删除所有自定义的 OpenAI 兼容接入点配置。

---

### 方案 C：清除浏览器/IDE 缓存

**方法 1**: 硬刷新
- Windows/Linux: `Ctrl + Shift + R`
- macOS: `Cmd + Shift + R`

**方法 2**: 清除应用数据并重新登录
```javascript
// 在 IDE Console 中运行
localStorage.clear();
sessionStorage.clear();
location.reload();
```

然后重新登录账号。

---

### 方案 D：检查是否有临时缓存模型

有时候服务器可能在测试阶段使用临时模型 ID，这些会被缓存到 IDE 中。

**清理服务器 Redis 缓存**:
```bash
docker exec -it server-redis-1 redis-cli
FLUSHALL  # 清空所有缓存（谨慎使用）
exit
```

然后**重启后端容器**:
```bash
docker compose -p server restart backend
```

---

## 📊 排查清单

### 立即执行的检查

1. ✅ **确认 `/api/models` 返回内容**
   ```bash
   curl http://127.0.0.1:8080/api/models | python3 -m json.tool
   ```

2. ✅ **检查 IDE 是否显示正确的 `model_id`**
   - 点击选择器中的任意模型
   - 查看底部的模型标签是否与实际一致

3. ✅ **验证数据库配置**
   ```sql
   SELECT model_names FROM models WHERE provider = 'claude';
   ```

### 深度排查

如果以上步骤后问题仍然存在：

1. **查看 IDE 代码中的模型重命名逻辑**:
   ```javascript
   // main.js 第 12234-12246 行
   function modelLabel(id = "") {
     if (id === "devin") return "Devin";
     if (id && id.startsWith && id.startsWith("custom:")) {
       const custom = _customModelById(id);
       if (custom) return custom.name;
     }
     return MODEL_NAMES[id] || id;
   }
   ```

2. **检查是否有其他模型组包含该名称**:
   ```bash
   curl http://127.0.0.1:8080/api/models | grep -i "4.6\|sonnect"
   ```

3. **检查 Grok/GPT 等其他提供商**:
   ```sql
   SELECT id, provider, enabled_models FROM models WHERE active = true;
   ```

---

## 🎯 快速修复脚本

如果您确定要更新所有 Claude 模型的名称映射，可以直接运行这个脚本：

```bash
#!/bin/bash
# 更新模型名称映射

SSH_CMD="ssh -i ~/.ssh/michael_server root@154.44.13.133"

$SSH_CMD "docker exec server-postgres-1 psql -U michael -d michael << EOF
UPDATE models 
SET model_names = '{\"claude-fable-5\": \"Claude Fable 5\", 
                     \"claude-opus-4-6\": \"Claude Opus 4.6\", 
                     \"claude-opus-4-7\": \"Claude Opus 4.7\", 
                     \"claude-opus-4-8\": \"Claude Opus 4.8\", 
                     \"claude-opus-5\": \"Claude Opus 5\", 
                     \"claude-sonnet-5\": \"Claude Sonnet 5\"}'::jsonb
WHERE provider = 'claude';
EOF"

echo "✅ 模型名称映射已更新"
echo "🔄 建议重启后端容器..."
$SSH_CMD "docker compose -p server restart backend"
```

---

## 🔍 模型 ID vs 显示名称对照表

| 实际模型 ID | 建议显示名称 | 用途 |
|-----------|-------------|------|
| `claude-fable-5` | Claude Fable 5 | 创意写作辅助 |
| `claude-opus-4-6` | Claude Opus 4.6 | 高端复杂任务 |
| `claude-opus-4-7` | Claude Opus 4.7 | 高端复杂任务 |
| `claude-opus-4-8` | Claude Opus 4.8 | 高端复杂任务（当前默认） |
| `claude-opus-5` | Claude Opus 5 | 最新旗舰版本 |
| `claude-sonnet-5` | Claude Sonnet 5 | 平衡性能与成本 |

**重要**: IDE 发送请求时使用 `model_id`，而用户看到的 UI 中使用 `name`。两者必须通过数据库的 `model_names` 映射关联。

---

## 💡 未来预防建议

### 1. 部署前检查清单
在每次部署后端时，验证模型配置一致性：
```bash
# 检查所有 active 模型的 model_names
psql -U michael -d michael -c "SELECT id, label, model_names FROM models WHERE active = true;"
```

### 2. 自动化同步脚本
创建同步脚本确保 `enabled_models` 中的所有模型都有对应的 `model_names` 条目。

### 3. IDE 错误反馈
当 IDE 检测到 `modelLabel(id)` 返回的值与 `id` 不同时，应该提示管理员检查配置。

---

## 📝 总结

**问题根源**: 可能是以下几种情况之一
1. ✅ 服务器数据库 `model_names` 映射不完整
2. ⚠️ IDE 本地缓存了旧数据
3. ⚠️ 存在自定义模型配置冲突

**推荐操作步骤**:
1. 首先运行快速修复脚本更新 `model_names`
2. 清除 IDE 缓存并硬刷新
3. 如果问题仍存在，检查自定义模型配置

现在您可以按照上述步骤进行修复！如果需要进一步帮助，请告诉我检查结果。
