import { X, Plus, Loader2, MessageSquare } from 'lucide-react'
import { useEffect, useRef } from 'react'
import type { ToolKind } from '@/types'

export interface TabConversation {
  id: string
  title: string
  tool: ToolKind
  isStreaming: boolean
  streamPhase: 'idle' | 'thinking' | 'generating' | 'finishing' | 'done' | 'error'
  streamStatus: string
  messageCount: number
  hasArtifacts: boolean
}

interface ConversationTabsProps {
  tabs: TabConversation[]
  activeTabId: string | null
  onSelectTab: (id: string) => void
  onCloseTab: (id: string) => void
  onNewTab: () => void
  canNewTab?: boolean
}

const toolDotColors: Record<ToolKind, string> = {
  general: 'bg-sky-500',
  ppt: 'bg-blue-500',
  doc: 'bg-emerald-500',
  drawio: 'bg-violet-500',
  excel: 'bg-amber-500',
  image: 'bg-pink-500',
  video: 'bg-rose-500',
  code: 'bg-slate-500',
}

const phaseLabels: Record<string, string> = {
  idle: '',
  thinking: '思考中',
  generating: '生成中',
  finishing: '整理中',
  done: '完成',
  error: '出错',
}

export function ConversationTabs({
  tabs,
  activeTabId,
  onSelectTab,
  onCloseTab,
  onNewTab,
  canNewTab = true,
}: ConversationTabsProps) {
  const scrollRef = useRef<HTMLDivElement>(null)
  const activeRef = useRef<HTMLButtonElement>(null)

  // 活跃 tab 滚入视野
  useEffect(() => {
    if (activeRef.current && scrollRef.current) {
      const tab = activeRef.current
      const container = scrollRef.current
      const tabLeft = tab.offsetLeft
      const tabRight = tabLeft + tab.offsetWidth
      const viewLeft = container.scrollLeft
      const viewRight = viewLeft + container.offsetWidth
      if (tabLeft < viewLeft) {
        container.scrollTo({ left: tabLeft - 8, behavior: 'smooth' })
      } else if (tabRight > viewRight) {
        container.scrollTo({ left: tabRight - container.offsetWidth + 8, behavior: 'smooth' })
      }
    }
  }, [activeTabId])

  if (tabs.length === 0) return null

  return (
    <div className="relative z-10 flex h-10 shrink-0 items-center border-b border-black/[0.05] bg-[#f6f4ef]/60 backdrop-blur-xl">
      <div
        ref={scrollRef}
        className="flex h-full min-w-0 flex-1 items-center gap-0.5 overflow-x-auto px-2 scrollbar-hide"
        style={{ scrollbarWidth: 'none' }}
      >
        {tabs.map((tab) => {
          const isActive = tab.id === activeTabId
          const dotColor = toolDotColors[tab.tool] || 'bg-surface-400'
          const isWorking = tab.isStreaming && tab.streamPhase !== 'done' && tab.streamPhase !== 'error' && tab.streamPhase !== 'idle'
          const isError = tab.streamPhase === 'error'
          const isDone = tab.streamPhase === 'done' && tab.messageCount > 0

          return (
            <button
              key={tab.id}
              ref={isActive ? activeRef : undefined}
              type="button"
              onClick={() => onSelectTab(tab.id)}
              className={`group relative flex h-8 shrink-0 items-center gap-2 rounded-lg px-3 transition-all ${
                isActive
                  ? 'bg-white/90 text-surface-950 shadow-[0_2px_8px_rgba(24,24,27,0.06)] ring-1 ring-black/[0.04]'
                  : 'text-surface-500 hover:bg-white/50 hover:text-surface-700'
              }`}
              title={tab.title}
            >
              {/* 左侧状态指示 */}
              <span className="relative flex shrink-0 items-center justify-center">
                {isWorking ? (
                  <span className="relative flex h-2 w-2">
                    <span className={`absolute inline-flex h-full w-full animate-ping rounded-full ${dotColor} opacity-60`} />
                    <span className={`relative inline-flex h-2 w-2 rounded-full ${dotColor}`} />
                  </span>
                ) : isError ? (
                  <span className="h-2 w-2 rounded-full bg-red-500" />
                ) : isDone ? (
                  <span className={`h-2 w-2 rounded-full ${dotColor}`} />
                ) : (
                  <span className={`h-2 w-2 rounded-full ${isActive ? dotColor : 'bg-surface-300'}`} />
                )}
              </span>

              {/* 标题 */}
              <span className="max-w-[140px] truncate text-[12px] font-semibold leading-4">
                {tab.title}
              </span>

              {/* 进行中状态文字 */}
              {isWorking && (
                <span className="hidden items-center gap-1 text-[10px] font-medium text-surface-400 sm:inline-flex">
                  <Loader2 className="h-2.5 w-2.5 animate-spin" />
                  {phaseLabels[tab.streamPhase] || '处理中'}
                </span>
              )}

              {/* 关闭按钮 */}
              <span
                role="button"
                tabIndex={-1}
                onClick={(e) => {
                  e.stopPropagation()
                  onCloseTab(tab.id)
                }}
                className={`ml-0.5 flex h-4 w-4 shrink-0 items-center justify-center rounded-full transition-all ${
                  isActive
                    ? 'text-surface-400 hover:bg-surface-100 hover:text-surface-700'
                    : 'text-surface-300 opacity-0 hover:bg-surface-100 hover:text-surface-600 group-hover:opacity-100'
                }`}
                title="关闭对话"
              >
                <X className="h-3 w-3" />
              </span>

              {/* 活跃底线 */}
              {isActive && (
                <span className="absolute inset-x-2 -bottom-px h-0.5 rounded-full bg-surface-950/70" />
              )}
            </button>
          )
        })}
      </div>

      {/* 新建对话按钮 */}
      {canNewTab && (
        <button
          type="button"
          onClick={onNewTab}
          className="ml-1 mr-2 flex h-7 w-7 shrink-0 items-center justify-center rounded-lg text-surface-400 transition-all hover:bg-white/70 hover:text-surface-700"
          title="新建对话"
        >
          <Plus className="h-4 w-4" />
        </button>
      )}
    </div>
  )
}
