/**
 * dsh-llm-openai-compatible: OpenAI 兼容 LLM 适配器
 *
 * 注册为 DSH 的 LLM 适配器，使用 OpenAI chat completions API。
 * 支持多 API Key 轮询和 SSE 流式输出。
 *
 * cordis 函数式插件格式：导出 name, inject, apply
 */

import { LlmAdapter, LlmError, attributionHeaders } from '@deepseek-ai/dsh-llm';

export const name = 'llm-openai-compatible';
export const inject = ['llm'];

const PROVIDER_NAME = 'openai-compatible';

class OpenAICompatibleAdapter extends LlmAdapter {
  private baseURL: string;
  private apiKeys: string[];
  private keyIndex: number = 0;
  private models: string[];
  private defaultModel: string;

  constructor(config: any) {
    super();
    this.baseURL = config?.baseURL || process.env.LLM_TEXT_BASE_URL || 'https://api.deepseek.com/v1';
    this.models = (config?.models || process.env.LLM_TEXT_MODELS || 'deepseek-chat').split(',').map((s: string) => s.trim());
    this.defaultModel = config?.defaultModel || process.env.LLM_TEXT_MODEL_DEFAULT || this.models[0];

    const keyEnv = config?.apiKeyEnv || 'LLM_TEXT_API_KEY';
    const rawKey = process.env[keyEnv] || '';
    this.apiKeys = rawKey.split(',').map((s: string) => s.trim()).filter(Boolean);

    if (this.apiKeys.length === 0) {
      console.warn('[dsh-llm-openai-compatible] Warning: No API keys found in', keyEnv);
    }
  }

  private getNextKey(): string {
    if (this.apiKeys.length === 0) return '';
    const key = this.apiKeys[this.keyIndex % this.apiKeys.length];
    this.keyIndex++;
    return key;
  }

  override providerInfo(provider: string) {
    return { id: provider, name: 'OpenAI Compatible', description: `OpenAI-compatible API at ${this.baseURL}` };
  }

  override async listModels(_provider: string) {
    return this.models.map((id) => ({ id, name: id, provider: PROVIDER_NAME }));
  }

  override async resolveModel(provider: string, model: string, _signal?: AbortSignal) {
    return { provider, id: model, name: model, contextWindow: 128000 };
  }

