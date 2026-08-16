import { defineTool } from '@deepseek-ai/dsh-tools';

const RUST_BACKEND_URL = process.env.RUST_BACKEND_URL || 'http://127.0.0.1:8000';

/**
 * 调用 Rust 后端执行工具渲染
 */
export async function callRustBackend(
  tool: string,
  input: Record<string, any>,
  sessionId?: string,
  userId?: string,
): Promise<{ success: boolean; observation: string; artifacts?: any[]; error?: string }> {
  const resp = await fetch(`${RUST_BACKEND_URL}/api/agent/tool/${tool}`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ tool, input, session_id: sessionId, user_id: userId }),
  });

  if (!resp.ok) {
    const text = await resp.text().catch(() => '');
    return { success: false, observation: `Rust backend error: ${resp.status} ${text}` };
  }

  return resp.json();
}

/**
 * 截断工具结果文本（DSH 对输出长度有限制）
 */
export function truncateResult(text: string, maxLen = 4000): string {
  if (text.length <= maxLen) return text;
  return text.slice(0, maxLen) + `\n...[truncated, total ${text.length} chars]`;
}

/**
 * 创建 Office 工具的通用 defineTool 工厂
 */
export function createOfficeTool(options: {
  name: string;
  description: string;
  parameters: Record<string, { type: string; required?: boolean; description?: string }>;
  outputSchema?: Record<string, any>;
}) {
  return defineTool({
    name: options.name,
    description: options.description,
    parameters: options.parameters,
    output: {
      schema: options.outputSchema || {
        type: 'object',
        additionalProperties: false,
        properties: {
          observation: { type: 'string' },
          artifacts: { type: 'string' },
        },
      },
      render(_args: any, value: any) {
        const text = typeof value === 'string' ? value : value?.observation || JSON.stringify(value);
        return [{ type: 'text' as const, text: truncateResult(text) }];
      },
    },
    async execute(args: any, _exec: any) {
      const result = await callRustBackend(options.name, args);
      if (!result.success) {
        throw new Error(result.error || result.observation);
      }
      return { observation: result.observation, artifacts: result.artifacts };
    },
  });
}
