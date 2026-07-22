# 让提示缓存对「全部模型」生效 —— 上游连接配置

## 为什么现在没生效

网关代码本身没问题:`chat_completions`（[models.rs](../src/models.rs)）把请求体**原样转发**给上游（`cache_control` 跟着走），流式时把上游的 `usage`（含缓存命中）**原样回传**给 IDE。IDE 端前缀也稳定（system 纯静态、动态内容下沉到 user 末尾）。

**真正的卡点是上游连接。** 后台「模型系统」里 6 条连接：

| 模型 | 当前 base_url | 能缓存吗 |
|---|---|---|
| Claude / GPT / Gemini / GLM / Minimax | `https://zyz.qingyanzhiying.top`（第三方转卖聚合商） | ❌ 基本不透传 cache_control、不回传缓存用量 |
| 官方 Deepseek | `https://api.deepseek.com`（直连） | ✅ 自动上下文缓存 |

通用转卖聚合商（zyz 这类）多半把请求归一化后丢掉 `cache_control`、也不回 `cache_read` 字段——所以上游永不命中、IDE 也看不到。它还是那批 **502** 的来源。

> 结论:把每个模型指到**真正支持提示缓存**的上游，缓存才会真命中。改完上游后，配合本次网关改造（流式按 token + 缓存折扣计费），省下的钱会体现在计费里。

## 逐模型怎么配（在后台「模型系统」改 base_url + 该供应商自己的 API Key）

### ✅ 自动缓存，直连官方即可（零额外工作）
这些供应商在官方 OpenAI 兼容端点上**自动**缓存稳定前缀，并在 `usage` 里回传缓存命中数：

| 模型 | base_url | 说明 |
|---|---|---|
| **DeepSeek** | `https://api.deepseek.com`（已是） | 自动上下文缓存，`prompt_cache_hit_tokens` 回传。无需改动。 |
| **OpenAI / GPT** | `https://api.openai.com/v1` | prompt ≥1024 token 自动缓存，回传 `prompt_tokens_details.cached_tokens`。 |
| **Gemini** | `https://generativelanguage.googleapis.com/v1beta/openai` | 2.5 系隐式缓存，回传 `cached_content_token_count`（已在 IDE 端补读）。 |
| **Qwen / DashScope** | `https://dashscope.aliyuncs.com/compatible-mode/v1` | 上下文缓存，OpenAI 兼容模式回传 `cached_tokens`。 |
| **Zhipu / GLM** | `https://open.bigmodel.cn/api/paas/v4` | 自带缓存，OpenAI 兼容。 |

→ 操作:后台编辑该连接，base_url 改成上表地址、api_key 填**该供应商官方 key**、保存。

### ⚠️ Claude（以及只有 Anthropic/Bedrock 渠道的模型）——需要缓存感知的兼容代理
Anthropic 原生 API 是 `/v1/messages`，**不是** `/chat/completions`，所以**不能**把 base_url 直接指向 `api.anthropic.com`（端点形状不对）。要在本网关（OpenAI 兼容）下用上 Claude 的提示缓存，上游必须是一个「OpenAI 兼容进、Anthropic/Bedrock 出、且透传 `cache_control`」的代理。三选一:

1. **自托管 LiteLLM（推荐，最可控）**
   - LiteLLM Proxy 暴露 `/v1/chat/completions`，后端接 Anthropic 或 AWS Bedrock，**透传 `cache_control`** 并回传 `cache_read_input_tokens` / `cache_creation_input_tokens`。
   - 部署一个 LiteLLM 容器，配置 Anthropic（或 Bedrock）凭据，把本网关该连接的 base_url 指到 `http://<litellm 内网地址>/v1`。
2. **AWS Bedrock 经 LiteLLM/Bedrock-Access-Gateway**：同上，凭据用 AWS（Bedrock 的 Anthropic 模型支持 prompt caching）。
3. **缓存感知的聚合商**（如 OpenRouter 等明确支持 Anthropic prompt caching 透传的）：把 base_url 指过去、用其 key。**先小流量验证**它确实回传了 `cache_read_input_tokens`，再切量。

> IDE 端我加的 `cache_control` 断点（system + 滚动尾部）只对 Anthropic 家有意义；上游换成上述任一缓存感知代理后，它才会真正命中。

## 验证（换上游后）
1. 用该模型跑一个 ≥5 步的多轮任务。
2. 看 IDE 底部**缓存命中率计量条**:第 2 轮起命中率应明显 >0。
3. 看后台 `model_usage` / 用户额度:多轮里单次扣费随缓存下降（本次网关改造后按 token+缓存折扣计费）。
4. 若命中率仍为 0:说明该上游没透传缓存——换上表里的官方直连或自托管 LiteLLM。

## 备注
- 后台连接里的 `provider` 字段当前都填的 `deepseek`，那只是个标签;实际是 OpenAI 兼容透传，不影响缓存。真正决定缓存的是 **base_url 背后的真实上游**。
- `zyz` 这类聚合商便宜但不稳（502 频发）也不缓存;对在意成本/稳定性的主力模型，建议走官方直连或自托管代理。
