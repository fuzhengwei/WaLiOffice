import { AlertCircle, Check, Circle, Loader2, Palette, Send, Sparkles, Square, Wand2, ChevronRight, Terminal, Wrench, FileEdit, Sheet, PenTool, Image as ImageIcon, LayoutDashboard, Bot } from 'lucide-react'
import { AGENT_TOOLS, getAgentTool } from '@/config/agent-tools'
import { useRef, useState, useEffect, Fragment } from 'react'
import type { AgentTraceEvent, ChatMessage, LLMProfile, PPTProject, ToolKind } from '@/types'

interface ChatPanelProps {
  messages: ChatMessage[]
  input: string
  isStreaming: boolean
  streamStatus: string
  streamPhase: 'idle' | 'thinking' | 'generating' | 'finishing' | 'done' | 'error'
  processLogs: string[]
  traceEvents: AgentTraceEvent[]
  selectedTheme: string
  activeTool: ToolKind
  projects: PPTProject[]
  selectedProjectId: string | null
  modelProfiles: LLMProfile[]
  selectedModel: string
  onProjectChange: (projectId: string | null) => void
  onModelChange: (model: string) => void
  onToolChange: (tool: ToolKind) => void
  onThemeChange: (v: string) => void
  onInputChange: (v: string) => void
  onSend: () => void
  onStop: () => void
  messagesEndRef: React.RefObject<HTMLDivElement>
}

function renderMarkdown(content: string) {
  const normalized = content
    .trim()
    .replace(/^```(?:md|markdown)\s*/i, '')
    .replace(/\s*```$/, '')
  const lines = normalized.split('\n')
  const nodes: React.ReactNode[] = []
  let list: string[] = []
  let orderedList: string[] = []
  let code: string[] = []
  let inCode = false

  const inline = (text: string) => {
    const parts = text.split(/(`[^`]+`|\*\*[^*]+\*\*)/g)
    return parts.map((part, idx) => {
      if (part.startsWith('**') && part.endsWith('**')) {
        return <strong key={idx}>{part.slice(2, -2)}</strong>
      }
      if (part.startsWith('`') && part.endsWith('`')) {
        return <code key={idx} className="rounded-md bg-surface-100 px-1.5 py-0.5 text-[0.9em] text-surface-800">{part.slice(1, -1)}</code>
      }
      return <span key={idx}>{part}</span>
    })
  }

  const flushList = () => {
    if (list.length) {
      nodes.push(<ul key={`ul-${nodes.length}`} className="my-2 list-disc space-y-1.5 pl-5">{list.map((item, i) => <li key={i}>{inline(item)}</li>)}</ul>)
      list = []
    }
  }
  const flushOrderedList = () => {
    if (orderedList.length) {
      nodes.push(<ol key={`ol-${nodes.length}`} className="my-2 list-decimal space-y-1.5 pl-5">{orderedList.map((item, i) => <li key={i}>{inline(item)}</li>)}</ol>)
      orderedList = []
    }
  }
  const flushCode = () => {
    if (code.length) {
      nodes.push(<pre key={`code-${nodes.length}`} className="my-3 overflow-auto rounded-2xl bg-surface-950 px-4 py-3 text-xs text-white shadow-inner"><code>{code.join('\n')}</code></pre>)
      code = []
    }
  }

  lines.forEach((line, idx) => {
    if (line.trim().startsWith('```')) {
      if (inCode) { inCode = false; flushCode() } else { flushList(); flushOrderedList(); inCode = true }
      return
    }
    if (inCode) { code.push(line); return }

    const trimmed = line.trim()
    if (!trimmed) { flushList(); flushOrderedList(); nodes.push(<div key={`br-${idx}`} className="h-1.5" />); return }
    const bullet = trimmed.match(/^[-*•]\s+(.+)$/)
    if (bullet) { list.push(bullet[1]); return }
    const ordered = trimmed.match(/^\d+\.\s+(.+)$/)
    if (ordered) { orderedList.push(ordered[1]); return }
    flushList()
    flushOrderedList()
    if (trimmed.startsWith('### ')) nodes.push(<h3 key={idx} className="mt-3 font-semibold text-surface-950">{inline(trimmed.slice(4))}</h3>)
    else if (trimmed.startsWith('## ')) nodes.push(<h2 key={idx} className="mt-4 text-base font-semibold text-surface-950">{inline(trimmed.slice(3))}</h2>)
    else if (trimmed.startsWith('# ')) nodes.push(<h1 key={idx} className="mt-4 text-lg font-bold text-surface-950">{inline(trimmed.slice(2))}</h1>)
    else if (trimmed.startsWith('> ')) nodes.push(<blockquote key={idx} className="my-2 border-l-2 border-surface-200 pl-3 text-surface-500">{inline(trimmed.slice(2))}</blockquote>)
    else nodes.push(<p key={idx} className="my-1.5">{inline(line)}</p>)
  })
  flushList(); flushOrderedList(); flushCode()
  return nodes
}

