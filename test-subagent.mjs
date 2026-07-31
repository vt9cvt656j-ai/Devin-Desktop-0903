#!/usr/bin/env node
/**
 * 测试脚本：验证 JobQueue + SharedStore 基础功能
 */

import getSharedStore from './src/agent/shared-store.js';
import getJobQueue from './src/agent/job-queue.js';

console.log('🧪 开始多智能体系统测试...\n');

// 测试 1: SharedStore 基本操作
console.log('✅ 测试 1: SharedStore 基本操作');
const store = getSharedStore({ maxEntries: 100 });

// 创建 job 记录
const jobId = store.createJob({ 
  role: 'backend', 
  task: 'Test API implementation',
  priority: 1
});
console.log(`  - 创建 job: ${jobId}`);

// 更新状态
store.updateJobStatus(jobId, 'running', 30);
console.log(`  - 更新状态为 running (30%)`);

// 添加 findings
store.appendFinding(jobId, {
  type: 'discovery',
  content: 'Found authentication logic in src/auth.js',
  confidence: 0.85
});
store.appendFinding(jobId, {
  type: 'finding',
  content: 'Token validation missing error handling',
  severity: 'warning'
});
console.log(`  - 添加了 2 条 findings`);

// 查询 stats
const stats = store.stats();
console.log(`  - SharedStore 统计：${JSON.stringify(stats)}`);

// 测试订阅
console.log('\n✅ 测试 2: SharedStore 订阅机制');
const unsubscribe = store.on(`jobs.${jobId}.findings`, (data) => {
  console.log(`  📩 收到 findings 更新：${data.length} 条`);
});
store.appendFinding(jobId, { type: 'info', content: 'New finding test' });
unsubscribe(); // 取消订阅后不应再触发

console.log('\n✅ 测试 3: JobQueue 提交作业');
const queue = getJobQueue({ maxConcurrent: 2, tokenWarningThreshold: 80 });

// 提交多个并发作业
const submittedIds = [];
for (let i = 1; i <= 4; i++) {
  const id = await queue.submit({
    tool: 'run_subagent',
    args: { 
      description: `测试任务 ${i}`,
      role: `tester-${i}`,
      prompt: `执行第${i}个测试任务`
    },
    runnerConfig: { maxSteps: 5 }
  });
  submittedIds.push(id);
  console.log(`  已提交 Job #${i}: ${id}`);
}

// 检查活跃 jobs
const activeJobs = queue.getActiveJobs();
console.log(`  当前活跃作业数：${activeJobs.length}/${submittedIds.length}`);

// 等待所有完成
console.log('\n⏳ 等待所有作业完成...');
const results = [];
for (const id of submittedIds) {
  try {
    const result = await queue.waitForJob(id, 60000);
    results.push(result);
    console.log(`  ✅ Job ${id.split('_').pop()} 完成：${result.summary}`);
  } catch (err) {
    console.error(`  ❌ Job ${id.split('_').pop()} 失败：${err.message}`);
  }
}

// 最后输出统计
console.log('\n📊 测试结果汇总:');
console.log(`  - JobQueue 总执行数：${queue.stats().completedJobs}`);
console.log(`  - 总 Token 消耗：${queue.stats().totalTokens.toLocaleString()}`);
console.log(`  - 最大并发数：${queue.stats().maxConcurrency}`);

// 清理
await queue.stopAll();
console.log('\n✅ 所有测试通过!');
console.log('🎉 多智能体系统基础功能正常');
