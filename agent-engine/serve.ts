/**
 * DSH Agent Engine 常驻服务入口（ESM）
 *
 * 被 Rust 后端作为子进程拉起，监听 port 3780 提供 HTTP/SSE 接口。
 * Rust 停止时 DSH 自动退出（kill_on_drop）。
 */

import 'dotenv/config';
import { Context } from '@deepseek-ai/cordis';

async function main() {
  const ctx = new Context();

  // ── dsh-base 核心插件 ──
  const timer = await import('@deepseek-ai/cordis-plugin-timer');
  ctx.plugin(timer.default ?? timer);

  const hmr = await import('@deepseek-ai/cordis-plugin-hmr');
  ctx.plugin(hmr.default ?? hmr, { root: ['.'] });

  const llm = await import('@deepseek-ai/dsh-llm');
  ctx.plugin(llm.default ?? llm);

  const session = await import('@deepseek-ai/dsh-session');
  ctx.plugin(session.default ?? session);

  const agent = await import('@deepseek-ai/dsh-agent');
  ctx.plugin(agent.default ?? agent);

  const agentDefaultModel = await import('@deepseek-ai/dsh-agent-default-model');
  ctx.plugin(agentDefaultModel.default ?? agentDefaultModel, {
    provider: 'openai-compatible',
    model: process.env.LLM_TEXT_MODEL_DEFAULT || 'deepseek-chat',
  });

  const agentLoop = await import('@deepseek-ai/dsh-agent-loop');
  ctx.plugin(agentLoop.default ?? agentLoop, {
    maxParallelToolCalls: 10,
    agents: [{
      id: 'office-agent',
      sessionId: 'default',
      provider: 'openai-compatible',
      model: process.env.LLM_TEXT_MODEL_DEFAULT || 'deepseek-chat',
    }],
  });

  const tools = await import('@deepseek-ai/dsh-tools');
  ctx.plugin(tools.default ?? tools, { mode: 'native' });

  const systemPrompt = await import('@deepseek-ai/dsh-system-prompt');
  ctx.plugin(systemPrompt.default ?? systemPrompt, {
    persona: `你是 WaLiOffice 智能办公助手，专门帮助用户生成和编辑办公文档。
你可以生成 PPT、Word 文档、Excel 表格、Markdown、图表、Draw.io 图、图片和视频。
当用户提出需求时，先理解意图，再选择合适的工具执行。
多文件交付时，按顺序生成每个文件。
搜索结果需要标注来源。`,
  });

  const sessionPersist = await import('@deepseek-ai/dsh-session-persistence-jsonl');
  ctx.plugin(sessionPersist.default ?? sessionPersist, {
    root: process.env.WALIOFFICE_SESSION_DIR || '/tmp/walioffice-dsh-sessions',
  });

  // ── WaLiOffice 自定义插件 ──
  const llmCompat = await import('./plugins/dsh-llm-openai-compatible/index.js');
  ctx.plugin(llmCompat.default ?? llmCompat, {
    baseURL: process.env.LLM_TEXT_BASE_URL || 'https://api.deepseek.com/v1',
    apiKeyEnv: 'LLM_TEXT_API_KEY',
    models: process.env.LLM_TEXT_MODELS || 'deepseek-chat',
    defaultModel: process.env.LLM_TEXT_MODEL_DEFAULT || 'deepseek-chat',
  });

  const hostHttp = await import('./plugins/dsh-host-http/index.js');
  ctx.plugin(hostHttp.default ?? hostHttp, {
    port: parseInt(process.env.DSH_PORT || '3780', 10),
  });

  const waliofficeTools = await import('./plugins/walioffice-tools/index.js');
  ctx.plugin(waliofficeTools.default ?? waliofficeTools);

  console.log('[DSH Agent Engine] All plugins registered, waiting for HTTP port ready...');

  // 保持进程运行
  process.on('SIGINT', () => { console.log('[DSH] SIGINT'); process.exit(0); });
  process.on('SIGTERM', () => { console.log('[DSH] SIGTERM'); process.exit(0); });
}

main().catch((err) => {
  console.error('[DSH Agent Engine] Failed to start:', err);
  process.exit(1);
});