const toolDot: Record<ToolKind, string> = {
  general: 'bg-zinc-900',
  ppt: 'bg-blue-500',
  doc: 'bg-emerald-500',
  drawio: 'bg-violet-500',
  excel: 'bg-amber-500',
  image: 'bg-pink-500',
  code: 'bg-slate-500',
}

const toolIcon: Record<ToolKind, typeof Bot> = {
  general: Bot,
  ppt: LayoutDashboard,
  doc: FileEdit,
  drawio: PenTool,
  excel: Sheet,
  image: ImageIcon,
  code: Terminal,
}

type LogStatus = 'running' | 'done' | 'error'

interface ParsedLog {
  id: number
  icon: typeof Bot
  title: string
  detail: string
  status: LogStatus
  isTool?: boolean
  toolName?: string
}

function parseLogEntry(log: string, idx: number): ParsedLog {
  // 工具调用日志: "工具 ppt ✓ 完成" or "工具 excel ✗ 失败"
  const toolMatch = log.match(/^工具\s+(\S+)\s+(✓|✗)/)
  if (toolMatch) {
    const tool = toolMatch[1] as ToolKind
    return {
      id: idx,
      icon: toolIcon[tool] || Wrench,
      title: `调用 ${toolMatch[1]}`,
      detail: '',
      status: toolMatch[2] === '✓' ? 'done' : 'error',
      isTool: true,
      toolName: toolMatch[1],
    }
  }

  // "step：detail" 格式
  const colonMatch = log.match(/^(.+?)：(.+)$/)
  if (colonMatch) {
    const step = colonMatch[1]
    const detail = colonMatch[2]
    // 根据关键词选图标
    let icon = Circle
    if (/思考|分析|理解|识别/i.test(step)) icon = Brain
    if (/工具|调用|执行|分发/i.test(step)) icon = Wrench
    if (/完成|done/i.test(step)) icon = Check
    if (/生成|绘制|创建/i.test(step)) icon = FileEdit
    return { id: idx, icon, title: step, detail, status: 'running' }
  }

  return { id: idx, icon: Circle, title: log, detail: '', status: 'running' }
}

// 动态导入 Brain 图标（lucide 没有导出 Brain，用 BrainCircuit 代替）
import { BrainCircuit as Brain } from 'lucide-react'

