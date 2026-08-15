import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import { AlertCircle, Check, Circle, Download, Eye, Files, Loader2, Palette, Send, Sparkles, Square, ChevronRight, ChevronDown, Terminal, Wrench, FileEdit, Sheet, PenTool, Image as ImageIcon, LayoutDashboard, Bot, Paperclip, X, Clapperboard } from 'lucide-react'
import { AGENT_TOOLS, getAgentTool } from '@/config/agent-tools'
import { useRef, useState, useEffect, Fragment, useMemo } from 'react'
import type { AgentTraceEvent, Artifact, ChatAttachment, ChatMessage, LLMProfile, PPTProject, ToolKind } from '@/types'
import { findArtifactTurnGroup, groupArtifactsByTurn } from '@/lib/artifact-turns'

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
  artifacts: Artifact[]
  activeArtifactId: string | null
  onProjectChange: (projectId: string | null) => void
  onModelChange: (model: string) => void
  onToolChange: (tool: ToolKind) => void
  onThemeChange: (v: string) => void
  onInputChange: (v: string) => void
  onSend: () => void
  onStop: () => void
  attachments: ChatAttachment[]
  onPickAttachments: () => void
  onRemoveAttachment: (id: string) => void
  onOpenArtifact: (artifactId: string) => void
  onExportArtifact: (artifact: Artifact) => void
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

function formatAttachmentSize(size?: number) {
  const value = size || 0
  if (value >= 1024 * 1024) return `${(value / (1024 * 1024)).toFixed(1)} MB`
  return `${Math.max(1, Math.round(value / 1024))} KB`
}

function AttachmentPreview({
  attachment,
  tone = 'light',
  onRemove,
}: {
  attachment: ChatAttachment
  tone?: 'light' | 'dark'
  onRemove?: (id: string) => void
}) {
  const isImage = attachment.kind === 'image' && attachment.data_url
  const dark = tone === 'dark'
  const shellClass = dark
    ? 'border-white/12 bg-white/10 text-white'
    : 'border-black/8 bg-[#f8f5ee] text-surface-700'
  const metaClass = dark ? 'text-white/65' : 'text-surface-400'

  if (isImage) {
    return (
      <div className={`group relative w-[168px] overflow-hidden rounded-2xl border ${shellClass}`}>
        <img
          src={attachment.data_url}
          alt={attachment.name}
          className="h-28 w-full object-cover"
        />
        <div className="space-y-1 px-3 py-2">
          <div className="flex items-center gap-1.5 text-[11px] font-medium">
            <ImageIcon className="h-3.5 w-3.5 shrink-0" />
            <span className="truncate">{attachment.name}</span>
          </div>
          <div className={`text-[10px] ${metaClass}`}>
            {formatAttachmentSize(attachment.size)}
            {attachment.compressed ? ' · 已压缩' : ''}
          </div>
        </div>
        {onRemove && (
          <button
            type="button"
            onClick={() => onRemove(attachment.id)}
            className="absolute right-2 top-2 inline-flex h-6 w-6 items-center justify-center rounded-full bg-black/55 text-white opacity-0 transition group-hover:opacity-100"
            title="移除图片"
          >
            <X className="h-3 w-3" />
          </button>
        )}
      </div>
    )
  }

  return (
    <div className={`inline-flex max-w-full items-center gap-2 rounded-full border px-3 py-1.5 text-[11px] ${shellClass}`}>
      <FileEdit className={`h-3.5 w-3.5 ${dark ? 'text-white/80' : 'text-emerald-600'}`} />
      <span className="max-w-[180px] truncate">{attachment.name}</span>
      <span className={metaClass}>{formatAttachmentSize(attachment.size)}</span>
      {onRemove && (
        <button
          type="button"
          onClick={() => onRemove(attachment.id)}
          className={`rounded-full p-0.5 ${dark ? 'hover:bg-white/10' : 'hover:bg-white'} ${metaClass}`}
          title="移除附件"
        >
          <X className="h-3 w-3" />
        </button>
      )}
    </div>
  )
}

