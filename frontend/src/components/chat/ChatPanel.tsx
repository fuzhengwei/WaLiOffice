import * as DropdownMenu from '@radix-ui/react-dropdown-menu'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { AlertCircle, Check, Circle, Download, Eye, Files, Loader2, Send, Sparkles, Square, ChevronRight, ChevronDown, Terminal, Wrench, FileEdit, Sheet, PenTool, Image as ImageIcon, LayoutDashboard, Bot, Paperclip, X, Clapperboard, MessageSquarePlus } from 'lucide-react'
import { AGENT_TOOLS, getAgentTool } from '@/config/agent-tools'
import { useRef, useState, useEffect, Fragment, useMemo } from 'react'
import { FilePickerPanel } from './FilePickerPanel'
import type { AgentTraceEvent, Artifact, ChatAttachment, ChatMessage, InputRef, LLMProfile, PPTProject, ToolKind, ToolConfigMap } from '@/types'
import { findArtifactTurnGroup, groupArtifactsByTurn } from '@/lib/artifact-turns'
import { ToolConfigDropdown } from './ToolConfigDropdown'

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
  toolConfig: ToolConfigMap
  onProjectChange: (projectId: string | null) => void
  onNewProject?: () => void
  onModelChange: (model: string) => void
  onToolChange: (tool: ToolKind) => void
  onThemeChange: (v: string) => void
  onInputChange: (v: string) => void
  onSend: () => void
  onStop: () => void
  onToolConfigChange: (config: ToolConfigMap) => void
  attachments: ChatAttachment[]
  onPickAttachments: () => void
  onRemoveAttachment: (id: string) => void
  inputRefs: InputRef[]
  onRemoveInputRef: (id: string) => void
  onOpenArtifact: (artifactId: string) => void
  onExportArtifact: (artifact: Artifact) => void
  onInsertArtifact: (artifact: Artifact, sessionId?: string) => void
  /** 历史会话产物列表（用于 @ 引用历史产物） */
  historyArtifacts?: { artifact: Artifact; sessionTitle: string; sessionId: string }[]
  messagesEndRef: React.RefObject<HTMLDivElement>
}

function renderMarkdown(content: string) {
  const normalized = content
    .trim()
    .replace(/^```(?:md|markdown)\s*/i, '')
    .replace(/\s*```$/, '')

  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        h1: ({ children }) => <h1 className="mt-3 text-lg font-bold text-surface-950">{children}</h1>,
        h2: ({ children }) => <h2 className="mt-3 text-base font-semibold text-surface-950">{children}</h2>,
        h3: ({ children }) => <h3 className="mt-3 font-semibold text-surface-950">{children}</h3>,
        p: ({ children }) => <p className="my-1.5">{children}</p>,
        strong: ({ children }) => <strong className="font-semibold text-surface-950">{children}</strong>,
        ul: ({ children }) => <ul className="my-2 list-disc space-y-1.5 pl-5">{children}</ul>,
        ol: ({ children }) => <ol className="my-2 list-decimal space-y-1.5 pl-5">{children}</ol>,
        li: ({ children }) => <li>{children}</li>,
        blockquote: ({ children }) => <blockquote className="my-2 border-l-2 border-surface-200 pl-3 text-surface-500">{children}</blockquote>,
        hr: () => <hr className="my-4 border-surface-200" />,
        table: ({ children }) => (
          <div className="my-3 max-w-full overflow-x-auto rounded-xl border border-surface-200">
            <table className="min-w-full border-collapse bg-white text-sm">{children}</table>
          </div>
        ),
        thead: ({ children }) => <thead className="bg-surface-50 text-surface-800">{children}</thead>,
        tr: ({ children }) => <tr className="border-b border-surface-200 last:border-b-0">{children}</tr>,
        th: ({ children }) => <th className="whitespace-nowrap px-3 py-2 text-left font-semibold">{children}</th>,
        td: ({ children }) => <td className="px-3 py-2 align-top text-surface-700">{children}</td>,
        code: ({ className, children }) => {
          const raw = String(children).replace(/\n$/, '')
          const isBlock = /language-/.test(className || '') || raw.includes('\n')
          if (isBlock) {
            return <code className="block overflow-auto rounded-2xl bg-surface-950 px-4 py-3 text-xs text-white shadow-inner">{raw}</code>
          }
          return <code className="rounded-md bg-surface-100 px-1.5 py-0.5 text-[0.9em] text-surface-800">{raw}</code>
        },
        pre: ({ children }) => <pre className="my-3">{children}</pre>,
      }}
    >
      {normalized}
    </ReactMarkdown>
  )
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

