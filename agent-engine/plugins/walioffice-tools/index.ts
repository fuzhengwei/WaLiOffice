/**
 * WaLiOffice Office Tools 宿主插件
 *
 * 将所有 10 个 Office 工具注册到 DSH 的工具注册表。
 * 工具参数与 Rust 后端完全对齐。
 */

import { defineTool } from '@deepseek-ai/dsh-tools';

const RUST_BACKEND_URL = process.env.RUST_BACKEND_URL || 'http://127.0.0.1:8000';

// ── 共享工具函数 ──

async function callRustBackend(
  tool: string,
  input: Record<string, any>,
): Promise<{ success: boolean; observation: string; artifacts?: any[]; error?: string }> {
  try {
    const resp = await fetch(`${RUST_BACKEND_URL}/api/agent/tool/${tool}`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ tool, input }),
    });

    if (!resp.ok) {
      const text = await resp.text().catch(() => '');
      return { success: false, observation: `Rust backend error: ${resp.status} ${text}` };
    }

    return resp.json();
  } catch (err: any) {
    return { success: false, observation: `Rust backend unreachable: ${err.message}` };
  }
}

function truncateResult(text: string, maxLen = 4000): string {
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen) + `\n...[truncated, total ${text.length} chars]`;
}

// ── output schema 和 render 共用 ──

const outputSchema = {
  type: 'object',
  additionalProperties: true,
  properties: {
    observation: { type: 'string' },
    success: { type: 'boolean' },
  },
};

function renderResult(_args: any, value: any) {
  const text = typeof value === 'string' ? value : value?.observation || JSON.stringify(value);
  const parts: Array<{ type: string; text: string }> = [
    { type: 'text', text: truncateResult(text) },
  ];
  // 把 artifacts 编码到第二个 text block，供 dsh-host-http 提取发送 artifact_update
  if (value?.artifacts && Array.isArray(value.artifacts) && value.artifacts.length > 0) {
    parts.push({ type: 'text', text: `__ARTIFACTS__${JSON.stringify(value.artifacts)}__END_ARTIFACTS__` });
  }
  return parts;
}

// ── 工具定义（参数与 Rust 后端完全对齐） ──

