import { Bot, BrainCircuit, ChevronDown, Clapperboard, Eraser, FileText, Folder, FolderPlus, Image, LayoutDashboard, MessageSquare, MoreHorizontal, PenTool, Plus, Search, Settings as SettingsIcon, Sheet, Sparkles, Trash2 } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import type { ChatMessage, ConversationRecord, PPTProject, ToolKind, ProjectMeta } from '@/types'

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
  onNewProject?: () => void
  onNewConversation?: () => void
  onSelectConversation?: (id: string) => void
  onSelectProject?: (projectId: string) => void
  onClearConversation?: (id: string) => void
  onDeleteConversation?: (id: string) => void
  onDeleteProject?: (projectId: string) => void
  onOpenSettings?: () => void
  searchQuery?: string
  onSearchQueryChange?: (query: string) => void
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
  onNewProject,
  onNewConversation,
  onSelectConversation,
  onSelectProject,
  onClearConversation,
  onDeleteConversation,
  onDeleteProject,
  onOpenSettings,
  searchQuery = '',
  onSearchQueryChange,
}: ConversationSidebarProps) {
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set())
  const [showUnassigned, setShowUnassigned] = useState(true)
  const [hovered, setHovered] = useState<string | null>(null)
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
      byProject.set(proj.id, filteredConversations.filter((c) => c.project_id === proj.id))
    }
    return {
      filteredProjects,
      unassignedConversations: filteredConversations.filter((c) => !c.project_id),
      conversationsByProject: byProject,
    }
  }, [projects, currentConversations, searchQuery])

  const toggleProject = (id: string) => {
    setExpandedProjects((prev) => {
      const next = new Set(prev)
      if (next.has(id)) next.delete(id)
      else next.add(id)
      return next
    })
  }

  const renderConversationItem = (item: ConversationRecord, child = false) => {
    const active = activeConversationId ? item.id === activeConversationId : item.id === 'current'
    const Icon = iconMap[item.tool] || MessageSquare

    if (child) {
      return (
        <div
          key={item.id}
          onMouseEnter={() => setHovered(item.id)}
          onMouseLeave={() => setHovered(null)}
          className="group relative pl-3"
        >
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
              {onClearConversation && (
                <button type="button" onClick={() => onClearConversation(item.id)} className="rounded-full p-1 text-surface-400 hover:bg-white hover:text-surface-800" title="清空消息">
                  <Eraser className="h-2.5 w-2.5" />
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
        onMouseEnter={() => setHovered(item.id)}
        onMouseLeave={() => setHovered(null)}
        className={`group relative rounded-2xl transition-all ${active ? 'bg-white shadow-sm ring-1 ring-black/[0.06]' : 'hover:bg-white/65'}`}
      >
        <button
          type="button"
          onClick={() => item.id !== 'current' && onSelectConversation?.(item.id)}
          className="flex w-full min-w-0 items-center gap-2 px-2.5 py-2 text-left"
        >
          <div className={`flex h-7 w-7 shrink-0 items-center justify-center rounded-xl ${active ? 'bg-surface-950 text-white' : 'bg-white/80 text-surface-500 ring-1 ring-black/[0.04]'}`}>
            <Icon className="h-3.5 w-3.5" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="flex items-center gap-1.5">
              <span className="truncate text-[12px] font-semibold leading-4 text-surface-800">{trimTitle(item.title)}</span>
              <span className={`h-1.5 w-1.5 shrink-0 rounded-full ${toolColors[item.tool] || 'bg-surface-400'}`} />
            </div>
            <div className="mt-0.5 flex items-center gap-1.5 text-[10px] text-surface-400">
              <span>{formatTime(item.updated_at) || '刚刚'}</span>
              <span>·</span>
              <span>{item.message_count || 0} 条</span>
              <span className="ml-auto rounded-md bg-surface-100/80 px-1 py-px text-[8px] font-bold leading-none text-surface-500">{toolLabel[item.tool]}</span>
            </div>
          </div>
        </button>

        {item.id !== 'current' && hovered === item.id && (
          <div className="absolute right-2 top-2 flex items-center gap-1 rounded-full bg-[#f7f2e8]/95 p-0.5 shadow-sm ring-1 ring-black/[0.06]" onClick={(e) => e.stopPropagation()}>
            {onClearConversation && (
              <button type="button" onClick={() => onClearConversation(item.id)} className="rounded-full p-1.5 text-surface-400 hover:bg-white hover:text-surface-800" title="清空消息">
                <Eraser className="h-3 w-3" />
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
    <aside className="flex h-full w-80 shrink-0 flex-col overflow-hidden border-r border-black/[0.07] bg-[#eee9df]/96 text-surface-900 shadow-[12px_0_40px_rgba(24,24,27,0.04)] backdrop-blur-2xl">
      <div className="border-b border-black/[0.06] p-4">
        <div className="mb-4 flex items-center gap-3 px-0.5">
          <div className="flex h-10 w-10 items-center justify-center rounded-[1.25rem] bg-surface-950 shadow-sm">
            <Sparkles className="h-5 w-5 text-white" />
          </div>
          <div className="min-w-0 flex-1">
            <div className="truncate text-sm font-black tracking-tight text-surface-950">WaLiOffice</div>
            <div className="truncate text-[11px] font-medium text-surface-500">项目制智能体工作台</div>
          </div>
        </div>

        <div className="mb-3 grid grid-cols-[1fr_auto] gap-2">
          <button
            type="button"
            onClick={onNewProject}
            className="flex items-center justify-center gap-1.5 rounded-2xl bg-surface-950 px-3 py-2.5 text-sm font-bold text-white shadow-[0_14px_32px_rgba(24,24,27,0.18)] transition-all hover:-translate-y-0.5 hover:bg-surface-800"
          >
            <FolderPlus className="h-4 w-4" />
            新建项目
          </button>
          <button
            type="button"
            onClick={onNewConversation}
            className="flex h-10 w-10 items-center justify-center rounded-2xl border border-black/[0.07] bg-white/75 text-surface-700 shadow-sm transition-all hover:bg-white"
            title="新建对话"
          >
            <Plus className="h-4 w-4" />
          </button>
        </div>

        <div className="flex items-center gap-2 rounded-2xl border border-black/[0.07] bg-white/82 px-3 py-2 text-xs text-surface-500 shadow-sm backdrop-blur">
          <Search className="h-3.5 w-3.5" />
          <input
            value={searchQuery}
            onChange={(event) => onSearchQueryChange?.(event.target.value)}
            placeholder="搜索项目 / 对话"
            className="min-w-0 flex-1 bg-transparent text-xs font-medium text-surface-900 placeholder:text-surface-400 outline-none"
          />
        </div>
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
              <section key={proj.id} className={`group overflow-hidden rounded-[1.35rem] border transition-all ${isActive ? 'border-surface-950/10 bg-white/80 shadow-sm' : 'border-black/[0.04] bg-white/42 hover:bg-white/62'}`}>
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

        {unassignedConversations.length > 0 && (
          <section className="mt-4">
            <button
              type="button"
              onClick={() => setShowUnassigned(!showUnassigned)}
              className="mb-2 flex w-full items-center justify-between px-1"
            >
              <span className="text-[10px] font-black uppercase tracking-[0.18em] text-surface-400">独立对话</span>
              <span className="flex items-center gap-1.5 text-[10px] font-bold text-surface-500">
                {unassignedConversations.length}
                <ChevronDown className={`h-3.5 w-3.5 transition-transform ${showUnassigned ? '' : '-rotate-90'}`} />
              </span>
            </button>
            {showUnassigned && (
              <div className="space-y-1 rounded-[1.35rem] bg-white/35 p-1.5">
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
            {searchQuery.trim() ? '没有匹配的项目或对话。试试换个关键词。' : '暂无项目和对话。点击「新建项目」开始。'}
          </div>
        )}
      </div>

      <div className="border-t border-black/[0.07] p-3">
        <div className="flex items-center gap-2 rounded-2xl bg-white/64 px-3 py-2 shadow-sm backdrop-blur">
          <div className="flex h-8 w-8 items-center justify-center rounded-full bg-surface-950 text-xs font-bold text-white">
            {userName?.[0]?.toUpperCase() || 'U'}
          </div>
          <div className="min-w-0 flex-1">
            <div className="truncate text-xs font-bold text-surface-900">{userName || 'User'}</div>
            <div className="truncate text-[10px] font-medium text-surface-500">本地智能体工作台</div>
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
        </div>
      </div>
    </aside>
  )
}