const toolDot: Record<ToolKind, string> = {
  general: 'bg-sky-400',
  ppt: 'bg-blue-500',
  doc: 'bg-emerald-500',
  drawio: 'bg-violet-500',
  excel: 'bg-amber-500',
  image: 'bg-pink-500',
  video: 'bg-rose-500',
  code: 'bg-slate-500',
}

const toolIcon: Record<ToolKind, typeof Bot> = {
  general: Bot,
  ppt: LayoutDashboard,
  doc: FileEdit,
  drawio: PenTool,
  excel: Sheet,
  image: ImageIcon,
  video: Clapperboard,
  code: Terminal,
}

type LogStatus = 'running' | 'done' | 'error'

interface ParsedLog {
  id: number
  icon: typeof Bot
  title: string
  detail: string
  status: LogStatus
  highlight?: boolean
  isTool?: boolean
  toolName?: string
}

function parseLogEntry(log: string, idx: number): ParsedLog {
  // 工具调用日志: "工具 ppt ✓ 完成" or "工具 excel ✗ 失败"
  const toolMatch = log.match(/^工具\s+(\S+)\s+(✓|✗)\s*(.*)$/)
  if (toolMatch) {
    const tool = toolMatch[1] as ToolKind
    return {
      id: idx,
      icon: toolIcon[tool] || Wrench,
      title: `调用 ${toolMatch[1]}`,
      detail: toolMatch[3] || '',
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
    if (/来源|检索来源/i.test(step)) icon = Sparkles
    return { id: idx, icon, title: step, detail, status: 'running', highlight: /来源|检索来源/i.test(step) }
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
  artifacts,
  activeArtifactId,
  onProjectChange,
  onModelChange,
  onToolChange,
  onThemeChange,
  onInputChange,
  onSend,
  onStop,
  attachments,
  onPickAttachments,
  onRemoveAttachment,
  onOpenArtifact,
  onExportArtifact,
  messagesEndRef,
}: ChatPanelProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const tool = getAgentTool(activeTool)
  const logsEndRef = useRef<HTMLDivElement>(null)
  const [processPanelExpanded, setProcessPanelExpanded] = useState(true)

  useEffect(() => {
    if (isStreaming) {
      setProcessPanelExpanded(true)
      return
    }
    if (streamPhase === 'done' || streamPhase === 'error') setProcessPanelExpanded(false)
  }, [isStreaming, streamPhase])

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
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 196)}px`
    }
  }

  const themes = [
    {
      id: 'default',
      name: '默认',
      description: '柔和中性',
      swatchClass: 'from-slate-100 via-white to-stone-200',
    },
    {
      id: 'business',
      name: '商务',
      description: '稳重专业',
      swatchClass: 'from-slate-700 via-slate-500 to-slate-200',
    },
    {
      id: 'tech',
      name: '科技',
      description: '冷调未来',
      swatchClass: 'from-cyan-400 via-sky-500 to-indigo-600',
    },
    {
      id: 'warm',
      name: '暖橙',
      description: '温暖亲和',
      swatchClass: 'from-amber-300 via-orange-400 to-rose-400',
    },
    {
      id: 'minimal',
      name: '极简',
      description: '黑白克制',
      swatchClass: 'from-zinc-900 via-zinc-500 to-zinc-100',
    },
  ]
  const activeTheme = themes.find((theme) => theme.id === selectedTheme) || themes[0]

  // 解析日志
  const parsedLogs: ParsedLog[] = processLogs.map((log, idx) => parseLogEntry(log, idx))
  // 最后一条如果是 running 状态且正在流式，保持 running；否则标记 done
  const visibleLogs = parsedLogs.slice(-8)

  // 判断阶段
  const phaseConfig = [
    { key: 'thinking', label: '分析', icon: Brain, desc: '理解需求' },
    { key: 'generating', label: '决策', icon: Wrench, desc: '调用工具' },
    { key: 'finishing', label: activeTool === 'general' ? '回复' : '绘制', icon: FileEdit, desc: activeTool === 'general' ? '整理回复' : '生成产物' },
  ] as const
  const currentPhaseIndex = streamPhase === 'thinking' ? 0 : streamPhase === 'generating' ? 1 : streamPhase === 'finishing' || streamPhase === 'done' ? 2 : -1

  const selectableModels = Array.from(
    new Set(
      modelProfiles
        .flatMap((profile) => profile.models || [])
        .filter((model) => !/^agnes-(image|video)-/i.test(model))
    )
  )

  const showProcessPanel = isStreaming || traceEvents.length > 0 || processLogs.length > 0
  const artifactMeta: Record<string, { icon: typeof Bot; label: string; exportLabel?: string }> = {
    ppt: { icon: LayoutDashboard, label: 'PPT', exportLabel: '导出 PPTX' },
    document: { icon: FileEdit, label: 'Word 文档', exportLabel: '导出 DOCX' },
    markdown: { icon: FileEdit, label: 'Markdown 文档', exportLabel: '下载 MD' },
    drawio: { icon: PenTool, label: 'draw.io 图表', exportLabel: '下载 draw.io' },
    sheet: { icon: Sheet, label: 'Excel 表格', exportLabel: '导出 XLSX' },
    image: { icon: ImageIcon, label: '图片结果', exportLabel: '下载图片' },
    video: { icon: Clapperboard, label: '视频结果', exportLabel: '下载 MP4' },
    search: { icon: Sparkles, label: '搜索结果卡片' },
    code: { icon: Terminal, label: '代码结果' },
    mixed: { icon: Bot, label: '综合结果' },
  }
  const selectedArtifact = artifacts.find((artifact) => artifact.id === activeArtifactId) || artifacts[0] || null
  const artifactTurnGroups = useMemo(() => groupArtifactsByTurn(artifacts, messages), [artifacts, messages])
  const selectedArtifactTurn = findArtifactTurnGroup(selectedArtifact?.id || null, artifactTurnGroups)
  const [expandedTurnKeys, setExpandedTurnKeys] = useState<string[]>([])
  const canSend = input.trim().length > 0 || attachments.length > 0

  useEffect(() => {
    setExpandedTurnKeys((current) => {
      const validKeys = current.filter((key) => artifactTurnGroups.some((group) => group.key === key))
      const next = new Set(validKeys)
      if (artifactTurnGroups[0]) next.add(artifactTurnGroups[0].key)
      if (selectedArtifactTurn) next.add(selectedArtifactTurn.key)
      return Array.from(next)
    })
  }, [artifactTurnGroups, selectedArtifactTurn?.key])

  const describeArtifact = (artifact: Artifact) => {
    if (artifact.kind === 'ppt') {
      return `共 ${(artifact.content?.slides || []).length || artifact.content?.slide_count || 0} 页幻灯片，支持右侧预览和 PPTX 导出。`
    }
    if (artifact.kind === 'document') return '结构化文档已生成，支持右侧阅读和 DOCX 导出。'
    if (artifact.kind === 'markdown') return 'Markdown 文档已生成，支持右侧渲染和 .md 下载。'
    if (artifact.kind === 'drawio') return '图表已生成，支持右侧预览/编辑，也可以下载 draw.io 源文件。'
    if (artifact.kind === 'sheet') return '表格数据已生成，支持右侧查看并导出为 Excel。'
    if (artifact.kind === 'image') return '图像结果已生成，可在右侧查看详情。'
    if (artifact.kind === 'video') return '视频结果已生成，可在右侧直接播放和下载 mp4。'
    if (artifact.kind === 'search') return `已生成搜索结果卡片，来源：${artifact.content?.provider_label || artifact.content?.provider || '未知来源'}。`
    if (artifact.kind === 'code') return '代码结果已生成，可在右侧查看步骤与内容。'
    return '综合产物已生成，可在右侧继续查看详细内容。'
  }

  const artifactExtension = (artifact: Artifact) => {
    if (artifact.kind === 'ppt') return '.pptx'
    if (artifact.kind === 'document') return '.docx'
    if (artifact.kind === 'markdown') return '.md'
    if (artifact.kind === 'drawio') return '.drawio'
    if (artifact.kind === 'sheet') return '.xlsx'
    if (artifact.kind === 'image') return '.png'
    if (artifact.kind === 'video') return '.mp4'
    return '.file'
  }

  return (
    <section className="relative flex h-full min-h-0 flex-col overflow-hidden bg-transparent">
      <div className="pointer-events-none absolute inset-x-0 top-0 z-0 h-20 bg-gradient-to-b from-[#f6f4ef] via-[#f6f4ef]/80 to-transparent" />

      <div className="relative z-10 flex-1 overflow-y-auto px-5 pb-8 pt-10">
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
                {msg.attachments && msg.attachments.length > 0 && (
                  <div className={`mt-3 flex flex-wrap gap-2 ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                    {msg.attachments.map((attachment) => (
                      <AttachmentPreview
                        key={attachment.id}
                        attachment={attachment}
                        tone={msg.role === 'user' ? 'dark' : 'light'}
                      />
                    ))}
                  </div>
                )}
              </div>
            </div>
          ))}

          {/* 流式执行过程面板 — Codex 风格 */}
          {showProcessPanel && (
            <div className="w-full overflow-hidden rounded-2xl border border-black/[0.06] bg-white/60 shadow-[0_12px_40px_rgba(24,24,27,0.06)] backdrop-blur-xl">
              {/* 阶段指示器 — 紧凑横向 */}
              <button
                type="button"
                onClick={() => !isStreaming && setProcessPanelExpanded((expanded) => !expanded)}
                disabled={isStreaming}
                className="flex w-full items-center gap-1 border-b border-black/[0.04] px-3 py-2 text-left transition-colors hover:bg-white/45 disabled:cursor-default disabled:hover:bg-transparent"
                aria-expanded={processPanelExpanded}
              >
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
                <div className="ml-auto flex min-w-0 items-center gap-1.5 text-[11px] text-surface-400">
                  {isStreaming ? <Loader2 className="h-3 w-3 shrink-0 animate-spin" /> : streamPhase === 'error' ? <AlertCircle className="h-3 w-3 shrink-0 text-red-500" /> : <Check className="h-3 w-3 shrink-0 text-emerald-600" />}
                  <span className="max-w-[260px] truncate">{streamStatus}</span>
                  {!isStreaming && <ChevronDown className={`h-3 w-3 shrink-0 transition-transform ${processPanelExpanded ? 'rotate-180' : ''}`} />}
                </div>
              </button>

              {/* 执行步骤日志 — 逐行流式 */}
              {processPanelExpanded && visibleLogs.length > 0 && (
                <div className="max-h-[164px] overflow-y-auto px-3 py-2 [scrollbar-gutter:stable]">
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
                        <span className={log.highlight ? 'rounded-full bg-surface-100 px-1.5 py-0.5 text-surface-700' : 'text-surface-500'}>{log.title}</span>
                        {log.detail && <span className="text-surface-400">：{log.detail}</span>}
                      </div>
                    ))}
                  </div>
                  <div ref={logsEndRef} />
                </div>
              )}
            </div>
          )}

          {artifacts.length > 0 && (
            <div className="rounded-[1.75rem] border border-black/[0.06] bg-white/72 p-4 shadow-[0_18px_50px_rgba(24,24,27,0.07)] backdrop-blur-xl">
              <div className="mb-3 flex items-center justify-between gap-3">
                <div>
                  <div className="inline-flex items-center gap-2 text-sm font-semibold text-surface-950">
                    <Files className="h-4 w-4 text-surface-700" />
                    产物汇总
                  </div>
                  <div className="mt-0.5 text-xs text-surface-500">同一会话里生成过的文件与结果会累计沉淀在这里，继续追问时也可以随时回看、预览和下载。</div>
                </div>
                <div className="rounded-full bg-surface-100 px-2.5 py-1 text-[11px] font-semibold text-surface-500">
                  累计 {artifacts.length} 个产物 · {artifactTurnGroups.length} 轮
                </div>
              </div>
              <div className="space-y-3">
                {artifactTurnGroups.map((group) => {
                  const expanded = expandedTurnKeys.includes(group.key)
                  return (
                    <div key={group.key} className="overflow-hidden rounded-[1.35rem] border border-black/[0.05] bg-[#fcfbf8]">
                      <button
                        type="button"
                        onClick={() => setExpandedTurnKeys((current) => current.includes(group.key) ? current.filter((key) => key !== group.key) : [...current, group.key])}
                        className="flex w-full items-center justify-between gap-3 px-4 py-3 text-left"
                      >
                        <div className="min-w-0">
                          <div className="flex items-center gap-2">
                            <div className="rounded-full bg-surface-950 px-2.5 py-1 text-[11px] font-semibold text-white">
                              {group.title}
                            </div>
                            <div className="rounded-full bg-white px-2.5 py-1 text-[11px] font-medium text-surface-500 ring-1 ring-black/[0.05]">
                              {group.timeLabel}
                            </div>
                          </div>
                          <div className="mt-1 text-xs text-surface-500">这一轮共生成 {group.artifacts.length} 个产物，支持继续回看和右侧预览。</div>
                        </div>
                        <div className="flex items-center gap-2 text-surface-400">
                          <span className="rounded-full bg-white px-2.5 py-1 text-[11px] font-semibold text-surface-500 ring-1 ring-black/[0.05]">
                            {group.artifacts.length} 个
                          </span>
                          <ChevronDown className={`h-4 w-4 transition-transform ${expanded ? 'rotate-180' : ''}`} />
                        </div>
                      </button>

                      {expanded && (
                        <div className="border-t border-black/[0.04] px-4 py-3">
                          <div className="flex gap-2 overflow-x-auto pb-1">
                            {group.artifacts.map((artifact) => {
                              const meta = artifactMeta[artifact.kind] || artifactMeta.mixed
                              const ArtifactIcon = meta.icon
                              const isActive = selectedArtifact?.id === artifact.id
                              return (
                                <button
                                  key={artifact.id}
                                  type="button"
                                  onClick={() => onOpenArtifact(artifact.id)}
                                  className={`group inline-flex shrink-0 items-center gap-2 rounded-full border px-3 py-2 text-xs transition-all ${
                                    isActive
                                      ? 'border-surface-900 bg-surface-950 text-white shadow-sm'
                                      : 'border-black/[0.07] bg-white text-surface-600 hover:bg-white hover:text-surface-900'
                                  }`}
                                >
                                  <span className={`flex h-7 w-7 items-center justify-center rounded-full ${isActive ? 'bg-white/12 text-white' : 'bg-surface-50 text-surface-700 ring-1 ring-black/[0.05]'}`}>
                                    <ArtifactIcon className="h-3.5 w-3.5" />
                                  </span>
                                  <span className="max-w-[180px] truncate text-left">
                                    {artifact.title || meta.label}
                                  </span>
                                  <span className={`rounded-full px-2 py-0.5 text-[10px] font-semibold ${isActive ? 'bg-white/12 text-white/85' : 'bg-surface-100 text-surface-500'}`}>
                                    {artifactExtension(artifact)}
                                  </span>
                                </button>
                              )
                            })}
                          </div>
                        </div>
                      )}
                    </div>
                  )
                })}
              </div>

              {selectedArtifact && (
                <div className="mt-3 rounded-[1.4rem] border border-black/[0.06] bg-[#fcfbf8] p-4">
                  <div className="flex flex-wrap items-start justify-between gap-3">
                    <div className="min-w-0">
                      <div className="flex items-center gap-2">
                        {selectedArtifactTurn && (
                          <div className="rounded-full bg-primary-50 px-2.5 py-1 text-[11px] font-semibold text-primary-700">
                            {selectedArtifactTurn.title}
                          </div>
                        )}
                        <div className="rounded-full bg-white px-2.5 py-1 text-[11px] font-semibold text-surface-500 ring-1 ring-black/[0.05]">
                          {(artifactMeta[selectedArtifact.kind] || artifactMeta.mixed).label}
                        </div>
                        <div className="rounded-full bg-emerald-50 px-2.5 py-1 text-[11px] font-semibold text-emerald-700">
                          {selectedArtifact.status === 'ready' ? '已生成' : selectedArtifact.status}
                        </div>
                      </div>
                      <div className="mt-3 text-base font-semibold text-surface-950">
                        {selectedArtifact.title || (artifactMeta[selectedArtifact.kind] || artifactMeta.mixed).label}
                      </div>
                      <div className="mt-1 text-sm leading-6 text-surface-500">
                        {describeArtifact(selectedArtifact)}
                      </div>
                    </div>

                    <div className="flex shrink-0 items-center gap-2">
                      <button
                        type="button"
                        onClick={() => onOpenArtifact(selectedArtifact.id)}
                        className="inline-flex items-center gap-1.5 rounded-full bg-surface-950 px-3.5 py-2 text-xs font-semibold text-white hover:bg-surface-800"
                      >
                        <Eye className="h-3.5 w-3.5" />
                        右侧预览
                      </button>
                      {(selectedArtifact.kind === 'ppt' || selectedArtifact.kind === 'document' || selectedArtifact.kind === 'markdown' || selectedArtifact.kind === 'sheet' || selectedArtifact.kind === 'drawio' || selectedArtifact.kind === 'image' || selectedArtifact.kind === 'video') && (
                        <button
                          type="button"
                          onClick={() => onExportArtifact(selectedArtifact)}
                          className="inline-flex items-center gap-1.5 rounded-full border border-black/10 bg-white px-3.5 py-2 text-xs font-semibold text-surface-700 hover:bg-surface-50"
                        >
                          <Download className="h-3.5 w-3.5" />
                          {(artifactMeta[selectedArtifact.kind] || artifactMeta.mixed).exportLabel || '下载文件'}
                        </button>
                      )}
                    </div>
                  </div>
                </div>
              )}
            </div>
          )}
          <div ref={messagesEndRef} />
        </div>
      </div>

      <div className="relative z-20 shrink-0 bg-[#f6f4ef]/88 px-5 pb-4 pt-3 backdrop-blur-xl">
        <div className="mx-auto w-full max-w-3xl">
          <div className={`rounded-[2rem] border bg-white/90 p-2.5 shadow-[0_14px_36px_rgba(24,24,27,0.09)] backdrop-blur-xl transition-all ${
            isStreaming ? 'border-surface-300 ring-4 ring-white/55' : 'border-black/10 focus-within:border-surface-500 focus-within:ring-4 focus-within:ring-white/70'
          }`}>
            {attachments.length > 0 && (
              <div className="mb-2 flex flex-wrap gap-2 px-1">
                {attachments.map((attachment) => (
                  <AttachmentPreview
                    key={attachment.id}
                    attachment={attachment}
                    onRemove={isStreaming ? undefined : onRemoveAttachment}
                  />
                ))}
              </div>
            )}
            <div className="rounded-[1.55rem] border border-black/[0.05] bg-[#fcfbf8]/96 px-3 py-2 shadow-[inset_0_1px_0_rgba(255,255,255,0.85)]">
              <div className="min-h-[90px]">
                {!isStreaming && (
                  <textarea
                    ref={textareaRef}
                    value={input}
                    onChange={handleInput}
                    onKeyDown={handleKeyDown}
                    onCompositionStart={handleCompositionStart}
                    onCompositionEnd={handleCompositionEnd}
                    placeholder={tool.promptPlaceholder}
                    rows={2}
                    className="min-h-[72px] max-h-[180px] w-full resize-none border-0 bg-transparent p-0 text-[15px] leading-[1.7] text-surface-900 outline-none placeholder:text-surface-400"
                  />
                )}
              </div>

              <div className="mt-1.5 flex flex-wrap items-center justify-between gap-2.5">
                <div className="flex min-w-0 flex-1 flex-wrap items-center gap-1.5">
                  {isStreaming && (
                    <span className="inline-flex h-8 items-center rounded-full bg-white/80 px-2.5 text-[11px] font-medium text-surface-500 ring-1 ring-black/[0.04]">
                      {streamStatus}
                    </span>
                  )}
                  {artifacts.length > 0 && !isStreaming && (
                    <span className="inline-flex h-8 items-center rounded-full bg-white/68 px-2.5 text-[11px] font-medium text-surface-500 ring-1 ring-black/[0.04]">
                      {artifacts.length} 个产物 · {artifactTurnGroups.length} 轮
                    </span>
                  )}
                  {selectedArtifact && !isStreaming && (
                    <button
                      type="button"
                      onClick={() => onOpenArtifact(selectedArtifact.id)}
                      className="inline-flex h-8 items-center gap-1 rounded-full bg-white px-2.5 text-[11px] font-semibold text-surface-800 ring-1 ring-black/[0.06] shadow-[0_1px_2px_rgba(15,23,42,0.04)] transition-all hover:-translate-y-[0.5px] hover:bg-white hover:text-surface-950 hover:shadow-[0_3px_10px_rgba(15,23,42,0.08)]"
                    >
                      <Eye className="h-3 w-3" />
                      查看当前产物
                    </button>
                  )}
                </div>

                <div className="flex flex-wrap items-center justify-end gap-2">
                  {isStreaming ? (
                    <button
                      onClick={onStop}
                      className="inline-flex h-9 items-center gap-1.5 rounded-full bg-red-50 px-3 text-xs font-medium text-red-600 transition-colors hover:bg-red-100"
                      title="停止生成"
                    >
                      <Square className="h-3.5 w-3.5 fill-current" />
                      停止
                    </button>
                  ) : (
                    <>
                      <div className="relative">
                        <select
                          value={selectedProjectId || ''}
                          onChange={(event) => onProjectChange(event.target.value || null)}
                          disabled={isStreaming}
                          className="h-8 max-w-[132px] appearance-none rounded-full border border-black/[0.05] bg-white/90 px-3 pr-8 text-[11px] text-surface-600 outline-none transition-all hover:border-black/[0.08] hover:bg-white disabled:opacity-50"
                        >
                          <option value="">未选择项目</option>
                          {projects.map((item) => <option key={item.id} value={item.id}>{item.title}</option>)}
                        </select>
                        <ChevronDown className="pointer-events-none absolute right-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-surface-400" />
                      </div>
                      <div className="relative">
                        <select
                          value={selectedModel}
                          onChange={(event) => onModelChange(event.target.value)}
                          disabled={isStreaming}
                          className="h-8 max-w-[142px] appearance-none rounded-full border border-black/[0.05] bg-white/90 px-3 pr-8 text-[11px] text-surface-600 outline-none transition-all hover:border-black/[0.08] hover:bg-white disabled:opacity-50"
                        >
                          {(selectableModels.length ? selectableModels : [selectedModel]).map((model) => <option key={model} value={model}>{model}</option>)}
                        </select>
                        <ChevronDown className="pointer-events-none absolute right-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-surface-400" />
                      </div>
                      <div className="inline-flex items-center gap-1 rounded-full border border-black/[0.06] bg-[#fbfaf7] p-[3px] shadow-[inset_0_1px_0_rgba(255,255,255,0.9),0_1px_3px_rgba(15,23,42,0.05)]">
                        <button
                          type="button"
                          onClick={onPickAttachments}
                          className="inline-flex h-[30px] w-[30px] items-center justify-center rounded-full text-surface-500 transition-all hover:bg-white hover:text-surface-900"
                          title="上传文件"
                        >
                          <Paperclip className="h-3.5 w-3.5" />
                        </button>
                        <button
                          onClick={onSend}
                          disabled={!canSend}
                          className="inline-flex h-[30px] items-center gap-1.5 rounded-full bg-surface-950 px-3 text-[11px] font-semibold text-white shadow-[0_1px_2px_rgba(15,23,42,0.18)] transition-all hover:-translate-y-[0.5px] hover:bg-surface-900 hover:shadow-[0_4px_12px_rgba(15,23,42,0.18)] disabled:translate-y-0 disabled:bg-surface-200 disabled:text-surface-400 disabled:shadow-none"
                          title="发送"
                        >
                          <Send className="h-3.5 w-3.5" />
                          发送
                        </button>
                      </div>
                    </>
                  )}
                </div>
              </div>
            </div>
            <div className="mt-2 flex flex-wrap items-center gap-1.5 rounded-[1.45rem] border border-black/[0.05] bg-[#f8f5ee]/86 px-2 py-2 backdrop-blur-sm">
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
            <div className="ml-auto flex items-center">
              <DropdownMenu.Root>
                <DropdownMenu.Trigger asChild>
                  <button
                    type="button"
                    disabled={isStreaming}
                    className="inline-flex h-9 items-center gap-2 rounded-full border border-black/[0.06] bg-white/78 px-2.5 text-[11px] font-medium text-surface-700 shadow-[0_1px_2px_rgba(15,23,42,0.04)] transition-all hover:-translate-y-[0.5px] hover:bg-white hover:shadow-[0_6px_14px_rgba(15,23,42,0.08)] disabled:opacity-50"
                  >
                    <span className={`inline-flex h-5 w-5 items-center justify-center rounded-full bg-gradient-to-br ${activeTheme.swatchClass} shadow-[inset_0_1px_0_rgba(255,255,255,0.6)]`}>
                      <Palette className="h-3 w-3 text-white drop-shadow-[0_1px_1px_rgba(15,23,42,0.3)]" />
                    </span>
                    <span className="text-surface-500">主题</span>
                    <span className="font-semibold text-surface-900">{activeTheme.name}</span>
                    <ChevronDown className="h-3.5 w-3.5 text-surface-400" />
                  </button>
                </DropdownMenu.Trigger>

                <DropdownMenu.Portal>
                  <DropdownMenu.Content
                    align="end"
                    sideOffset={8}
                    className="z-50 min-w-[228px] rounded-2xl border border-black/[0.06] bg-[#fffdfa]/96 p-2 shadow-[0_18px_48px_rgba(15,23,42,0.16)] backdrop-blur-xl"
                  >
                    <div className="px-2 pb-2 pt-1">
                      <div className="text-[11px] font-semibold text-surface-900">选择主题风格</div>
                      <div className="mt-0.5 text-[10px] text-surface-400">当前生成内容会优先贴合所选风格。</div>
                    </div>
                    <div className="flex flex-col gap-1">
                      {themes.map((theme) => (
                        <DropdownMenu.Item
                          key={theme.id}
                          onSelect={() => onThemeChange(theme.id)}
                          className={`flex cursor-pointer items-center gap-2 rounded-2xl px-2 py-2 text-[11px] outline-none transition-colors ${
                            selectedTheme === theme.id
                              ? 'bg-surface-950 text-white shadow-sm'
                              : 'text-surface-700 hover:bg-white focus:bg-white'
                          }`}
                        >
                          <span className={`h-7 w-7 rounded-full bg-gradient-to-br ${theme.swatchClass} shadow-[inset_0_1px_0_rgba(255,255,255,0.6)]`} />
                          <span className="min-w-0 flex-1">
                            <span className="block font-semibold">{theme.name}</span>
                            <span className={`block text-[10px] ${selectedTheme === theme.id ? 'text-white/70' : 'text-surface-400'}`}>
                              {theme.description}
                            </span>
                          </span>
                          {selectedTheme === theme.id && <Check className="h-3.5 w-3.5" />}
                        </DropdownMenu.Item>
                      ))}
                    </div>
                  </DropdownMenu.Content>
                </DropdownMenu.Portal>
              </DropdownMenu.Root>
            </div>
          </div>
          </div>
        </div>
      </div>
    </section>
  )
}