const tools = [
  defineTool({
    name: 'ppt_plan',
    description: '根据用户需求生成 PPT 大纲和结构规划。',
    parameters: {
      topic: { type: 'string', required: true, description: 'PPT 主题/用户需求' },
    },
    output: { schema: outputSchema, render: renderResult },
    async execute(args: any, _exec: any) {
      const result = await callRustBackend('ppt_plan', args);
      if (!result.success) throw new Error(result.error || result.observation);
      return { observation: result.observation, artifacts: result.artifacts };
    },
  }),

  defineTool({
    name: 'ppt_generate',
    description: '根据大纲和主题生成 PPT 文件。',
    parameters: {
      title: { type: 'string', required: true, description: 'PPT 标题' },
      topic: { type: 'string', description: '用户原始需求（如果没有大纲则用于生成）' },
      theme: { type: 'string', description: '主题：default/business/tech/warm/minimal' },
    },
    output: { schema: outputSchema, render: renderResult },
    async execute(args: any, _exec: any) {
      const result = await callRustBackend('ppt_generate', args);
      if (!result.success) throw new Error(result.error || result.observation);
      return { observation: result.observation, artifacts: result.artifacts };
    },
  }),

  defineTool({
    name: 'doc_generate',
    description: '生成 Word 文档（.docx）。',
    parameters: {
      topic: { type: 'string', required: true, description: '文档主题/用户需求' },
      audience: { type: 'string', description: '目标读者（可选）' },
      format: { type: 'string', description: '文档类型：report/plan/summary/article/prd' },
    },
    output: { schema: outputSchema, render: renderResult },
    async execute(args: any, _exec: any) {
      const result = await callRustBackend('doc_generate', args);
      if (!result.success) throw new Error(result.error || result.observation);
      return { observation: result.observation, artifacts: result.artifacts };
    },
  }),

  defineTool({
    name: 'md_generate',
    description: '生成 Markdown 文档。',
    parameters: {
      topic: { type: 'string', required: true, description: 'Markdown 文档主题/用户需求' },
      audience: { type: 'string', description: '目标读者（可选）' },
    },
    output: { schema: outputSchema, render: renderResult },
    async execute(args: any, _exec: any) {
      const result = await callRustBackend('md_generate', args);
      if (!result.success) throw new Error(result.error || result.observation);
      return { observation: result.observation, artifacts: result.artifacts };
    },
  }),

  defineTool({
    name: 'sheet_generate',
    description: '生成 Excel 表格（.xlsx）。',
    parameters: {
      topic: { type: 'string', required: true, description: '表格主题/用户需求' },
    },
    output: { schema: outputSchema, render: renderResult },
    async execute(args: any, _exec: any) {
      const result = await callRustBackend('sheet_generate', args);
      if (!result.success) throw new Error(result.error || result.observation);
      return { observation: result.observation, artifacts: result.artifacts };
    },
  }),

  defineTool({
    name: 'chart_generate',
    description: '生成图表（bar/line/pie/gauge/funnel/scatter）。',
    parameters: {
      topic: { type: 'string', required: true, description: '图表主题/用户想可视化的问题' },
      chart_type: { type: 'string', description: '图表类型：line/bar/pie/gauge/funnel/scatter，默认 bar' },
    },
    output: { schema: outputSchema, render: renderResult },
    async execute(args: any, _exec: any) {
      const result = await callRustBackend('chart_generate', args);
      if (!result.success) throw new Error(result.error || result.observation);
      return { observation: result.observation, artifacts: result.artifacts };
    },
  }),

  defineTool({
    name: 'drawio_generate',
    description: '生成 Draw.io 图表。',
    parameters: {
      topic: { type: 'string', required: true, description: '图表主题/用户需求' },
      diagram_type: { type: 'string', description: '图表类型：flowchart/architecture/swimlane/topology/er/mindmap' },
    },
    output: { schema: outputSchema, render: renderResult },
    async execute(args: any, _exec: any) {
      const result = await callRustBackend('drawio_generate', args);
      if (!result.success) throw new Error(result.error || result.observation);
      return { observation: result.observation, artifacts: result.artifacts };
    },
  }),

  defineTool({
    name: 'image_prompt',
    description: '生成图片提示词或直接生成图片。',
    parameters: {
      topic: { type: 'string', required: true, description: '图片需求描述' },
      image_url: { type: 'string', description: '图生图参考图片 URL（可选）' },
    },
    output: { schema: outputSchema, render: renderResult },
    async execute(args: any, _exec: any) {
      const result = await callRustBackend('image_prompt', args);
      if (!result.success) throw new Error(result.error || result.observation);
      return { observation: result.observation, artifacts: result.artifacts };
    },
  }),

  defineTool({
    name: 'video_generate',
    description: '生成视频。',
    parameters: {
      topic: { type: 'string', required: true, description: '视频需求描述' },
      image_url: { type: 'string', description: '图生视频参考图片 URL（可选）' },
    },
    output: { schema: outputSchema, render: renderResult },
    async execute(args: any, _exec: any) {
      const result = await callRustBackend('video_generate', args);
      if (!result.success) throw new Error(result.error || result.observation);
      return { observation: result.observation, artifacts: result.artifacts };
    },
  }),

  defineTool({
    name: 'web_search',
    description: '搜索网页获取信息。',
    parameters: {
      query: { type: 'string', required: true, description: '搜索关键词，尽量具体' },
    },
    output: { schema: outputSchema, render: renderResult },
    async execute(args: any, _exec: any) {
      const result = await callRustBackend('web_search', args);
      if (!result.success) throw new Error(result.error || result.observation);
      return { observation: result.observation, artifacts: result.artifacts };
    },
  }),
];

// ── cordis 插件入口 ──

export const name = 'walioffice-tools';
export const inject = ['tools'];

export function apply(ctx: any, _config: any) {
  for (const tool of tools) {
    try {
      ctx.tools.register(tool);
      ctx.logger?.info?.(`[walioffice-tools] Registered tool: ${tool.name}`);
    } catch (err: any) {
      ctx.logger?.warn?.(`[walioffice-tools] Failed to register tool ${tool.name}:`, err?.message);
    }
  }
  ctx.logger?.info?.(`[walioffice-tools] ${tools.length} office tools registered`);
}