  override async *stream(options: any): AsyncGenerator<any> {
    const apiKey = this.getNextKey();
    const model = options.model || this.defaultModel;
    const messages = this.serializeMessages(options.messages || []);

    const body: Record<string, any> = { model, messages, stream: true };

    if (options.tools?.length > 0) {
      body.tools = options.tools.map((t: any) => ({
        type: 'function',
        function: { name: t.name, description: t.description, parameters: t.parameters },
      }));
    }

    if (options.maxTokens) body.max_tokens = options.maxTokens;
    if (options.temperature !== undefined) body.temperature = options.temperature;

    const url = `${this.baseURL}/chat/completions`;
    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiKey}`,
      ...attributionHeaders(),
    };

    const response = await fetch(url, {
      method: 'POST',
      headers,
      body: JSON.stringify(body),
      signal: options.signal,
    });

    if (!response.ok) {
      const text = await response.text().catch(() => '');
      throw new LlmError(`OpenAI API error ${response.status}: ${text}`, this.classifyErrorCode(response.status));
    }

    if (!response.body) {
      throw new LlmError('No response body', 'EMPTY_RESPONSE');
    }

    const { EventSourceParserStream } = await import('eventsource-parser/stream');
    const sseStream = response.body.pipeThrough(new TextDecoderStream()).pipeThrough(new EventSourceParserStream());

    // DSH BlockAssembler 期望的 chunk 格式：
    // block-start → text-delta/reasoning-delta/tool-call-delta → block-end → usage → finish
    let nextIndex = 0;
    let textBlock: any = null;
    let reasoningBlock: any = null;
    const toolBlocks = new Map<number, any>();
    const order: any[] = [];
    let pendingFinish: any = null;
    let pendingUsage: any = null;

    function openBlock(kind: string) {
      const block = { index: nextIndex++, kind, text: '' };
      order.push(block);
      return block;
    }

    function closeBlock(block: any): any {
      if (block.kind === 'text') return { type: 'text', text: block.text };
      if (block.kind === 'reasoning') return { type: 'reasoning', text: block.text };
      if (block.kind === 'tool-call') return { type: 'tool-call', id: block.callId || '', name: block.name || '', arguments: block.text };
      return { type: 'text', text: block.text };
    }

    for await (const event of sseStream) {
      if (event.data === '[DONE]') {
        // 发送所有 block-end
        for (const block of order) {
          yield { type: 'block-end', index: block.index, block: closeBlock(block) };
        }
        if (pendingUsage) {
          yield { type: 'usage', usage: pendingUsage };
        }
        const reason = pendingFinish || { kind: 'stop' };
        yield { type: 'finish', reason };
        return;
      }

      try {
        const data = JSON.parse(event.data);
        const choice = data.choices?.[0];
        if (!choice) continue;

        const delta = choice.delta;

        // reasoning_content (思考过程)
        const reasoning = delta?.reasoning_content;
        if (typeof reasoning === 'string' && reasoning.length > 0) {
          if (!reasoningBlock) {
            reasoningBlock = openBlock('reasoning');
            yield { type: 'block-start', index: reasoningBlock.index, blockType: 'reasoning' };
          }
          reasoningBlock.text += reasoning;
          yield { type: 'reasoning-delta', index: reasoningBlock.index, text: reasoning };
        }

        // content (正文)
        const content = delta?.content;
        if (typeof content === 'string' && content.length > 0) {
          if (!textBlock) {
            textBlock = openBlock('text');
            yield { type: 'block-start', index: textBlock.index, blockType: 'text' };
          }
          textBlock.text += content;
          yield { type: 'text-delta', index: textBlock.index, text: content };
        }

        // tool_calls
        for (const call of delta?.tool_calls ?? []) {
          const callIdx = call.index ?? 0;
          let block = toolBlocks.get(callIdx);
          if (!block) {
            block = openBlock('tool-call');
            toolBlocks.set(callIdx, block);
            yield { type: 'block-start', index: block.index, blockType: 'tool-call' };
          }
          if (call.id !== undefined) block.callId = call.id;
          if (call.function?.name !== undefined) block.name = call.function.name;
          const fragment = call.function?.arguments ?? '';
          block.text += fragment;
          yield {
            type: 'tool-call-delta',
            index: block.index,
            id: block.callId || '',
            ...(block.name !== undefined ? { name: block.name } : {}),
            argumentsDelta: fragment,
          };
        }

        if (typeof choice.finish_reason === 'string') {
          pendingFinish = this.mapFinishReason(choice.finish_reason);
        }
      } catch {
        // 忽略解析错误
      }
    }

    // 流结束但没有 [DONE]
    throw new LlmError('SSE payload stream ended without [DONE]', 'STREAM_CLOSED');
  }

  private mapFinishReason(reason: string): any {
    switch (reason) {
      case 'stop': return { kind: 'stop' };
      case 'length': return { kind: 'length' };
      case 'tool_calls': return { kind: 'tool_calls' };
      case 'content_filter': return { kind: 'content_filter' };
      default: return { kind: 'stop' };
    }
  }

  private serializeMessages(messages: any[]): any[] {
    const result: any[] = [];
    for (const msg of messages) {
      if (msg.role === 'system') {
        result.push({ role: 'system', content: this.extractText(msg.content) });
      } else if (msg.role === 'assistant') {
        const text = this.extractText(msg.content);
        const toolCalls = msg.content?.filter?.((c: any) => c.type === 'tool-call') || [];
        result.push({
          role: 'assistant',
          content: text || null,
          ...(toolCalls.length > 0 ? { tool_calls: toolCalls.map((tc: any) => ({ id: tc.id, type: 'function', function: { name: tc.name, arguments: tc.arguments } })) } : {}),
        });
      } else if (msg.role === 'user') {
        // user 消息可能包含 tool-result 内容块
        const toolResults = msg.content?.filter?.((c: any) => c.type === 'tool-result') || [];
        if (toolResults.length > 0) {
          for (const tr of toolResults) {
            const output = typeof tr.content === 'string' ? tr.content :
              Array.isArray(tr.content) ? tr.content.map((c: any) => c?.text || '').join('') :
              JSON.stringify(tr.content);
            result.push({ role: 'tool', tool_call_id: tr.toolCallId || tr.id, content: output });
          }
        } else {
          const text = this.extractText(msg.content);
          if (text) result.push({ role: 'user', content: text });
        }
      }
    }
    return result;
  }

  private extractText(content: any): string {
    if (typeof content === 'string') return content;
    if (Array.isArray(content)) return content.filter((c: any) => c.type === 'text').map((c: any) => c.text).join('');
    return '';
  }

  private classifyErrorCode(status: number): string {
    if (status === 401 || status === 403) return 'INVALID_CREDENTIAL';
    if (status === 429) return 'QUOTA_EXCEEDED';
    if (status === 400) return 'INVALID_REQUEST';
    return 'PROVIDER_ERROR';
  }
}

export function apply(ctx: any, config: any) {
  const adapter = new OpenAICompatibleAdapter(config);
  ctx.llm.registerAdapter([PROVIDER_NAME], adapter);
  ctx.logger?.info?.('[dsh-llm-openai-compatible] Registered adapter for provider:', PROVIDER_NAME);
}
