/**
 * spawn_multiple_agents - 并发派发给多个角色的子智能体
 * 
 * 实现思路：
 * 1. 解析参数中的 agents 数组
 * 2. 每个 agent 派发独立的 JobQueue job
 * 3. 可选启用 SharedStore 协同模式
 *
 * This compatibility module is only usable when the host injects the real
 * Agent runner into `context.jobQueue`. It deliberately does not fall back to
 * the old simulated runner.
 */

export function createSpawnMultipleAgentsTool() {
  return {
    name: "spawn_multiple_agents",
    description: `并发派发给多个角色专属的子智能体（backend/frontend/test/research 等），每个独立执行并定期汇报进度。

【何时用】
- 大型任务需要并行开发：例如"构建完整 todo app"(backend API + frontend UI + E2E tests)
- 多视角调研：例如"分析项目架构"(security review + performance audit + code quality scan)
- 分工协作：每个角色专注自己的领域，最后汇总结果

【用法示例】
spawn_multiple_agents(
  task="构建一个带 REST API 的 todo app",
  agents=[
    {role='backend', focus='设计用户认证和 todos CRUD 接口', tools=['run_worker']},
    {role='frontend', focus='实现 React 组件和状态管理', tools=['run_worker']},
    {role='test', focus='编写 Jest 测试覆盖核心功能', tools=['run_subagent']}
  ],
  collaborationMode='shared_store'  // 或'independent'/eventbus
)

【注意事项】
- 默认限制最大并发数：5 个
- token 消耗会指数增长，注意预算控制
- 建议给每个 agent 明确的 scope/边界，避免工作重叠`,
    inputSchema: {
      type: "object",
      properties: {
        task: {
          type: "string",
          description: "整体任务描述，会被拆解分配给每个 agent"
        },
        agents: {
          type: "array",
          items: {
            type: "object",
            properties: {
              role: {
                type: "string",
                description: "角色名称，如 backend/frontend/test/security/research"
              },
              focus: {
                type: "string",
                description: "该 agent 的专注点/子任务描述"
              },
              tools: {
                type: "array",
                items: { type: "string" },
                description: "可用的工具列表"
              },
              priority: {
                type: "integer",
                default: 1,
                description: "优先级数字，越高越优先"
              }
            },
            required: ["role", "focus"]
          },
          minItems: 1,
          maxItems: 5
        },
        collaborationMode: {
          type: "string",
          enum: ["independent", "shared_store", "eventbus"],
          default: "shared_store",
          description: "协作模式：independent=完全独立/shared_store=共享 Store/eventbus=事件驱动"
        },
        maxStepsPerAgent: {
          type: "integer",
          default: 20,
          description: "每个子智能体的最大步数"
        },
        timeoutSeconds: {
          type: "integer",
          default: 300,
          description: "总超时时间 (秒)"
        }
      },
      required: ["task", "agents"]
    }
  };
}

// ===== 实际执行函数 (在 Rust 端实现) =====
// 由于这是前端 JS 代码，真实执行需要在 Tauri/Rust 层调用
// 这里提供调用框架逻辑

export async function executeSpawnMultipleAgents(args, context) {
  const { task, agents, collaborationMode = 'shared_store', maxStepsPerAgent = 20 } = args;
  
  const _globalJobQueue = context.jobQueue;
  const _globalSharedStore = context.sharedStore;

  if (!_globalJobQueue || typeof _globalJobQueue.submit !== 'function' || typeof _globalJobQueue.runner !== 'function') {
    throw new Error("spawn_multiple_agents requires a JobQueue with an injected Agent runner");
  }
  if (!_globalSharedStore || typeof _globalSharedStore.on !== 'function') {
    throw new Error("spawn_multiple_agents requires a SharedStore collaboration context");
  }
  
  if (!agents || !Array.isArray(agents) || agents.length === 0) {
    throw new Error("必须提供至少一个 agent");
  }
  
  if (agents.length > 5) {
    throw new Error("最多同时派发 5 个子智能体，请分批执行");
  }
  
  const jobIds = [];
  
  // 为每个 agent 提交独立 job
  for (const agent of agents) {
    const jobId = await _globalJobQueue.submit({
      id: `multi_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      tool: agent.tools?.includes('run_worker') ? 'run_worker' : 'run_subagent',
      args: {
        description: `${task} (${agent.role})`,
        prompt: `${task}\n\n[${agent.role.toUpperCase()}] Focus on: ${agent.focus}`,
        role: agent.role,
        scope: [], // 如有需要可指定
        write: agent.tools?.includes('run_worker')
      },
      runnerConfig: {
        maxSteps: maxStepsPerAgent,
        ...agent.runnerConfig
      },
      onProgress: ({ step, progress }) => {
        console.log(`[${agent.role}] Job progress: ${progress}%`);
      },
      onComplete: async (result) => {
        console.log(`[${agent.role}] Job completed`);
        // 完成后自动注入 messages (由 JobQueue 处理)
      }
    });
    
    jobIds.push(jobId);
  }
  
  // 如果启用 shared_store 模式，设置协同规则
  if (collaborationMode === 'shared_store') {
    setupSharedStoreCollaboration(jobIds, _globalSharedStore);
  }
  
  return {
    success: true,
    message: `已并发派发 ${agents.length} 个子智能体`,
    jobIds,
    estimatedCompletion: `~${agents.length * 2}分钟`,
    statusEndpoint: `/jobs/status?ids=${jobIds.join(',')}`
  };
}

/**
 * 设置 SharedStore 协同规则
 * 当某个 job 有新 findings 时，广播到其他相关 jobs
 */
function setupSharedStoreCollaboration(jobIds, sharedStore) {
  jobIds.forEach(sourceJobId => {
    sharedStore.on(`jobs.${sourceJobId}.findings`, (findings) => {
      // 只广播最新 3 条，减少噪音
      const latestFindings = Array.isArray(findings) ? findings.slice(-3) : [];
      
      if (!latestFindings.length) return;
      
      // 广播给其他 job
      jobIds.forEach(targetJobId => {
        if (targetJobId !== sourceJobId) {
          sharedStore.appendFinding(targetJobId, {
            source: sourceJobId,
            channel: 'collaboration',
            data: latestFindings,
            isExternal: true,
            timestamp: Date.now()
          });
        }
      });
    });
  });
}