export function ChatPanel({
  messages,
  input,
  isStreaming,
  streamStatus,
  streamPhase,
  processLogs,
  traceEvents,
  selectedTheme,
  activeTool,
  projects,
  selectedProjectId,
  modelProfiles,
  selectedModel,
  onProjectChange,
  onModelChange,
  onToolChange,
  onThemeChange,
  onInputChange,
  onSend,
  onStop,
  messagesEndRef,
}: ChatPanelProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const tool = getAgentTool(activeTool)
  const logsEndRef = useRef<HTMLDivElement>(null)

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const nativeEvent = e.nativeEvent as KeyboardEvent & { isComposing?: boolean }
    if (e.key === 'Enter' && !e.shiftKey) {
      if (nativeEvent.isComposing || e.currentTarget.dataset.composing === 'true') return
      e.preventDefault()
      onSend()
    }
  }

  const handleCompositionStart = (e: React.CompositionEvent<HTMLTextAreaElement>) => {
    e.currentTarget.dataset.composing = 'true'
  }

  const handleCompositionEnd = (e: React.CompositionEvent<HTMLTextAreaElement>) => {
    e.currentTarget.dataset.composing = 'false'
  }

  const handleInput = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
    onInputChange(e.target.value)
    if (textareaRef.current) {
      textareaRef.current.style.height = 'auto'
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 148)}px`
    }
  }

  const themes = [
    { id: 'default', name: '默认' },
    { id: 'business', name: '商务' },
    { id: 'tech', name: '科技' },
    { id: 'warm', name: '暖橙' },
    { id: 'minimal', name: '极简' },
  ]

  // 解析日志
  const parsedLogs: ParsedLog[] = processLogs.map((log, idx) => parseLogEntry(log, idx))
  // 最后一条如果是 running 状态且正在流式，保持 running；否则标记 done
  const visibleLogs = parsedLogs.slice(-8)

  // 判断阶段
  const phaseConfig = [
    { key: 'thinking', label: '分析', icon: Brain, desc: '理解需求' },
    { key: 'generating', label: '决策', icon: Wrench, desc: '调用工具' },
    { key: 'finishing', label: '绘制', icon: FileEdit, desc: '生成产物' },
  ] as const
  const currentPhaseIndex = streamPhase === 'thinking' ? 0 : streamPhase === 'generating' ? 1 : streamPhase === 'finishing' || streamPhase === 'done' ? 2 : -1

  const selectableModels = Array.from(new Set(modelProfiles.flatMap((profile) => profile.models || [])))

  const showProcessPanel = isStreaming || traceEvents.length > 0 || processLogs.length > 0

  return (
    <section className="relative flex h-full min-h-0 flex-col overflow-hidden bg-transparent">
      <div className="pointer-events-none absolute inset-x-0 top-0 z-10 h-24 bg-gradient-to-b from-[#f6f4ef] via-[#f6f4ef]/90 to-transparent" />

      <div className="relative z-0 flex-1 overflow-y-auto px-5 pb-52 pt-8">
        <div className="mx-auto flex w-full max-w-3xl flex-col gap-5">
          {messages.length === 0 && (
            <div className="flex min-h-[48vh] flex-col items-center justify-center text-center">
              <div className="mb-5 flex h-14 w-14 items-center justify-center rounded-3xl border border-black/5 bg-white/80 shadow-[0_18px_50px_rgba(24,24,27,0.10)] backdrop-blur">
                <Sparkles className="h-7 w-7 text-surface-900" />
              </div>
              <h1 className="text-3xl font-semibold tracking-tight text-surface-950">今天要做什么？</h1>
              <p className="mt-3 max-w-xl text-sm leading-6 text-surface-500">
                像 Codex 一样把过程透明化：先分析需求，再决策工具，最后绘制或生成产物。你可以先描述目标，也可以直接选工具开工。
              </p>
              <div className="mt-7 grid w-full max-w-2xl grid-cols-1 gap-2 sm:grid-cols-3">
                {tool.examples.slice(0, 3).map((suggestion) => (
                  <button
                    key={suggestion}
                    onClick={() => onInputChange(suggestion)}
                    disabled={isStreaming}
                    className="rounded-2xl border border-black/5 bg-white/65 px-4 py-3 text-left text-xs leading-relaxed text-surface-600 shadow-sm backdrop-blur transition-all hover:-translate-y-0.5 hover:bg-white hover:shadow-md disabled:opacity-50"
                  >
                    {suggestion}
                  </button>
                ))}
              </div>
            </div>
          )}

          {messages.map((msg, i) => (
            <div key={i} className={`flex ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
              <div
                className={`max-w-[86%] rounded-[1.4rem] px-4 py-3 text-sm leading-relaxed shadow-sm ${
                  msg.role === 'user'
                    ? 'bg-surface-950 text-white shadow-[0_12px_30px_rgba(24,24,27,0.16)]'
                    : 'border border-black/5 bg-white/78 text-surface-800 shadow-[0_12px_36px_rgba(24,24,27,0.07)] backdrop-blur'
                }`}
              >
                {msg.content ? (
                  msg.role === 'assistant' ? renderMarkdown(msg.content) : msg.content
                ) : (msg.role === 'assistant' && isStreaming ? (
                  <span className="inline-flex items-center gap-2 text-surface-500">
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                    {streamStatus || '正在处理...'}
                  </span>
                ) : '')}
              </div>
            </div>
          ))}

          {/* 流式执行过程面板 — Codex 风格 */}
          {showProcessPanel && (
            <div className="overflow-hidden rounded-2xl border border-black/[0.06] bg-white/60 shadow-[0_12px_40px_rgba(24,24,27,0.06)] backdrop-blur-xl">
              {/* 阶段指示器 — 紧凑横向 */}
              <div className="flex items-center gap-1 border-b border-black/[0.04] px-3 py-2">
                {phaseConfig.map((phase, idx) => {
                  const PhaseIcon = phase.icon
                  const active = idx === currentPhaseIndex && isStreaming
                  const done = streamPhase === 'done' || idx < currentPhaseIndex
                  return (
                    <Fragment key={phase.key}>
                      {idx > 0 && <ChevronRight className="h-3 w-3 text-surface-300" />}
                      <div className={`flex items-center gap-1.5 px-2 py-1 rounded-lg text-xs font-medium transition-all ${
                        active ? 'bg-surface-950 text-white' :
                        done ? 'text-emerald-600' :
                        'text-surface-400'
                      }`}>
                        {active ? <Loader2 className="h-3 w-3 animate-spin" /> :
                         done ? <Check className="h-3 w-3" /> :
                         <PhaseIcon className="h-3 w-3" />}
                        {phase.label}
                      </div>
                    </Fragment>
                  )
                })}
                <div className="ml-auto flex items-center gap-1.5 text-[11px] text-surface-400">
                  {isStreaming ? <Loader2 className="h-3 w-3 animate-spin" /> : streamPhase === 'error' ? <AlertCircle className="h-3 w-3 text-red-500" /> : <Check className="h-3 w-3 text-emerald-600" />}
                  <span className="max-w-[200px] truncate">{streamStatus}</span>
                </div>
              </div>

              {/* 执行步骤日志 — 逐行流式 */}
              {visibleLogs.length > 0 && (
                <div className="max-h-[180px] overflow-y-auto px-3 py-2">
                  <div className="space-y-0.5 font-mono text-[11px] leading-relaxed">
                    {visibleLogs.map((log) => (
                      <div key={log.id} className="flex items-start gap-2 py-0.5">
                        <span className="mt-0.5 shrink-0">
                          {log.status === 'done' ? (
                            <Check className="h-3 w-3 text-emerald-500" />
                          ) : log.status === 'error' ? (
                            <AlertCircle className="h-3 w-3 text-red-500" />
                          ) : isStreaming && log.id === visibleLogs[visibleLogs.length - 1]?.id ? (
                            <Loader2 className="h-3 w-3 animate-spin text-surface-400" />
                          ) : (
                            <Check className="h-3 w-3 text-surface-300" />
                          )}
                        </span>
                        <span className="text-surface-500">{log.title}</span>
                        {log.detail && <span className="text-surface-400">：{log.detail}</span>}
                      </div>
                    ))}
                  </div>
                  <div ref={logsEndRef} />
                </div>
              )}
            </div>
          )}
          <div ref={messagesEndRef} />
        </div>
      </div>

      <div className="absolute inset-x-0 bottom-0 z-20 px-5 pb-5 pt-12 bg-gradient-to-t from-[#f6f4ef] via-[#f6f4ef]/95 to-transparent">
        <div className="mx-auto w-full max-w-3xl">
          <div className="mb-2 flex flex-wrap items-center gap-1.5 px-1">
            {AGENT_TOOLS.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => onToolChange(item.id)}
                disabled={isStreaming}
                className={`inline-flex items-center gap-1.5 rounded-full border px-3 py-1.5 text-[11px] font-medium transition-all disabled:opacity-50 ${
                  activeTool === item.id
                    ? 'border-surface-900 bg-surface-950 text-white shadow-sm'
                    : 'border-black/5 bg-white/70 text-surface-600 backdrop-blur hover:bg-white hover:text-surface-900'
                }`}
              >
                <span className={`h-1.5 w-1.5 rounded-full ${toolDot[item.id] || 'bg-surface-400'}`} />
                {item.shortName}
              </button>
            ))}
            <div className="ml-auto hidden items-center gap-1.5 text-[11px] text-surface-400 sm:flex">
              <Palette className="h-3.5 w-3.5" />
              {themes.map((theme) => (
                <button
                  key={theme.id}
                  type="button"
                  onClick={() => onThemeChange(theme.id)}
                  disabled={isStreaming}
                  className={`rounded-full px-2 py-1 transition-colors ${selectedTheme === theme.id ? 'bg-white text-surface-900 shadow-sm' : 'hover:bg-white/70'}`}
                >
                  {theme.name}
                </button>
              ))}
            </div>
          </div>

          <div className={`rounded-[1.75rem] border bg-white/86 p-2 shadow-[0_24px_80px_rgba(24,24,27,0.18)] backdrop-blur-2xl transition-all ${
            isStreaming ? 'border-surface-300 ring-4 ring-white/55' : 'border-black/10 focus-within:border-surface-500 focus-within:ring-4 focus-within:ring-white/70'
          }`}>
            <div className="mb-1 flex flex-wrap items-center justify-between gap-2 px-3 pt-1.5 text-[11px] text-surface-400">
              <span className="inline-flex items-center gap-1.5"><Wand2 className="h-3.5 w-3.5" /> 当前工具：{tool.name}</span>
              <div className="flex flex-wrap items-center gap-2">
                <select
                  value={selectedProjectId || ''}
                  onChange={(event) => onProjectChange(event.target.value || null)}
                  disabled={isStreaming}
                  className="max-w-[170px] rounded-full border border-black/5 bg-white/80 px-2 py-1 text-[11px] text-surface-600 outline-none disabled:opacity-50"
                >
                  <option value="">未选择项目</option>
                  {projects.map((item) => <option key={item.id} value={item.id}>{item.title}</option>)}
                </select>
                <select
                  value={selectedModel}
                  onChange={(event) => onModelChange(event.target.value)}
                  disabled={isStreaming}
                  className="max-w-[160px] rounded-full border border-black/5 bg-white/80 px-2 py-1 text-[11px] text-surface-600 outline-none disabled:opacity-50"
                >
                  {(selectableModels.length ? selectableModels : [selectedModel]).map((model) => <option key={model} value={model}>{model}</option>)}
                </select>
                <span>{isStreaming ? streamStatus : 'Enter 发送 · Shift+Enter 换行'}</span>
              </div>
            </div>
            <div className="flex items-end gap-2">
              <textarea
                ref={textareaRef}
                value={input}
                onChange={handleInput}
                onKeyDown={handleKeyDown}
                onCompositionStart={handleCompositionStart}
                onCompositionEnd={handleCompositionEnd}
                placeholder={isStreaming ? '正在执行中，可点击停止...' : tool.promptPlaceholder}
                rows={1}
                disabled={isStreaming}
                className="min-h-[52px] max-h-[148px] flex-1 resize-none border-0 bg-transparent px-3 py-3 text-[15px] leading-6 text-surface-900 outline-none placeholder:text-surface-400 disabled:text-surface-400"
              />
              {isStreaming ? (
                <button
                  onClick={onStop}
                  className="mb-1 inline-flex h-11 w-11 items-center justify-center rounded-2xl bg-red-50 text-red-600 transition-colors hover:bg-red-100"
                  title="停止生成"
                >
                  <Square className="h-4 w-4 fill-current" />
                </button>
              ) : (
                <button
                  onClick={onSend}
                  disabled={!input.trim()}
                  className="mb-1 inline-flex h-11 w-11 items-center justify-center rounded-2xl bg-surface-950 text-white shadow-sm transition-all hover:-translate-y-0.5 hover:bg-surface-800 disabled:translate-y-0 disabled:bg-surface-200 disabled:text-surface-400"
                  title="发送"
                >
                  <Send className="h-4 w-4" />
                </button>
              )}
            </div>
          </div>
        </div>
      </div>
    </section>
  )
}
