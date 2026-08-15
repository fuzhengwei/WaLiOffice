import { Bot, BrainCircuit, ChevronDown, Clapperboard, Edit3, FileText, Folder, Image, LayoutDashboard, LogOut, MessageSquare, MoreHorizontal, PenTool, Plus, Search, Settings as SettingsIcon, Sheet, Sparkles, SquareCheckBig, Trash2 } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import type { CSSProperties, DragEvent } from 'react'
import { useLocation, useNavigate } from 'react-router-dom'
import type { ChatMessage, ConversationRecord, PPTProject, ToolKind, ProjectMeta } from '@/types'

const LOGO_URL = '/logo.png'

interface ConversationSidebarProps {
  project: PPTProject | null
  messages: ChatMessage[]
  conversations: ConversationRecord[]
  projects: ProjectMeta[]
  activeProjectId: string | null
  userName?: string
  activeTool: ToolKind
  activeConversationId?: string | null
  activeView?: 'chat' | 'settings'
  onToolChange: (tool: ToolKind) => void
  onSelectConversation?: (id: string) => void
  onSelectProject?: (projectId: string) => void
  onNewConversation?: () => void
  onRenameConversation?: (id: string, title: string) => void
  onDeleteConversation?: (id: string) => void
  onDeleteProject?: (projectId: string) => void
  onMoveConversation?: (id: string, projectId: string | null, beforeId?: string | null) => void
  onOpenSettings?: () => void
  onLogout?: () => void
  searchQuery?: string
  onSearchQueryChange?: (query: string) => void
  width?: number
  onResizeStart?: (event: React.PointerEvent<HTMLDivElement>) => void
}

const iconMap: Record<ToolKind, any> = {
  general: Bot,
  ppt: LayoutDashboard,
  doc: FileText,
  drawio: PenTool,
  excel: Sheet,
  image: Image,
  video: Clapperboard,
  code: BrainCircuit,
}

const toolColors: Record<ToolKind, string> = {
  general: 'bg-sky-500',
  ppt: 'bg-blue-500',
  doc: 'bg-emerald-500',
  drawio: 'bg-violet-500',
  excel: 'bg-amber-500',
  image: 'bg-pink-500',
  video: 'bg-rose-500',
  code: 'bg-slate-500',
}

const toolLabel: Record<ToolKind, string> = {
  general: '综合',
  ppt: 'PPT',
  doc: 'docx',
  drawio: 'draw.io',
  excel: 'Excel',
  image: '图像',
  video: '视频',
  code: 'Code',
}

function formatTime(value?: string) {
  if (!value) return ''
  const d = new Date(value)
  if (Number.isNaN(d.getTime())) return ''
  const now = new Date()
  if (d.toDateString() === now.toDateString()) return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })
  return d.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' })
}

function firstUserPrompt(messages: ChatMessage[]) {
  return messages.find((msg) => msg.role === 'user')?.content || '新的对话'
}

function trimTitle(title?: string) {
  return (title || '未命名对话').replace(/\s+/g, ' ').trim()
}

function includesKeyword(value: string | undefined, keyword: string) {
  return (value || '').toLowerCase().includes(keyword.toLowerCase())
}

function sortConversations(items: ConversationRecord[]) {
  return [...items].sort((a, b) => (a.order_col || 0) - (b.order_col || 0) || Date.parse(b.updated_at || '') - Date.parse(a.updated_at || ''))
}

