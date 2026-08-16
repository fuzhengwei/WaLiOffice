/**
 * dsh-host-http: HTTP/SSE 接口插件
 *
 * 被 Rust 后端代理调用，提供 DSH Agent 的流式对话接口。
 * 使用 DSH AgentLoop 的 session event 系统获取流式输出。
 *
 * SSE 事件格式（与前端 Studio.tsx 对齐）：
 * - state_update: { phase, step, detail } — 状态/思考过程
 * - message: { text, start? } — AI 文字回复（流式分片）
 * - tool_result: { tool, success, result: { observation } }
 * - artifact_update: { artifact }
 * - done: { session_id }
 * - error: { message }
 */

import http from 'node:http';

export const name = 'host-http';
export const inject = ['agents', 'agentLoop'];

const DEFAULT_PORT = 3780;

export function apply(ctx: any, config: any) {
  const port = config?.port ?? DEFAULT_PORT;

  const server = http.createServer(async (req, res) => {
    const url = new URL(req.url || '/', `http://localhost:${port}`);

    res.setHeader('Access-Control-Allow-Origin', '*');
    res.setHeader('Access-Control-Allow-Methods', 'GET, POST, OPTIONS');
    res.setHeader('Access-Control-Allow-Headers', 'Content-Type, Authorization');

    if (req.method === 'OPTIONS') {
      res.writeHead(204);
      res.end();
      return;
    }

    try {
      if (url.pathname === '/agent/status' && req.method === 'GET') {
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ dsh_available: true, engine: 'dsh', version: '0.1.0-rc.6' }));
      } else if (url.pathname === '/agent/stream' && req.method === 'POST') {
        await handleStream(req, res, ctx);
      } else {
        res.writeHead(404, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: 'not found' }));
      }
    } catch (err: any) {
      console.error('[dsh-host-http] Error:', err);
      if (!res.headersSent) {
        res.writeHead(500, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify({ error: err.message }));
      }
    }
  });

  server.listen(port, () => {
    console.log(`[dsh-host-http] Listening on port ${port}`);
  });

  ctx.on('dispose', () => {
    server.close();
  });
}

async function handleStream(req: http.IncomingMessage, res: http.ServerResponse, ctx: any) {
  const body = await readBody(req);
  const { message, session_id, model } = JSON.parse(body);

  res.writeHead(200, {
    'Content-Type': 'text/event-stream',
    'Cache-Control': 'no-cache',
    'Connection': 'keep-alive',
  });

  try {
    const agents = ctx.agents;
    if (!agents) throw new Error('Agent service not available');

    const sessionId = session_id || `session-${Date.now()}`;

    const handle = await agents.create({
      sessionId,
      agentOptions: {
        provider: 'openai-compatible',
        model: model || process.env.LLM_TEXT_MODEL_DEFAULT || 'deepseek-chat',
      },
    });

    const agent = handle.agent;

    // 流式状态追踪
    const state = {
      callIdToName: new Map<string, string>(),
      announcedTools: new Set<string>(),
      reasoningBuf: '',       // 累积 reasoning 文本
      reasoningSentLen: 0,    // 已发送的 reasoning 长度
      textStarted: false,     // text block 是否已发 start
      fullAssistantText: '',  // 完整 assistant 文本（用于去重 assistant/message）
    };

    const send = (type: string, data: any) => {
      res.write(`event: ${type}\ndata: ${JSON.stringify(data)}\n\n`);
    };

    const eventListener = (_subject: any, event: any) => {
      try {
        // 预处理：记录 tool/call 的 callId→name
        if (event?.type === 'tool/call') {
          const cid = event.data?.callId || '';
          const cname = event.data?.name || '';
          if (cid && cname) state.callIdToName.set(cid, cname);
        }

        const events = mapEvent(event, state);
        for (const ev of events) {
          send(ev.type, ev.data);
        }
      } catch (e) {
        // ignore
      }
    };

    agent.ctx.on('session/event', eventListener);

    agent.ctx.on('agent/error', (_s: any, payload: any) => {
      send('error', { message: payload?.error?.message || 'agent error' });
    });

    // 发送用户消息，触发 agent 循环
    agent.followup({ role: 'user', content: [{ type: 'text', text: message }] });

    // 等待 agent 完成
    await agent.whenIdle();

    // 发送 done 事件
    send('done', { session_id: sessionId });
  } catch (err: any) {
    res.write(`event: error\ndata: ${JSON.stringify({ message: err.message })}\n\n`);
  }
  res.end();
}

// ─── 事件映射 ───

interface StreamState {
  callIdToName: Map<string, string>;
  announcedTools: Set<string>;
  reasoningBuf: string;
  reasoningSentLen: number;
  textStarted: boolean;
  fullAssistantText: string;
}

