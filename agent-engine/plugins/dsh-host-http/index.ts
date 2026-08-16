/**
 * dsh-host-http: HTTP/SSE 接口插件
 *
 * 被 Rust 后端代理调用，提供 DSH Agent 的流式对话接口。
 * 使用 DSH AgentLoop 的 session event 系统获取流式输出。
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

    // 创建 agent
    const handle = await agents.create({
      sessionId,
      agentOptions: {
        provider: 'openai-compatible',
        model: model || process.env.LLM_TEXT_MODEL_DEFAULT || 'deepseek-chat',
      },
    });

    const agent = handle.agent;

    // 维护 callId → toolName 映射（用于 tool/result 事件）
    const callIdToName = new Map<string, string>();

    // 订阅 session 事件，转发为 SSE
    const eventListener = (subject: any, event: any) => {
      try {
        // tool/call: 记录 callId → name
        if (event?.type === 'tool/call') {
          const cid = event.data?.callId || '';
          const cname = event.data?.name || '';
          if (cid && cname) callIdToName.set(cid, cname);
        }

        const sseEvents = mapSessionEvent(event, callIdToName);
        for (const sseEvent of sseEvents) {
          res.write(`event: ${sseEvent.type}\ndata: ${JSON.stringify(sseEvent.data)}\n\n`);
        }
      } catch (e) {
        // ignore write errors
      }
    };

    // 监听 agent 上下文的 session 事件
    agent.ctx.on('session/event', eventListener);

    // 也监听 agent 错误事件
    agent.ctx.on('agent/error', (_subject: any, payload: any) => {
      res.write(`event: error\ndata: ${JSON.stringify({ message: payload?.error?.message || 'agent error' })}\n\n`);
    });

    // 发送用户消息
    agent.followup({ role: 'user', content: [{ type: 'text', text: message }] });

    // 等待 agent 完成当前工作
    await agent.whenIdle();

    // 清理
    

    res.write('event: done\ndata: {}\n\n');
  } catch (err: any) {
    res.write(`event: error\ndata: ${JSON.stringify({ message: err.message })}\n\n`);
  }
  res.end();
}

function mapSessionEvent(event: any, callIdToName?: Map<string, string>): { type: string; data: any }[] {
  if (!event || !event.type) return [];
  const d = event.data || {};

  switch (event.type) {
    case 'turn/start':
    case 'turn/end':
    case 'step/start':
    case 'step/end':
    case 'request/header':
      return []; // 内部事件，前端不需要

    case 'assistant/chunk': {
      const chunk = d.chunk;
      if (!chunk) return [];

      switch (chunk.type) {
        case 'text-delta': {
          const text = chunk.text || chunk.delta || '';
          if (text) return [{ type: 'message', data: { text } }];
          return [];
        }
        case 'reasoning-delta': {
          return [{ type: 'state_update', data: { step: '思考', detail: '正在推理...' } }];
        }
        case 'tool-call-delta': {
          if (chunk.id && chunk.name && callIdToName) {
            callIdToName.set(chunk.id, chunk.name);
          }
          const toolName = chunk.name || (chunk.id && callIdToName?.get(chunk.id)) || '工具';
          return [{ type: 'state_update', data: { step: '工具调用', detail: `正在调用 ${toolName}` } }];
        }
        case 'block-start': {
          if (chunk.blockType === 'reasoning') {
            return [{ type: 'state_update', data: { step: '思考', detail: '正在推理...' } }];
          }
          if (chunk.blockType === 'text') {
            return [{ type: 'message', data: { text: '', start: true } }];
          }
          return [];
        }
        case 'block-end':
        case 'usage':
        case 'finish':
          return [];
        default:
          return [];
      }
    }

    case 'assistant/message': {
      const text = d.content?.map((c: any) => c?.text || '').join('') || d.text || '';
      return [{ type: 'message', data: { text, start: false } }];
    }

    case 'tool/call': {
      if (d.callId && d.name && callIdToName) {
        callIdToName.set(d.callId, d.name);
      }
      return [{ type: 'state_update', data: { step: '工具调用', detail: `正在调用 ${d.name || '工具'}` } }];
    }

    case 'tool/result': {
      const msg = d.message || {};
      const source = msg.source || {};
      const callId = source.callId || msg.tool_call_id || '';
      const toolName = (callId && callIdToName?.get(callId)) || 'unknown';

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
      const artifactMarker = fullText.match(/__ARTIFACTS__([\s\S]*?)__END_ARTIFACTS__/);
      if (artifactMarker) {
        observation = fullText.replace(/__ARTIFACTS__[\s\S]*?__END_ARTIFACTS__/, '').trim();
        try {
          const artifacts = JSON.parse(artifactMarker[1]);
          if (Array.isArray(artifacts)) {
            for (const art of artifacts) {
              events.push({ type: 'artifact_update', data: { artifact: art } });
            }
          }
        } catch (e) {
          // JSON 解析失败，忽略
        }
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