export function ConversationSidebar({
  project,
  messages,
  conversations,
  projects,
  activeProjectId,
  userName,
  activeTool,
  activeConversationId,
  activeView = 'chat',
  onSelectConversation,
  onSelectProject,
  onNewConversation,
  onRenameConversation,
  onDeleteConversation,
  onDeleteProject,
  onMoveConversation,
  onOpenSettings,
  onLogout,
  searchQuery = '',
  onSearchQueryChange,
  width,
  onResizeStart,
}: ConversationSidebarProps) {
  const navigate = useNavigate()
  const location = useLocation()
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set())
  const [showUnassigned, setShowUnassigned] = useState(true)
  const [hovered, setHovered] = useState<string | null>(null)
  const [draggingId, setDraggingId] = useState<string | null>(null)
  const [dropTarget, setDropTarget] = useState<{ projectId: string | null; beforeId: string | null } | null>(null)
  const [unassignedLimit, setUnassignedLimit] = useState(5)

  useEffect(() => {
    if (!activeProjectId) return
    setExpandedProjects((prev) => {
      if (prev.has(activeProjectId)) return prev
      const next = new Set(prev)
      next.add(activeProjectId)
      return next
    })
  }, [activeProjectId])

  const latestMessage = messages[messages.length - 1]
  const conversationTitle = project?.title || firstUserPrompt(messages)
  const shouldShowDraftConversation = messages.length > 0 && !activeConversationId
  const currentConversations = shouldShowDraftConversation
    ? [{
        id: 'current',
        title: conversationTitle,
        tool: activeTool,
        summary: latestMessage?.content || '正在处理当前任务...',
        updated_at: latestMessage?.timestamp || new Date().toISOString(),
        message_count: messages.length,
        project_id: activeProjectId || undefined,
      } as ConversationRecord, ...conversations.filter((item) => item.id !== 'current')]
    : conversations

  const { filteredProjects, unassignedConversations, conversationsByProject } = useMemo(() => {
    const keyword = searchQuery.trim()
    const filteredProjects = keyword
      ? projects.filter((proj) => includesKeyword(proj.title, keyword) || includesKeyword(proj.description, keyword))
      : projects
    const filteredConversations = keyword
      ? currentConversations.filter((item) =>
          includesKeyword(item.title, keyword)
          || includesKeyword(item.summary, keyword)
          || includesKeyword(item.project_title, keyword)
        )
      : currentConversations
    const byProject = new Map<string, ConversationRecord[]>()
    for (const proj of filteredProjects) {
      byProject.set(proj.id, sortConversations(filteredConversations.filter((c) => c.project_id === proj.id)))
    }
    return {
      filteredProjects,
      unassignedConversations: sortConversations(filteredConversations.filter((c) => !c.project_id)),
      conversationsByProject: byProject,
    }
  }, [projects, currentConversations, searchQuery])

  const workspaceLinks = [
    { to: '/', label: '智能助手', icon: Sparkles, active: location.pathname === '/' },
    { to: '/tasks', label: '任务清单', icon: SquareCheckBig, active: location.pathname.startsWith('/tasks') },
    { to: '/files', label: '我的文件', icon: Folder, active: location.pathname.startsWith('/files') },
  ]

  const toggleProject = (id: string) => {
    setExpandedProjects((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const handleRenameConversation = (item: ConversationRecord) => {
    const nextTitle = prompt('修改对话标题', trimTitle(item.title))?.trim()
    if (!nextTitle || nextTitle === trimTitle(item.title)) return
    onRenameConversation?.(item.id, nextTitle)
  }

  const handleDragStart = (event: DragEvent<HTMLDivElement>, item: ConversationRecord) => {
    if (item.id === 'current') return
    event.dataTransfer.effectAllowed = 'move'
    event.dataTransfer.setData('text/plain', item.id)
    setDraggingId(item.id)
  }

  const handleDragOver = (event: DragEvent<HTMLElement>, projectId: string | null, beforeId: string | null = null) => {
    if (!draggingId || draggingId === beforeId) return
    event.preventDefault()
    event.stopPropagation()
    event.dataTransfer.dropEffect = 'move'
    setDropTarget({ projectId, beforeId })
  }

  const handleDrop = (event: DragEvent<HTMLElement>, projectId: string | null, beforeId: string | null = null) => {
    event.preventDefault()
    event.stopPropagation()
    const id = event.dataTransfer.getData('text/plain') || draggingId
    setDraggingId(null)
    setDropTarget(null)
    if (!id || id === 'current' || id === beforeId) return
    const item = currentConversations.find((conv) => conv.id === id)
    if (!item) return
    if ((item.project_id || null) === projectId && id === beforeId) return
    onMoveConversation?.(id, projectId, beforeId)
  }

  const handleDragEnd = () => {
    setDraggingId(null)
    setDropTarget(null)
  }

  const isDropTarget = (projectId: string | null, beforeId: string | null = null) => (
    dropTarget?.projectId === projectId && dropTarget.beforeId === beforeId
  )

  const renderConversationItem = (item: ConversationRecord, child = false) => {
    const active = activeConversationId ? item.id === activeConversationId : item.id === 'current'
    const Icon = iconMap[item.tool] || MessageSquare

    if (child) {
      return (
        <div
          key={item.id}
          draggable={item.id !== 'current'}
          onDragStart={(event) => handleDragStart(event, item)}
          onDragOver={(event) => handleDragOver(event, item.project_id || null, item.id)}
          onDrop={(event) => handleDrop(event, item.project_id || null, item.id)}
          onDragEnd={handleDragEnd}
          onMouseEnter={() => setHovered(item.id)}
          onMouseLeave={() => setHovered(null)}
          className={`group relative pl-3 ${draggingId === item.id ? 'opacity-45' : ''}`}
        >
          {isDropTarget(item.project_id || null, item.id) && <div className="mb-1 ml-2 h-0.5 rounded-full bg-surface-950/35" />}
          <div className="absolute left-0 top-0 h-full w-px bg-black/[0.06]" />
          <div className={`absolute left-0 top-1/2 h-px w-2 bg-black/[0.06]`} />
          <button
            type="button"
            onClick={() => item.id !== 'current' && onSelectConversation?.(item.id)}
            className={`flex w-full min-w-0 items-center gap-2 rounded-xl px-2 py-1.5 text-left transition-all ${active ? 'bg-surface-950/5 ring-1 ring-surface-950/10' : 'hover:bg-white/55'}`}
          >
            <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${toolColors[item.tool] || 'bg-surface-400'}`} />
            <span className="min-w-0 flex-1 truncate text-[11px] font-semibold leading-4 text-surface-650">
              {trimTitle(item.title)}
            </span>
            <span className="shrink-0 text-[9px] font-medium text-surface-400">{formatTime(item.updated_at) || '刚刚'}</span>
          </button>

          {item.id !== 'current' && hovered === item.id && (
            <div className="absolute right-1 top-1/2 flex -translate-y-1/2 items-center gap-1 rounded-full bg-[#f7f2e8]/95 p-0.5 shadow-sm ring-1 ring-black/[0.06]" onClick={(e) => e.stopPropagation()}>
              {onRenameConversation && (
                <button type="button" onClick={() => handleRenameConversation(item)} className="rounded-full p-1 text-surface-400 hover:bg-white hover:text-surface-800" title="修改标题">
                  <Edit3 className="h-2.5 w-2.5" />
                </button>
              )}
              {onDeleteConversation && (
                <button type="button" onClick={() => onDeleteConversation(item.id)} className="rounded-full p-1 text-red-400 hover:bg-red-50 hover:text-red-600" title="删除对话">
                  <Trash2 className="h-2.5 w-2.5" />
                </button>
              )}
            </div>
          )}
        </div>
      )
    }

    return (
      <div
        key={item.id}
        draggable={item.id !== 'current'}
        onDragStart={(event) => handleDragStart(event, item)}
        onDragOver={(event) => handleDragOver(event, item.project_id || null, item.id)}
        onDrop={(event) => handleDrop(event, item.project_id || null, item.id)}
        onDragEnd={handleDragEnd}
        onMouseEnter={() => setHovered(item.id)}
        onMouseLeave={() => setHovered(null)}
        className={`group relative overflow-hidden rounded-[1.35rem] transition-all duration-200 ${draggingId === item.id ? 'opacity-45' : ''} ${isDropTarget(item.project_id || null, item.id) ? 'ring-2 ring-surface-950/20' : ''} ${active ? 'bg-white shadow-[0_16px_38px_rgba(24,24,27,0.08)] ring-1 ring-black/[0.06]' : 'bg-white/38 hover:-translate-y-0.5 hover:bg-white/72 hover:shadow-[0_14px_30px_rgba(24,24,27,0.06)]'}`}
      >
        {active && <div className="absolute inset-y-3 left-0 w-1 rounded-r-full bg-surface-950" />}
        <button
          type="button"
          onClick={() => item.id !== 'current' && onSelectConversation?.(item.id)}
          className="flex w-full min-w-0 items-center gap-3 px-3 py-3 text-left"
        >
          <div className={`flex h-10 w-10 shrink-0 items-center justify-center rounded-2xl transition-all ${active ? 'bg-surface-950 text-white shadow-[0_10px_24px_rgba(24,24,27,0.18)]' : 'bg-white/90 text-surface-500 ring-1 ring-black/[0.06] group-hover:text-surface-800'}`}>
            <Icon className="h-[18px] w-[18px]" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-start gap-2">
              <span className="min-w-0 flex-1 truncate text-[13px] font-black leading-5 text-surface-900">{trimTitle(item.title)}</span>
              <span className={`mt-1.5 h-2 w-2 shrink-0 rounded-full shadow-[0_0_0_3px_rgba(255,255,255,0.9)] ${toolColors[item.tool] || 'bg-surface-400'}`} />
            </div>
            <div className="mt-1.5 flex items-center gap-2 text-[11px] font-medium text-surface-400">
              <span>{formatTime(item.updated_at) || '刚刚'}</span>
              <span className="h-1 w-1 rounded-full bg-surface-300" />
              <span>{item.message_count || 0} 条</span>
              <span className="ml-auto rounded-full bg-surface-100/90 px-2 py-0.5 text-[10px] font-black leading-none text-surface-500 ring-1 ring-black/[0.03]">{toolLabel[item.tool]}</span>
            </div>
          </div>
        </button>

        {item.id !== 'current' && hovered === item.id && (
          <div className="absolute right-2 top-2 flex items-center gap-1 rounded-full bg-[#f7f2e8]/95 p-0.5 shadow-sm ring-1 ring-black/[0.06]" onClick={(e) => e.stopPropagation()}>
            {onRenameConversation && (
              <button type="button" onClick={() => handleRenameConversation(item)} className="rounded-full p-1.5 text-surface-400 hover:bg-white hover:text-surface-800" title="修改标题">
                <Edit3 className="h-3 w-3" />
              </button>
            )}
            {onDeleteConversation && (
              <button type="button" onClick={() => onDeleteConversation(item.id)} className="rounded-full p-1.5 text-red-400 hover:bg-red-50 hover:text-red-600" title="删除对话">
                <Trash2 className="h-3 w-3" />
              </button>
            )}
          </div>
        )}
      </div>
    )
  }

  return (
    <aside
      className="group/sidebar relative flex h-full shrink-0 flex-col overflow-hidden border-r border-black/[0.05] bg-gradient-to-b from-[#fbf8f1]/95 via-[#f4efe6]/95 to-[#ece6db]/95 text-surface-900 shadow-[18px_0_50px_rgba(24,24,27,0.07)] backdrop-blur-2xl"
      style={{ width } as CSSProperties}
    >
      <div className="border-b border-black/[0.04] bg-[#fbf8f1]/68 px-5 pb-5 pt-6 shadow-[0_1px_0_rgba(255,255,255,0.75)_inset]">
        <div className="mb-5 flex items-center gap-3">
          <div className="flex h-10 w-10 items-center justify-center overflow-hidden rounded-2xl bg-white text-surface-900 shadow-sm ring-1 ring-black/[0.06]">
            <img src={LOGO_URL} alt="WaLiOffice logo" className="h-full w-full object-cover" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="truncate text-base font-black tracking-tight text-surface-950">WaLiOffice</div>
            <div className="mt-0.5 truncate text-[11px] font-semibold text-surface-400">办公创作空间</div>
          </div>
        </div>

        <div className="flex h-10 items-center gap-2 rounded-full border border-black/[0.04] bg-white/82 px-3.5 text-sm text-surface-500 shadow-sm backdrop-blur transition-all focus-within:bg-white focus-within:ring-2 focus-within:ring-surface-950/8">
          <Search className="h-4 w-4 shrink-0 text-surface-400" />
          <input
            value={searchQuery}
            onChange={(event) => onSearchQueryChange?.(event.target.value)}
            placeholder="搜索项目 / 对话"
            className="min-w-0 flex-1 bg-transparent text-sm font-semibold text-surface-900 placeholder:text-surface-400 outline-none"
          />
        </div>

        <button
          type="button"
          onClick={onNewConversation}
          className="mt-3 flex h-10 w-full items-center justify-center gap-2 rounded-full bg-surface-950 text-sm font-black text-white shadow-[0_10px_24px_rgba(24,24,27,0.16)] transition-all hover:-translate-y-0.5 hover:bg-surface-900 hover:shadow-[0_14px_30px_rgba(24,24,27,0.2)]"
        >
          <Plus className="h-4 w-4" />
          新建对话
        </button>
      </div>

      <div className="flex-1 overflow-y-auto px-3 py-3">
        <div className="mb-3 flex items-center justify-between px-1">
          <span className="text-[10px] font-black uppercase tracking-[0.18em] text-surface-400">项目空间</span>
          <span className="rounded-full bg-white/75 px-2 py-0.5 text-[10px] font-bold text-surface-500">{filteredProjects.length}</span>
        </div>

        <div className="space-y-2">
          {filteredProjects.map((proj) => {
            const convs = conversationsByProject.get(proj.id) || []
            const isExpanded = expandedProjects.has(proj.id)
            const isActive = activeProjectId === proj.id
            const ToolIcon = iconMap[proj.tool_kind || 'general'] || Folder
            return (
              <section
                key={proj.id}
                onDragOver={(event) => handleDragOver(event, proj.id)}
                onDrop={(event) => handleDrop(event, proj.id)}
                className={`group overflow-hidden rounded-[1.35rem] border transition-all ${isDropTarget(proj.id) ? 'border-surface-950/25 bg-white/82 ring-2 ring-surface-950/10' : isActive ? 'border-surface-950/10 bg-white/80 shadow-sm' : 'border-black/[0.04] bg-white/42 hover:bg-white/62'}`}
              >
                <div className="flex items-center gap-1 p-1.5">
                  <button
                    type="button"
                    onClick={() => toggleProject(proj.id)}
                    className="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl text-surface-500 hover:bg-white"
                    title={isExpanded ? '收起项目' : '展开项目'}
                  >
                    <ChevronDown className={`h-4 w-4 transition-transform ${isExpanded ? '' : '-rotate-90'}`} />
                  </button>
                  <button
                    type="button"
                    onClick={() => onSelectProject?.(proj.id)}
                    className="flex min-w-0 flex-1 items-center gap-2 rounded-xl px-1.5 py-1.5 text-left hover:bg-white/65"
                  >
                    <div className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-xl ${isActive ? 'bg-surface-950 text-white' : 'bg-white/85 text-surface-500 ring-1 ring-black/[0.04]'}`}>
                      <Folder className="h-4 w-4" />
                    </div>
                    <div className="min-w-0 flex-1">
                      <div className="truncate text-[13px] font-black leading-4 text-surface-900">{proj.title}</div>
                      <div className="mt-0.5 flex items-center gap-1.5 text-[10px] text-surface-400">
                        <ToolIcon className="h-3 w-3" />
                        <span>{toolLabel[proj.tool_kind || 'general']}</span>
                        <span>·</span>
                        <span>{convs.length} 个对话</span>
                      </div>
                    </div>
                  </button>
                  {onDeleteProject && (
                    <button
                      type="button"
                      onClick={(e) => { e.stopPropagation(); if (confirm(`删除项目「${proj.title}」？项目下的对话不会删除。`)) onDeleteProject(proj.id) }}
                      className="mr-1 rounded-xl p-2 text-surface-300 opacity-0 transition-all hover:bg-red-50 hover:text-red-500 group-hover:opacity-100"
                      title="删除项目"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  )}
                </div>

                {isExpanded && (
                  <div className="border-t border-black/[0.04] bg-black/[0.015] px-3 pb-2 pt-1.5">
                    {convs.length > 0 ? (
                      <div className="ml-8 space-y-0.5">{convs.map((item) => renderConversationItem(item, true))}</div>
                    ) : (
                      <div className="ml-8 rounded-xl border border-dashed border-black/[0.08] bg-white/35 px-3 py-3 text-center text-[11px] font-medium text-surface-400">
                        暂无对话
                      </div>
                    )}
                  </div>
                )}
              </section>
            )
          })}
        </div>

        {(unassignedConversations.length > 0 || draggingId) && (
          <section className="mt-4">
            <button
              type="button"
              onClick={() => setShowUnassigned(!showUnassigned)}
              className="mb-2 flex w-full items-center justify-between px-1"
            >
              <span className="text-[10px] font-black uppercase tracking-[0.18em] text-surface-400">独立对话</span>
              <span className="flex items-center gap-1.5 rounded-full bg-white/[0.58] px-2 py-0.5 text-[10px] font-bold text-surface-500 shadow-sm">
                {unassignedConversations.length}
                <ChevronDown className={`h-3.5 w-3.5 transition-transform ${showUnassigned ? '' : '-rotate-90'}`} />
              </span>
            </button>
            {(showUnassigned || draggingId) && (
              <div
                onDragOver={(event) => handleDragOver(event, null)}
                onDrop={(event) => handleDrop(event, null)}
                className={`space-y-1.5 rounded-[1.6rem] border p-1.5 shadow-[0_12px_28px_rgba(24,24,27,0.035)] backdrop-blur ${isDropTarget(null) ? 'border-surface-950/20 bg-white/55 ring-2 ring-surface-950/10' : 'border-white/55 bg-white/30'}`}
              >
                {unassignedConversations.slice(0, unassignedLimit).map((item) => renderConversationItem(item))}
                {unassignedConversations.length > unassignedLimit && (
                  <button
                    type="button"
                    onClick={() => setUnassignedLimit(unassignedLimit + 10)}
                    className="flex w-full items-center justify-center gap-1.5 rounded-2xl px-2.5 py-2 text-[11px] font-bold text-surface-500 hover:bg-white/70 hover:text-surface-800"
                  >
                    <MoreHorizontal className="h-3.5 w-3.5" />
                    查看更多（{unassignedConversations.length - unassignedLimit} 条）
                  </button>
                )}
              </div>
            )}
          </section>
        )}

        {(currentConversations.length === 0 && filteredProjects.length === 0) && (
          <div className="rounded-[1.5rem] border border-dashed border-black/10 bg-white/58 px-4 py-8 text-center text-xs font-medium leading-relaxed text-surface-500">
            {searchQuery.trim() ? '没有匹配的项目或对话。试试换个关键词。' : '暂无项目和对话。可在输入框项目下拉中新建项目。'}
          </div>
        )}
      </div>

      <div className="border-t border-black/[0.07] p-3">
        <div className="mb-2 grid grid-cols-3 gap-1.5 rounded-2xl bg-white/42 p-1 shadow-sm backdrop-blur">
          {workspaceLinks.map((item) => {
            const Icon = item.icon
            return (
              <button
                key={item.to}
                type="button"
                onClick={() => navigate(item.to)}
                className={`flex min-w-0 flex-col items-center gap-1 rounded-xl px-2 py-2 text-[10px] font-bold transition-all ${
                  item.active
                    ? 'bg-surface-950 text-white shadow-sm'
                    : 'text-surface-500 hover:bg-white/80 hover:text-surface-950'
                }`}
                title={item.label}
              >
                <Icon className="h-4 w-4" />
                <span className="truncate">{item.label}</span>
              </button>
            )
          })}
        </div>
        <div className="flex items-center gap-2 rounded-2xl bg-white/64 px-3 py-2 shadow-sm backdrop-blur">
          <div className="flex h-8 w-8 items-center justify-center rounded-full bg-surface-950 text-xs font-bold text-white">
            {userName?.[0]?.toUpperCase() || 'U'}
          </div>
          <div className="min-w-0 flex-1">
            <div className="truncate text-xs font-bold text-surface-900">{userName || 'User'}</div>
            <div className="truncate text-[10px] font-medium text-surface-500">个人办公空间</div>
          </div>
          <button
            type="button"
            onClick={onOpenSettings}
            className={`flex h-8 w-8 shrink-0 items-center justify-center rounded-xl transition-all ${activeView === 'settings' ? 'bg-surface-950 text-white shadow-sm' : 'bg-white/80 text-surface-500 hover:bg-white hover:text-surface-950'}`}
            title="设置"
            aria-label="设置"
          >
            <SettingsIcon className="h-4 w-4" />
          </button>
          <button
            type="button"
            onClick={onLogout}
            className="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl bg-red-50 text-red-500 transition-all hover:bg-red-100 hover:text-red-600"
            title="退出登录"
            aria-label="退出登录"
          >
            <LogOut className="h-4 w-4" />
          </button>
        </div>
      </div>
      <div
        role="separator"
        aria-orientation="vertical"
        title="拖动调整侧边栏宽度"
        onPointerDown={onResizeStart}
        className="absolute inset-y-0 right-0 z-20 w-2 cursor-col-resize touch-none bg-transparent transition-colors hover:bg-surface-950/10 active:bg-surface-950/15"
      >
        <div className="absolute right-0 top-1/2 h-12 w-1 -translate-y-1/2 rounded-full bg-surface-950/0 transition-colors group-hover/sidebar:bg-surface-950/12" />
      </div>
    </aside>
  )
}