function mapEvent(event: any, s: StreamState): { type: string; data: any }[] {
  if (!event || !event.type) return [];
  const d = event.data || {};

  switch (event.type) {
    // ─── turn/step 边界 ───
    case 'turn/start':
      return [{ type: 'state_update', data: { phase: 'running', step: '开始处理', detail: '正在分析您的请求...' } }];

    case 'turn/end':
    case 'step/start':
    case 'step/end':
    case 'request/header':
      return [];

    // ─── LLM 流式 chunk ───
    case 'assistant/chunk': {
      const chunk = d.chunk;
      if (!chunk) return [];

      switch (chunk.type) {
        // LLM 文字输出
        case 'text-delta': {
          const text = chunk.text || chunk.delta || '';
          if (!text) return [];
          s.fullAssistantText += text;

          // 第一次收到 text-delta 时发 start 标记
          if (!s.textStarted) {
            s.textStarted = true;
            return [
              { type: 'message', data: { text: '', start: true } },
              { type: 'message', data: { text } },
            ];
          }
          return [{ type: 'message', data: { text } }];
        }

        // LLM 推理输出（reasoning model 的思考过程）
        case 'reasoning-delta': {
          const text = chunk.text || chunk.delta || '';
          if (!text) return [];
          s.reasoningBuf += text;

          // 每累积 ~100 字符发一次，避免刷屏
          if (s.reasoningBuf.length - s.reasoningSentLen >= 100) {
            s.reasoningSentLen = s.reasoningBuf.length;
            return [{
              type: 'state_update',
              data: { phase: 'running', step: 'AI 思考中', detail: s.reasoningBuf.slice(-300) },
            }];
          }
          return [];
        }

        // 工具调用增量（只记录映射，不发事件）
        case 'tool-call-delta': {
          if (chunk.id && chunk.name) {
            s.callIdToName.set(chunk.id, chunk.name);
          }
          return [];
        }

        // block 边界
        case 'block-start': {
          if (chunk.blockType === 'reasoning') {
            s.reasoningBuf = '';
            s.reasoningSentLen = 0;
            return [{ type: 'state_update', data: { phase: 'running', step: 'AI 思考中', detail: '正在思考...' } }];
          }
          // text block start — 不发 message start，等第一个 text-delta
          return [];
        }

        case 'block-end': {
          // reasoning block 结束 — 发送最终推理摘要
          if (s.reasoningBuf && s.reasoningSentLen < s.reasoningBuf.length) {
            s.reasoningSentLen = s.reasoningBuf.length;
            return [{
              type: 'state_update',
              data: { phase: 'running', step: 'AI 思考完成', detail: s.reasoningBuf.slice(-500) },
            }];
          }
          return [];
        }

        case 'usage':
        case 'finish':
          return [];

        default:
          return [];
      }
    }

    // ─── 完整 assistant 消息（去重：如果 text-delta 已发过则跳过）───
    case 'assistant/message': {
      const text = d.content?.map((c: any) => c?.text || '').join('') || d.text || '';
      if (!text) return [];

      // 如果 text-delta 已经流式发送了相同内容，只发 start:false 标记
      if (s.textStarted) {
        return [{ type: 'message', data: { text: '', start: false } }];
      }
      // 否则发完整文本（兜底）
      s.textStarted = true;
      return [
        { type: 'message', data: { text: '', start: true } },
        { type: 'message', data: { text } },
        { type: 'message', data: { text: '', start: false } },
      ];
    }

    // ─── 工具调用（每个工具只发一次 state_update）───
    case 'tool/call': {
      if (d.callId && d.name) {
        s.callIdToName.set(d.callId, d.name);
      }
      const toolName = d.name || '工具';
      const callKey = d.callId || toolName;
      if (!s.announcedTools.has(callKey)) {
        s.announcedTools.add(callKey);
        return [{ type: 'state_update', data: { phase: 'running', step: '工具调用', detail: `正在调用 ${toolName}` } }];
      }
      return [];
    }

    // ─── 工具结果 ───
    case 'tool/result': {
      const msg = d.message || {};
      const source = msg.source || {};
      const callId = source.callId || msg.tool_call_id || '';
      const toolName = (callId && s.callIdToName.get(callId)) || 'unknown';

      let fullText = '';
      let isError = false;
      if (Array.isArray(msg.content)) {
        for (const block of msg.content) {
          if (block?.isError) isError = true;
          if (Array.isArray(block?.content)) {
            for (const c of block.content) {
              if (c?.text) fullText += c.text;
            }
          } else if (typeof block?.text === 'string') {
            fullText += block.text;
          }
        }
      }

      const events: { type: string; data: any }[] = [];

      // 解析 __ARTIFACTS__ 标记块
      let observation = fullText;
      const marker = fullText.match(/__ARTIFACTS__([\s\S]*?)__END_ARTIFACTS__/);
      if (marker) {
        observation = fullText.replace(/__ARTIFACTS__[\s\S]*?__END_ARTIFACTS__/, '').trim();
        try {
          const artifacts = JSON.parse(marker[1]);
          if (Array.isArray(artifacts)) {
            for (const art of artifacts) {
              events.push({ type: 'artifact_update', data: { artifact: art } });
            }
          }
        } catch (e) { /* ignore */ }
      }

      events.push({
        type: 'tool_result',
        data: {
          tool: toolName,
          success: !isError,
          result: { observation },
          error: isError ? (observation || '工具执行失败') : undefined,
        },
      });

      return events;
    }

    default:
      return [];
  }
}

function readBody(req: http.IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    let body = '';
    req.on('data', (chunk) => { body += chunk; });
    req.on('end', () => resolve(body));
    req.on('error', reject);
  });
}