/** @ 引用产物的标签 chip 图标映射 */
const refIconMap: Record<string, typeof Bot> = {
  ppt: LayoutDashboard,
  document: FileEdit,
  markdown: FileEdit,
  sheet: Sheet,
  drawio: PenTool,
  image: ImageIcon,
  video: Clapperboard,
  code: Terminal,
  search: Sparkles,
  mixed: Bot,
}

const refColorMap: Record<string, string> = {
  ppt: 'bg-blue-50 text-blue-700 ring-blue-200',
  document: 'bg-emerald-50 text-emerald-700 ring-emerald-200',
  markdown: 'bg-emerald-50 text-emerald-700 ring-emerald-200',
  sheet: 'bg-amber-50 text-amber-700 ring-amber-200',
  drawio: 'bg-violet-50 text-violet-700 ring-violet-200',
  image: 'bg-pink-50 text-pink-700 ring-pink-200',
  video: 'bg-rose-50 text-rose-700 ring-rose-200',
  code: 'bg-slate-50 text-slate-700 ring-slate-200',
  search: 'bg-sky-50 text-sky-700 ring-sky-200',
  mixed: 'bg-surface-50 text-surface-700 ring-surface-200',
}

function InputRefChip({ refItem, onRemove }: { refItem: InputRef; onRemove?: (id: string) => void }) {
  const Icon = refIconMap[refItem.kind] || Bot
  const colorClass = refColorMap[refItem.kind] || refColorMap.mixed
  return (
    <div className={`group inline-flex max-w-full items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] font-medium ring-1 ${colorClass}`}>
      <Icon className="h-3.5 w-3.5 shrink-0" />
      <span className="max-w-[160px] truncate">{refItem.title}</span>
      {onRemove && (
        <button
          type="button"
          onClick={() => onRemove(refItem.id)}
          className="ml-0.5 rounded-full p-0.5 opacity-60 transition hover:bg-black/5 hover:opacity-100"
          title="移除引用"
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
  toolConfig,
  onProjectChange,
  onNewProject,
  onModelChange,
  onToolChange,
  onThemeChange,
  onInputChange,
  onSend,
  onStop,
  onToolConfigChange,
  attachments,
  onPickAttachments,
  onRemoveAttachment,
  inputRefs,
  onRemoveInputRef,
  historyArtifacts,
  onOpenArtifact,
  onExportArtifact,
  onInsertArtifact,
  messagesEndRef,
}: ChatPanelProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null)
  const tool = getAgentTool(activeTool)
  const logsEndRef = useRef<HTMLDivElement>(null)
  const [showArtifactPanel, setShowArtifactPanel] = useState(false)
  const sessionId = null
  const [processPanelExpanded, setProcessPanelExpanded] = useState(true)
  const [artifactSummaryExpanded, setArtifactSummaryExpanded] = useState(false)
  const [showFilePicker, setShowFilePicker] = useState(false)
  const [artifactPickerScope, setArtifactPickerScope] = useState<Artifact[]>(artifacts)
  const previousArtifactsLengthRef = useRef(artifacts.length)

  const [streamStartedAt, setStreamStartedAt] = useState<number | null>(null)
  const [elapsedSeconds, setElapsedSeconds] = useState(0)

  useEffect(() => {
    if (isStreaming) {
      setProcessPanelExpanded(true)
      return
    }
    if (streamPhase === 'done' || streamPhase === 'error') setProcessPanelExpanded(false)
  }, [isStreaming, streamPhase])

  useEffect(() => {
    if (isStreaming && artifacts.length > previousArtifactsLengthRef.current) {
      setArtifactSummaryExpanded(true)
    }
    previousArtifactsLengthRef.current = artifacts.length
  }, [artifacts.length, isStreaming])

  useEffect(() => {
    if (!isStreaming) {
      setStreamStartedAt(null)
      setElapsedSeconds(0)
      return
    }
    const startedAt = Date.now()
    setStreamStartedAt(startedAt)
    setElapsedSeconds(0)
    const timer = window.setInterval(() => {
      setElapsedSeconds(Math.max(1, Math.floor((Date.now() - startedAt) / 1000)))
    }, 1000)
    return () => window.clearInterval(timer)
  }, [isStreaming])

  const formatElapsed = (seconds: number) => {
    if (seconds < 60) return `${seconds}s`
    const minutes = Math.floor(seconds / 60)
    const rest = seconds % 60
    return `${minutes}m ${rest}s`
  }

  const summarizeStreamStatus = (status: string) => {
    if (/第\s*\d+\s*\/\s*\d+\s*页|幻灯片|大纲/.test(status)) return status
    if (/导出|下载/.test(status)) return status
    if (/失败|错误|失效/.test(status)) return status
    if (/连接|模型/.test(status)) return '正在连接模型'
    if (/附件|文件/.test(status)) return '正在整理附件'
    if (/工具|调用|执行/.test(status)) return '正在调用工具'
    if (/产物|生成|更新|创建|绘制/.test(status)) return '正在生成产物'
    if (/回复|整理/.test(status)) return '正在整理回复'
    if (/理解|需求|分析/.test(status)) return '正在分析需求'
    if (streamPhase === 'thinking') return '正在分析需求'
    if (streamPhase === 'generating') return '正在生成产物'
    if (streamPhase === 'finishing') return activeTool === 'general' ? '正在整理回复' : '正在收尾产物'
    return '正在处理任务'
  }

  const inputPlaceholder = input ? '' : `输入需求，@ 引用产物，描述要生成的${tool.artifactLabel}`
  const handleFilePickerSelect = (artifact: Artifact, sessionId?: string) => {
    setShowFilePicker(false)
    onInsertArtifact(artifact, sessionId)
    // 焦点回 textarea
    window.setTimeout(() => textareaRef.current?.focus(), 50)
  }

  const compactStreamStatus = summarizeStreamStatus(streamStatus)
  const elapsedLabel = streamStartedAt ? formatElapsed(elapsedSeconds) : '0s'

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    const nativeEvent = e.nativeEvent as KeyboardEvent & { isComposing?: boolean }
    // FilePickerPanel 打开时，textarea 不响应快捷键（由面板全局监听接管）
    if (showFilePicker) {
      // Enter 仍然需要阻止发送
      if (e.key === 'Enter' && !e.shiftKey) {
        e.preventDefault()
      }
      // 上下键阻止默认行为（避免 textarea 光标移动）
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
        e.preventDefault()
      }
      return
    }
    if (e.key === 'Enter' && !e.shiftKey) {
      if (nativeEvent.isComposing || e.currentTarget.dataset.composing === 'true') return
      e.preventDefault()
      onSend()
      return
    }
    // @ 快捷键触发文件选择浮层（不在输入法组词状态下）
    if (e.key === '@' && !(nativeEvent.isComposing || e.currentTarget.dataset.composing === 'true')) {
      // 仅当光标前是空格、换行或处于行首时触发（避免邮箱地址等误触）
      const ta = e.currentTarget
      const pos = ta.selectionStart
      const before = ta.value.slice(0, pos)
      const isAtStart = pos === 0 || /\s$/.test(before)
      if (isAtStart && (artifacts.length > 0 || (historyArtifacts && historyArtifacts.length > 0))) {
        e.preventDefault()
        setArtifactPickerScope(artifacts)
        setShowFilePicker(true)
      }
    }
    // ESC 关闭浮层
    if (e.key === 'Escape' && showFilePicker) {
      e.preventDefault()
      setShowFilePicker(false)
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
      textareaRef.current.style.height = `${Math.min(textareaRef.current.scrollHeight, 140)}px`
    }
  }

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
  const canSend = input.trim().length > 0 || attachments.length > 0 || inputRefs.length > 0
  const starterCards: Array<{ title: string; desc: string; prompt: string; tool: ToolKind; icon: typeof Bot; accent: string }> = [
    {
      title: '生成 PPT',
      desc: '年度总结汇报演示',
      prompt: '帮我做一份 2025 年度团队总结汇报 PPT，共 8 页，包含：年度业绩回顾、重点项目里程碑、团队成长故事、遇到的挑战与应对、明年展望，风格商务大气，配数据图表。',
      tool: 'ppt',
      icon: LayoutDashboard,
      accent: 'from-blue-500 to-indigo-500',
    },
    {
      title: '分析 Excel',
      desc: '学区房价格与升学率排序',
      prompt: '请帮我检索天津和平区学区房价格和重点小学升学率数据，按学校排名整理成表格，包含学校名称、学区房均价、初中升学率、重点高中录取率等指标，并生成一份可汇报的 Excel 文档。',
      tool: 'excel',
      icon: Sheet,
      accent: 'from-amber-500 to-orange-500',
    },
    {
      title: '写 Word 文档',
      desc: '新员工入职指南手册',
      prompt: '帮我写一份新员工入职指南，包含公司简介、组织架构、办公环境介绍、常用系统账号开通流程、考勤制度、福利待遇、新人 30 天成长计划，语气亲切友好，让新人看了不慌。',
      tool: 'doc',
      icon: FileEdit,
      accent: 'from-emerald-500 to-teal-500',
    },
    {
      title: '画流程图',
      desc: '请假审批流程图',
      prompt: '帮我画一个员工请假审批流程图：员工提交请假 → 直属领导审批 → 3天以上需部门总监审批 → HR 备案 → 通知本人结果，包含驳回和退回修改的分支，节点清晰，适合放进员工手册。',
      tool: 'drawio',
      icon: PenTool,
      accent: 'from-violet-500 to-fuchsia-500',
    },
    {
      title: 'AI 画图',
      desc: '团队团建活动海报',
      prompt: '帮我生成一张公司秋季团建活动海报，画面是阳光下的露营草地，大家围坐烧烤欢笑，背景有帐篷和远山，氛围轻松温暖，底部留出活动时间地点的文字区域。',
      tool: 'image',
      icon: ImageIcon,
      accent: 'from-pink-500 to-rose-500',
    },
    {
      title: '生成动画片',
      desc: '分镜制作动画短片',
      prompt: '帮我用分镜模式生成一段 15 秒的动画短片：一只小猫在办公室里踩键盘打字，结果屏幕上弹出了满屏的猫爪印，小猫吓得从椅子上摔下来，风格轻松搞笑，Q版萌系，分 3 个镜头。',
      tool: 'video',
      icon: Clapperboard,
      accent: 'from-rose-500 to-orange-500',
    },
  ]

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
    if (artifact.kind === 'image') return '图象结果已生成，可在右侧查看详情。'
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

      <div className="relative z-10 flex-1 overflow-y-auto px-3 pb-6 pt-6 md:px-5 md:pb-8 md:pt-10">
        <div className="mx-auto flex w-full max-w-3xl flex-col gap-5">

          {messages.length === 0 && (
            <div className="flex min-h-[50vh] flex-col items-center justify-center text-center">
              <div className="mb-5 flex h-14 w-14 items-center justify-center rounded-3xl border border-black/5 bg-white/80 shadow-[0_18px_50px_rgba(24,24,27,0.10)] backdrop-blur">
                <Sparkles className="h-7 w-7 text-surface-900" />
              </div>
              <h1 className="text-3xl font-semibold tracking-tight text-surface-950">今天要做什么？</h1>
              <p className="mt-3 max-w-xl text-sm leading-6 text-surface-500">
                WaLiOffice 在线智能办公 AI：先分析需求，再决策工具，最后绘制或生成产物。你可以先描述目标，也可以直接选工具开工。
              </p>
              <div className="mt-7 grid w-full max-w-3xl grid-cols-1 gap-3 sm:grid-cols-2">
                {starterCards.map((card) => {
                  const Icon = card.icon
                  return (
                    <button
                      key={card.title}
                      onClick={() => {
                        onToolChange(card.tool)
                        onInputChange(card.prompt)
                        window.setTimeout(() => textareaRef.current?.focus(), 0)
                      }}
                      disabled={isStreaming}
                      className="group rounded-3xl border border-black/5 bg-white/68 p-4 text-left shadow-sm backdrop-blur transition-all hover:-translate-y-0.5 hover:bg-white hover:shadow-[0_18px_38px_rgba(24,24,27,0.08)] disabled:opacity-50"
                    >
                      <span className="flex items-start gap-3">
                        <span className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl bg-gradient-to-br ${card.accent} text-white shadow-sm`}>
                          <Icon className="h-5 w-5" />
                        </span>
                        <span className="min-w-0">
                          <span className="block text-sm font-black text-surface-900">{card.title}</span>
                          <span className="mt-1 block text-xs leading-5 text-surface-500">{card.desc}</span>
                          <span className="mt-3 line-clamp-2 block text-[11px] leading-5 text-surface-400 group-hover:text-surface-600">{card.prompt}</span>
                        </span>
                      </span>
                    </button>
                  )
                })}
              </div>
              <div className="mt-3 flex w-full max-w-3xl flex-wrap justify-center gap-2">
                {tool.examples.slice(0, 3).map((suggestion) => (
                  <button
                    key={suggestion}
                    type="button"
                    onClick={() => onInputChange(suggestion)}
                    disabled={isStreaming}
                    className="rounded-full border border-black/5 bg-white/55 px-3 py-1.5 text-[11px] font-medium text-surface-500 transition hover:bg-white hover:text-surface-900 disabled:opacity-50"
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
                {msg.inputRefs && msg.inputRefs.length > 0 && (
                  <div className={`mt-2 flex flex-wrap gap-1.5 ${msg.role === 'user' ? 'justify-end' : 'justify-start'}`}>
                    {msg.inputRefs.map((refItem) => {
                      const Icon = refIconMap[refItem.kind] || Bot
                      const colorClass = msg.role === 'user'
                        ? 'bg-white/10 text-white ring-white/20'
                        : (refColorMap[refItem.kind] || refColorMap.mixed)
                      return (
                        <div
                          key={refItem.id}
                          className={`group inline-flex max-w-full items-center gap-1.5 rounded-lg px-2.5 py-1.5 text-[11px] font-medium ring-1 ${colorClass}`}
                        >
                          <Icon className="h-3.5 w-3.5 shrink-0" />
                          <span className="max-w-[160px] truncate">{refItem.title}</span>
                        </div>
                      )
                    })}
                  </div>
                )}
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

          <div ref={messagesEndRef} />
        </div>
      </div>

      <div className="relative z-20 shrink-0 bg-[#f6f4ef]/88 px-3 pb-2 pt-2 backdrop-blur-xl md:px-5 md:pb-3">
        <div className="mx-auto w-full max-w-3xl">
          {artifacts.length > 0 && (
            <div className="mb-2 overflow-hidden rounded-2xl border border-white/80 bg-[linear-gradient(135deg,rgba(255,255,255,0.96),rgba(249,247,240,0.88))] shadow-[0_8px_28px_rgba(24,24,27,0.08)] backdrop-blur-xl ring-1 ring-black/[0.03]">
              <button
                type="button"
                onClick={() => setArtifactSummaryExpanded((expanded) => !expanded)}
                className="relative flex w-full items-center justify-between gap-2 overflow-hidden px-3 py-2 text-left transition-colors hover:bg-white/55"
                aria-expanded={artifactSummaryExpanded}
              >
                <span className="pointer-events-none absolute inset-y-0 left-0 w-1/3 bg-[radial-gradient(circle_at_20%_30%,rgba(59,130,246,0.06),transparent_46%)]" />
                <div className="relative flex min-w-0 items-center gap-2">
                  <span className="flex h-7 w-7 shrink-0 items-center justify-center rounded-lg bg-surface-950 text-white shadow-sm">
                    <Files className="h-3.5 w-3.5" />
                  </span>
                  <span className="min-w-0">
                    <span className="flex items-center gap-1.5 text-xs font-bold text-surface-950">
                      产物汇总
                      <span className="rounded-full bg-emerald-50 px-1.5 py-0.5 text-[9px] font-semibold text-emerald-700 ring-1 ring-emerald-100">实时</span>
                    </span>
                    <span className="mt-0.5 block truncate text-[10px] text-surface-400">点击展开查看、预览和下载</span>
                  </span>
                </div>
                <div className="relative flex shrink-0 items-center gap-1.5">
                  <span className="rounded-full bg-white/82 px-2 py-1 text-[10px] font-bold text-surface-700 shadow-sm ring-1 ring-black/[0.05]">
                    {artifacts.length} 个 · {artifactTurnGroups.length} 轮
                  </span>
                  <span className="flex h-6 w-6 items-center justify-center rounded-full bg-white/75 text-surface-400 shadow-sm ring-1 ring-black/[0.05]">
                    <ChevronDown className={`h-3 w-3 transition-transform ${artifactSummaryExpanded ? 'rotate-180' : ''}`} />
                  </span>
                </div>
              </button>

              {artifactSummaryExpanded && (
                <div className="max-h-[220px] space-y-2 overflow-y-auto border-t border-white/70 bg-white/30 p-2 [scrollbar-gutter:stable]">
                  {artifactTurnGroups.map((group) => {
                    const expanded = expandedTurnKeys.includes(group.key)
                    return (
                      <div key={group.key} className="overflow-hidden rounded-xl border border-white/75 bg-[#fffdfa]/86 shadow-sm ring-1 ring-black/[0.02]">
                        <button
                          type="button"
                          onClick={() => setExpandedTurnKeys((current) => current.includes(group.key) ? current.filter((key) => key !== group.key) : [...current, group.key])}
                          className="flex w-full items-center justify-between gap-2 px-3 py-2 text-left transition-colors hover:bg-white/75"
                        >
                          <div className="min-w-0">
                            <div className="flex flex-wrap items-center gap-2">
                              <span className="rounded-full bg-primary-50 px-2.5 py-1 text-[11px] font-bold text-primary-700 ring-1 ring-primary-100">{group.title}</span>
                              <span className="rounded-full bg-surface-50 px-2.5 py-1 text-[11px] font-medium text-surface-500 ring-1 ring-black/[0.05]">{group.timeLabel}</span>
                              <span className="rounded-full bg-emerald-50 px-2.5 py-1 text-[11px] font-semibold text-emerald-700 ring-1 ring-emerald-100">已生成</span>
                            </div>
                            <div className="mt-1.5 truncate text-xs text-surface-500">这一轮沉淀 {group.artifacts.length} 个可预览、可下载的产物。</div>
                          </div>
                          <div className="flex items-center gap-2 text-surface-400">
                            <span className="rounded-full bg-white px-2.5 py-1 text-[11px] font-bold text-surface-600 shadow-sm ring-1 ring-black/[0.05]">{group.artifacts.length} 个</span>
                            <span className="flex h-7 w-7 items-center justify-center rounded-full bg-surface-50 ring-1 ring-black/[0.04]">
                              <ChevronDown className={`h-4 w-4 transition-transform ${expanded ? 'rotate-180' : ''}`} />
                            </span>
                          </div>
                        </button>

                        {expanded && (
                          <div className="border-t border-black/[0.04] bg-white/42 px-3 py-2">
                            <div className="flex gap-2 overflow-x-auto pb-1.5">
                              {group.artifacts.map((artifact) => {
                                const meta = artifactMeta[artifact.kind] || artifactMeta.mixed
                                const ArtifactIcon = meta.icon
                                const isActive = selectedArtifact?.id === artifact.id
                                return (
                                  <div
                                    key={artifact.id}
                                    className={`group relative inline-flex min-w-[180px] shrink-0 items-center gap-2 rounded-xl border px-2.5 py-1.5 text-xs transition-all ${
                                      isActive
                                        ? 'border-surface-900 bg-surface-950 text-white shadow-[0_12px_26px_rgba(24,24,27,0.20)]'
                                        : 'border-black/[0.06] bg-white/92 text-surface-600 shadow-sm hover:-translate-y-0.5 hover:bg-white hover:text-surface-900 hover:shadow-md'
                                    }`}
                                  >
                                    <button
                                      type="button"
                                      onClick={() => onOpenArtifact(artifact.id)}
                                      className="flex min-w-0 flex-1 items-center gap-2.5 text-left"
                                    >
                                      <span className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-lg ${isActive ? 'bg-white/12 text-white' : 'bg-surface-50 text-surface-700 ring-1 ring-black/[0.05]'}`}>
                                        <ArtifactIcon className="h-4 w-4" />
                                      </span>
                                      <span className="min-w-0 flex-1 text-left">
                                        <span className="block truncate font-semibold">{artifact.title || meta.label}</span>
                                        <span className={`mt-0.5 block text-[10px] ${isActive ? 'text-white/60' : 'text-surface-400'}`}>{meta.label}</span>
                                      </span>
                                      <span className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-bold ${isActive ? 'bg-white/12 text-white/85' : 'bg-surface-100 text-surface-500'}`}>{artifactExtension(artifact)}</span>
                                    </button>
                                    <button
                                      type="button"
                                      onClick={() => onInsertArtifact(artifact)}
                                      disabled={isStreaming}
                                      className={`shrink-0 rounded-full p-1 transition-all disabled:opacity-30 ${isActive ? 'text-white/70 hover:bg-white/15 hover:text-white' : 'text-surface-400 hover:bg-surface-100 hover:text-surface-700'}`}
                                      title="引用到对话"
                                    >
                                      <MessageSquarePlus className="h-3.5 w-3.5" />
                                    </button>
                                  </div>
                                )
                              })}
                            </div>
                          </div>
                        )}
                      </div>
                    )
                  })}

                  {selectedArtifact && (
                    <div className="rounded-xl border border-white/75 bg-white/88 p-2.5 shadow-sm ring-1 ring-black/[0.02]">
                      <div className="flex flex-wrap items-center justify-between gap-3">
                        <div className="min-w-0">
                          <div className="flex flex-wrap items-center gap-2">
                            {selectedArtifactTurn && <span className="rounded-full bg-primary-50 px-2.5 py-1 text-[11px] font-bold text-primary-700 ring-1 ring-primary-100">{selectedArtifactTurn.title}</span>}
                            <span className="rounded-full bg-surface-100 px-2.5 py-1 text-[11px] font-semibold text-surface-600 ring-1 ring-black/[0.04]">{(artifactMeta[selectedArtifact.kind] || artifactMeta.mixed).label}</span>
                            <span className="rounded-full bg-emerald-50 px-2.5 py-1 text-[11px] font-bold text-emerald-700 ring-1 ring-emerald-100">{selectedArtifact.status === 'ready' ? '已生成' : selectedArtifact.status}</span>
                          </div>
                          <div className="mt-2 truncate text-[15px] font-bold text-surface-950">{selectedArtifact.title || (artifactMeta[selectedArtifact.kind] || artifactMeta.mixed).label}</div>
                          <div className="mt-1 line-clamp-1 text-xs text-surface-500">{describeArtifact(selectedArtifact)}</div>
                        </div>
                        <div className="flex shrink-0 items-center gap-2">
                          <button type="button" onClick={() => onOpenArtifact(selectedArtifact.id)} className="inline-flex items-center gap-1.5 rounded-full bg-surface-950 px-3.5 py-2.5 text-xs font-bold text-white shadow-[0_10px_22px_rgba(24,24,27,0.20)] hover:bg-surface-800">
                            <Eye className="h-3.5 w-3.5" />
                            预览
                          </button>
                          <button type="button" onClick={() => onInsertArtifact(selectedArtifact)} disabled={isStreaming} className="inline-flex items-center gap-1.5 rounded-full border border-black/10 bg-white px-3.5 py-2.5 text-xs font-bold text-surface-700 shadow-sm hover:bg-surface-50 disabled:opacity-40">
                            <MessageSquarePlus className="h-3.5 w-3.5" />
                            引用
                          </button>
                          {(selectedArtifact.kind === 'ppt' || selectedArtifact.kind === 'document' || selectedArtifact.kind === 'markdown' || selectedArtifact.kind === 'sheet' || selectedArtifact.kind === 'drawio' || selectedArtifact.kind === 'image' || selectedArtifact.kind === 'video') && (
                            <button type="button" onClick={() => onExportArtifact(selectedArtifact)} className="inline-flex items-center gap-1.5 rounded-full border border-black/10 bg-white px-3.5 py-2.5 text-xs font-bold text-surface-700 shadow-sm hover:bg-surface-50">
                              <Download className="h-3.5 w-3.5" />
                              {(artifactMeta[selectedArtifact.kind] || artifactMeta.mixed).exportLabel || '下载'}
                            </button>
                          )}
                        </div>
                      </div>
                    </div>
                  )}
                </div>
              )}
            </div>
          )}
          <div className={`rounded-[1.6rem] border bg-white/90 p-2 shadow-[0_8px_28px_rgba(24,24,27,0.07)] backdrop-blur-xl transition-all ${
            isStreaming ? 'border-surface-300 ring-2 ring-white/55' : 'border-black/10 focus-within:border-surface-500 focus-within:ring-2 focus-within:ring-white/70'
          }`}>
            {attachments.length > 0 && (
              <div className="mb-1.5 flex flex-wrap gap-2 px-1">
                {attachments.map((attachment) => (
                  <AttachmentPreview
                    key={attachment.id}
                    attachment={attachment}
                    onRemove={isStreaming ? undefined : onRemoveAttachment}
                  />
                ))}
              </div>
            )}
            {inputRefs.length > 0 && (
              <div className="mb-1.5 flex flex-wrap gap-1.5 px-1">
                {inputRefs.map((refItem) => (
                  <InputRefChip
                    key={refItem.id}
                    refItem={refItem}
                    onRemove={isStreaming ? undefined : onRemoveInputRef}
                  />
                ))}
              </div>
            )}
            <div className="relative rounded-[1.3rem] border border-black/[0.05] bg-[#fcfbf8]/96 px-2.5 py-1.5 shadow-[inset_0_1px_0_rgba(255,255,255,0.85)]">
              {showFilePicker && !isStreaming && (
                <FilePickerPanel
                  artifacts={artifactPickerScope}
                  historyArtifacts={historyArtifacts}
                  onSelect={handleFilePickerSelect}
                  onClose={() => setShowFilePicker(false)}
                />
              )}
              <div className="min-h-[52px]">
                {isStreaming ? (
                  <div className="flex min-h-[44px] items-start text-[14px] leading-[1.6] text-surface-400">
                    <span>{tool.promptPlaceholder}</span>
                  </div>
                ) : (
                  <textarea
                    ref={textareaRef}
                    value={input}
                    onChange={handleInput}
                    onKeyDown={handleKeyDown}
                    onCompositionStart={handleCompositionStart}
                    onCompositionEnd={handleCompositionEnd}
                    placeholder={inputPlaceholder}
                    rows={2}
                    className="min-h-[44px] max-h-[140px] w-full resize-none border-0 bg-transparent p-0 text-[14px] leading-[1.6] text-surface-900 outline-none placeholder:text-surface-400"
                  />
                )}
              </div>

              <div className="mt-1.5 flex flex-wrap items-center justify-between gap-2.5">
                <div className="flex min-w-0 flex-1 flex-wrap items-center gap-1.5">
                  {isStreaming && (
                    <span className="inline-flex h-8 max-w-full items-center gap-1.5 rounded-full bg-white/80 px-2.5 text-[11px] font-medium text-surface-600 ring-1 ring-black/[0.04]">
                      <Loader2 className="h-3 w-3 shrink-0 animate-spin text-surface-400" />
                      <span className="truncate">{compactStreamStatus}</span>
                      <span className="shrink-0 text-surface-300">·</span>
                      <span className="shrink-0 text-surface-500">耗时 {elapsedLabel}</span>
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
                      {selectableModels.length > 1 && (
                        <div className="relative hidden sm:block">
                          <select
                            value={selectedModel}
                            onChange={(event) => onModelChange(event.target.value)}
                            disabled={isStreaming}
                            className="h-8 max-w-[120px] appearance-none rounded-full border border-black/[0.05] bg-white/90 px-3 pr-8 text-[11px] text-surface-600 outline-none transition-all hover:border-black/[0.08] hover:bg-white disabled:opacity-50"
                            title="选择模型"
                          >
                            {selectableModels.map((model) => (
                              <option key={model} value={model}>{model}</option>
                            ))}
                          </select>
                          <ChevronDown className="pointer-events-none absolute right-2.5 top-1/2 h-3.5 w-3.5 -translate-y-1/2 text-surface-400" />
                        </div>
                      )}
                      <div className="relative hidden sm:block">
                        <select
                          value={selectedProjectId || ''}
                          onChange={(event) => {
                            if (event.target.value === '__new_project__') {
                              onNewProject?.()
                              return
                            }
                            onProjectChange(event.target.value || null)
                          }}
                          disabled={isStreaming}
                          className="h-8 max-w-[132px] appearance-none rounded-full border border-black/[0.05] bg-white/90 px-3 pr-8 text-[11px] text-surface-600 outline-none transition-all hover:border-black/[0.08] hover:bg-white disabled:opacity-50"
                        >
                          <option value="">未选择项目</option>
                          {projects.map((item) => <option key={item.id} value={item.id}>{item.title}</option>)}
                          <option disabled value="__project_separator__">──────────</option>
                          <option value="__new_project__">＋ 新建项目</option>
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
            <div className="mt-1.5 flex items-center gap-1 overflow-x-auto rounded-[1.2rem] border border-black/[0.05] bg-[#f8f5ee]/86 px-2 py-1.5 backdrop-blur-sm md:flex-wrap">
            {AGENT_TOOLS.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => onToolChange(item.id)}
                disabled={isStreaming}
                className={`inline-flex shrink-0 items-center gap-1.5 rounded-full border px-3 py-1.5 text-[11px] font-medium transition-all disabled:opacity-50 ${
                  activeTool === item.id
                    ? 'border-surface-900 bg-surface-950 text-white shadow-sm'
                    : 'border-black/5 bg-white/70 text-surface-600 backdrop-blur hover:bg-white hover:text-surface-900'
                }`}
              >
                <span className={`h-1.5 w-1.5 rounded-full ${toolDot[item.id] || 'bg-surface-400'}`} />
                {item.shortName}
              </button>
            ))}
            <div className="ml-auto flex shrink-0 items-center">
              <ToolConfigDropdown
                activeTool={activeTool}
                toolConfig={toolConfig}
                onToolConfigChange={onToolConfigChange}
                disabled={isStreaming}
              />
            </div>
          </div>
          </div>
        </div>
      </div>
    </section>
  )
}
