/**
 * 把「这一轮动过哪些文件」挂到入账消息上，供长会话压缩时的摘要读取。
 *
 * 为什么不直接带 tool_calls：assistant 消息带 tool_calls 却没有配套的 tool 角色
 * 回复，是**非法请求体**，回放历史时上游直接报错。所以 agent 收尾入账一律 text-only
 * （main.js 那段有原话 "Text-only (no tool_calls / tool-role messages)"），这条限制
 * 必须保持。
 *
 * 但代价一直没人补：ConversationMemory._summarizeBatch 的 Files:/Actions: 两行读的
 * 正是 msg.tool_calls，而全仓 14 个 memory.push 没有一个带它。于是压缩摘要里这两行
 * **恒为空**——不是「漏了 multi_edit」这种名单问题，是一个文件都没有。长会话压缩之后
 * 模型对「我改过什么」彻底失忆，只能回去重读整个目录，或者照着收尾正文那句
 * 「已完成重构」瞎说自己改了哪几处。
 *
 * 为什么是 _ideMeta：_sanitizeProviderMessages 是**排除法不是白名单**（它自己的注释
 * 写着「未知字段会原样发给上游」），而 _ideMeta 已经在那份解构里。换任何一个新字段名
 * 都会让用户的文件路径跟着每一次请求发到第三方端点去。
 *
 * 路径要**全路径**：同名文件（Next.js 一个项目里六个 page.tsx）只留 basename 等于没说。
 */
const MAX_FILES = 60;

export function attachExecutionFacts(message, mutatedFiles) {
  try {
    const touched = [...(mutatedFiles || [])].filter(Boolean).map(String);
    if (!touched.length) return message;
    message._ideMeta = { ...message._ideMeta, files: touched.slice(0, MAX_FILES), filesTotal: touched.length };
  } catch {}
  return message;
}
