/**
 * WaLiOffice DSH Agent Engine 入口
 *
 * 两种启动方式：
 * 1. dsh --profile walioffice  （推荐，通过 DSH CLI 启动）
 * 2. node --import tsx main.ts  （直接启动，用于调试）
 *
 * 方式 2 通过 cordis 直接加载 dsh-base + walioffice profile 的插件。
 */

import { Context } from '@deepseek-ai/cordis';

// 加载环境变量
import dotenv from 'dotenv';
dotenv.config({ path: '../../.env' });

const ctx = new Context();

// 加载 dsh-base 核心插件（手动加载，模拟 dsh --profile walioffice）
// 在正式使用中，dsh CLI 会自动处理这些
async function main() {
  // 核心基础设施
  ctx.plugin(require('@deepseek-ai/cordis-plugin-timer'));
  ctx.plugin(require('@deepseek-ai/cordis-plugin-hmr'));
  ctx.plugin(require('@deepseek-ai/dsh-llm'));
  ctx.plugin(require('@deepseek-ai/dsh-session'));
  ctx.plugin(require('@deepseek-ai/dsh-agent'));
  ctx.plugin(require('@deepseek-ai/dsh-agent-loop'), {
    maxParallelToolCalls: 10,
    agents: [{
      id: 'office-agent',
      provider: 'openai-compatible',
      model: process.env.LLM_TEXT_MODEL_DEFAULT || 'deepseek-chat',
    }],
  });
  ctx.plugin(require('@deepseek-ai/dsh-tools'), { mode: 'native' });
  ctx.plugin(require('@deepseek-ai/dsh-system-prompt'), {
    persona: '你是 WaLiOffice 智能办公助手，专门帮助用户生成和编辑办公文档。',
  });

  // LLM 适配器
  const llmCompat = await import('./plugins/dsh-llm-openai-compatible/index.js');
  ctx.plugin(llmCompat.default, {
    baseURL: process.env.LLM_TEXT_BASE_URL || 'https://api.deepseek.com/v1',
    apiKeyEnv: 'LLM_TEXT_API_KEY',
    models: process.env.LLM_TEXT_MODELS || 'deepseek-chat',
    defaultModel: process.env.LLM_TEXT_MODEL_DEFAULT || 'deepseek-chat',
  });

  // HTTP Host
  const hostHttp = await import('./plugins/dsh-host-http/index.js');
  ctx.plugin(hostHttp.default, {
    port: parseInt(process.env.DSH_PORT || '3780', 10),
  });

  // Office 工具
  const tools = [
    'dsh-tool-ppt-plan',
    'dsh-tool-ppt-generate',
    'dsh-tool-doc-generate',
    'dsh-tool-md-generate',
    'dsh-tool-sheet-generate',
    'dsh-tool-chart-generate',
    'dsh-tool-drawio-generate',
    'dsh-tool-image-prompt',
    'dsh-tool-video-generate',
    'dsh-tool-web-search',
  ];

  for (const toolName of tools) {
    const tool = await import(`./plugins/${toolName}/index.js`);
    ctx.tools.register(tool.default);
  }

  // 启动
  await ctx.start();
  console.log('[WaLiOffice DSH Agent Engine] Started');
}

main().catch((err) => {
  console.error('[WaLiOffice DSH Agent Engine] Failed to start:', err);
  process.exit(1);
});
