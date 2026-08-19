import { useEffect, useRef, useState, useMemo, useCallback } from 'react'
import { FileEdit, Sheet, PenTool, Image as ImageIcon, Clapperboard, FileText, Search, Clock, MessageSquare } from 'lucide-react'
import type { Artifact } from '@/types'

export interface FilePickerProps {
  /** 当前会话产物 */
  artifacts: Artifact[]
  /** 历史会话产物（从其他会话加载的） */
  historyArtifacts?: HistoryArtifactItem[]
  /** 选择回调 */
  onSelect: (artifact: Artifact, sessionId?: string) => void
  onClose: () => void
}

export interface HistoryArtifactItem {
  artifact: Artifact
  sessionTitle: string
  sessionId: string
}

const KIND_ICON: Record<string, typeof FileEdit> = {
  document: FileText,
  markdown: FileText,
  ppt: FileEdit,
  sheet: Sheet,
  drawio: PenTool,
  image: ImageIcon,
  video: Clapperboard,
  chart: Sheet,
  code: FileEdit,
  mixed: FileEdit,
  search: FileEdit,
}

const KIND_LABEL: Record<string, string> = {
  document: '文档',
  markdown: 'MD',
  ppt: 'PPT',
  sheet: '表格',
  drawio: '图表',
  image: '图片',
  video: '视频',
  chart: '图表',
  code: '代码',
  mixed: '混合',
  search: '搜索',
}

type PickerTab = 'current' | 'history'

export function FilePickerPanel({ artifacts, historyArtifacts, onSelect, onClose }: FilePickerProps) {
  const [query, setQuery] = useState('')
  const [selectedIndex, setSelectedIndex] = useState(0)
  const [activeTab, setActiveTab] = useState<PickerTab>('current')
  const inputRef = useRef<HTMLInputElement>(null)
  const listRef = useRef<HTMLDivElement>(null)

  // 当前会话没有产物但历史有 → 自动切到历史 tab
  useEffect(() => {
    if (artifacts.length === 0 && historyArtifacts && historyArtifacts.length > 0) {
      setActiveTab('history')
    }
  }, [artifacts.length, historyArtifacts])

  const currentList = useMemo(() => artifacts, [artifacts])

  const historyList = useMemo(() => historyArtifacts || [], [historyArtifacts])

  // 按类型分组 - 当前列表
  const currentGrouped = useMemo(() => {
    const filtered = currentList.filter((a) => {
      if (!query.trim()) return true
      const q = query.toLowerCase()
      return a.title?.toLowerCase().includes(q) || a.kind?.toLowerCase().includes(q)
    })
    const groups: Record<string, Artifact[]> = {}
    for (const a of filtered) {
      const key = a.kind || 'mixed'
      if (!groups[key]) groups[key] = []
      groups[key].push(a)
    }
    return groups
  }, [currentList, query])

  // 历史列表分组
  const historyGrouped = useMemo(() => {
    const filtered = historyList.filter((item) => {
      if (!query.trim()) return true
      const q = query.toLowerCase()
      return item.artifact.title?.toLowerCase().includes(q) || item.artifact.kind?.toLowerCase().includes(q) || item.sessionTitle?.toLowerCase().includes(q)
    })
    // 按 sessionTitle 分组
    const groups: Record<string, HistoryArtifactItem[]> = {}
    for (const item of filtered) {
      const key = item.sessionTitle || '未命名会话'
      if (!groups[key]) groups[key] = []
      groups[key].push(item)
    }
    return groups
  }, [historyList, query])

  // 扁平化列表用于键盘导航
  const flatList = useMemo(() => {
    if (activeTab === 'current') {
      const arr: { artifact: Artifact; sessionId?: string }[] = []
      for (const kind of Object.keys(currentGrouped).sort()) {
        for (const a of currentGrouped[kind]) {
          arr.push({ artifact: a })
        }
      }
      return arr
    } else {
      const arr: { artifact: Artifact; sessionId?: string }[] = []
      for (const sessionTitle of Object.keys(historyGrouped).sort()) {
        for (const item of historyGrouped[sessionTitle]) {
          arr.push({ artifact: item.artifact, sessionId: item.sessionId })
        }
      }
      return arr
    }
  }, [activeTab, currentGrouped, historyGrouped])

  useEffect(() => {
    inputRef.current?.focus()
  }, [])

  useEffect(() => {
    setSelectedIndex(0)
  }, [query, activeTab])

  // 滚动到选中项
  useEffect(() => {
    const el = listRef.current?.querySelector(`[data-idx="${selectedIndex}"]`)
    el?.scrollIntoView({ block: 'nearest' })
  }, [selectedIndex])

  const handleKeyDown = (e: React.KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault()
      onClose()
    } else if (e.key === 'ArrowDown') {
      e.preventDefault()
      setSelectedIndex((i) => Math.min(i + 1, flatList.length - 1))
    } else if (e.key === 'ArrowUp') {
      e.preventDefault()
      setSelectedIndex((i) => Math.max(i - 1, 0))
    } else if (e.key === 'Enter') {
      e.preventDefault()
      const item = flatList[selectedIndex]
      if (item) onSelect(item.artifact, item.sessionId)
    } else if (e.key === 'Tab') {
      e.preventDefault()
      // Tab 切换 current/history
      if (activeTab === 'current' && historyList.length > 0) {
        setActiveTab('history')
      } else if (activeTab === 'history' && currentList.length > 0) {
        setActiveTab('current')
      }
    }
  }

  const handleItemClick = useCallback((artifact: Artifact, sessionId?: string) => {
    onSelect(artifact, sessionId)
  }, [onSelect])

  const hasCurrent = currentList.length > 0
  const hasHistory = historyList.length > 0
  const isEmpty = flatList.length === 0

  if (isEmpty) {
    return (
      <div className="absolute bottom-full left-0 right-0 z-50 mb-2 overflow-hidden rounded-2xl border border-surface-200 bg-white shadow-[0_8px_30px_rgba(15,23,42,0.12)]">
        <div className="border-b border-surface-100 px-3 py-2">
          <div className="flex items-center gap-2 text-xs text-surface-400">
            <Search className="h-3.5 w-3.5" />
            <span>没有可引用的产物</span>
          </div>
        </div>
        <div className="px-3 py-6 text-center text-xs text-surface-300">
          当前还没有任何可引用的产物
        </div>
      </div>
    )
  }

  let runningIdx = -1

  return (
    <div className="absolute bottom-full left-0 right-0 z-50 mb-2 flex max-h-[360px] flex-col overflow-hidden rounded-2xl border border-surface-200 bg-white shadow-[0_8px_30px_rgba(15,23,42,0.12)]">
      {/* 搜索栏 + Tab 切换 */}
      <div className="border-b border-surface-100 px-3 py-2">
        <div className="flex items-center gap-2">
          <Search className="h-3.5 w-3.5 shrink-0 text-surface-400" />
          <input
            ref={inputRef}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            onKeyDown={handleKeyDown}
            placeholder="搜索产物…"
            className="w-full border-0 bg-transparent p-0 text-xs text-surface-800 outline-none placeholder:text-surface-300"
          />
          <span className="shrink-0 rounded bg-surface-50 px-1.5 py-0.5 text-[10px] font-medium text-surface-400">
            ESC 关闭
          </span>
        </div>
        {/* Tab 切换 */}
        {(hasCurrent && hasHistory) && (
          <div className="mt-2 flex gap-1">
            <button
              type="button"
              onClick={() => setActiveTab('current')}
              className={`inline-flex items-center gap-1 rounded-md px-2 py-1 text-[10px] font-medium transition ${
                activeTab === 'current' ? 'bg-primary-50 text-primary-700' : 'text-surface-400 hover:bg-surface-50'
              }`}
            >
              <MessageSquare className="h-3 w-3" />
              当前会话（{currentList.length}）
            </button>
            <button
              type="button"
              onClick={() => setActiveTab('history')}
              className={`inline-flex items-center gap-1 rounded-md px-2 py-1 text-[10px] font-medium transition ${
                activeTab === 'history' ? 'bg-primary-50 text-primary-700' : 'text-surface-400 hover:bg-surface-50'
              }`}
            >
              <Clock className="h-3 w-3" />
              历史会话（{historyList.length}）
            </button>
          </div>
        )}
      </div>

      {/* 列表 */}
      <div ref={listRef} className="flex-1 overflow-y-auto py-1">
        {activeTab === 'current' ? (
          // 当前会话产物 - 按类型分组
          Object.keys(currentGrouped).sort().map((kind) => (
            <div key={kind}>
              <div className="px-3 py-1 text-[10px] font-semibold uppercase tracking-wide text-surface-300">
                {KIND_LABEL[kind] || kind}（{currentGrouped[kind].length}）
              </div>
              {currentGrouped[kind].map((artifact) => {
                runningIdx++
                const idx = runningIdx
                const Icon = KIND_ICON[kind] || FileEdit
                const isSelected = idx === selectedIndex
                return (
                  <div
                    key={artifact.id}
                    data-idx={idx}
                    onMouseEnter={() => setSelectedIndex(idx)}
                    onClick={() => handleItemClick(artifact)}
                    className={`flex cursor-pointer items-center gap-2.5 px-3 py-2 text-xs transition-colors ${
                      isSelected ? 'bg-primary-50 text-primary-700' : 'text-surface-700 hover:bg-surface-50'
                    }`}
                  >
                    <Icon className={`h-3.5 w-3.5 shrink-0 ${isSelected ? 'text-primary-500' : 'text-surface-400'}`} />
                    <span className="truncate font-medium">{artifact.title || '未命名'}</span>
                    <span className="ml-auto shrink-0 text-[10px] text-surface-300">
                      {KIND_LABEL[kind]}
                    </span>
                  </div>
                )
              })}
            </div>
          ))
        ) : (
          // 历史会话产物 - 按会话分组
          Object.keys(historyGrouped).sort().map((sessionTitle) => (
            <div key={sessionTitle}>
              <div className="px-3 py-1 text-[10px] font-semibold uppercase tracking-wide text-surface-300">
                {sessionTitle}（{historyGrouped[sessionTitle].length}）
              </div>
              {historyGrouped[sessionTitle].map((item) => {
                runningIdx++
                const idx = runningIdx
                const kind = item.artifact.kind || 'mixed'
                const Icon = KIND_ICON[kind] || FileEdit
                const isSelected = idx === selectedIndex
                return (
                  <div
                    key={item.artifact.id}
                    data-idx={idx}
                    onMouseEnter={() => setSelectedIndex(idx)}
                    onClick={() => handleItemClick(item.artifact, item.sessionId)}
                    className={`flex cursor-pointer items-center gap-2.5 px-3 py-2 text-xs transition-colors ${
                      isSelected ? 'bg-primary-50 text-primary-700' : 'text-surface-700 hover:bg-surface-50'
                    }`}
                  >
                    <Icon className={`h-3.5 w-3.5 shrink-0 ${isSelected ? 'text-primary-500' : 'text-surface-400'}`} />
                    <span className="truncate font-medium">{item.artifact.title || '未命名'}</span>
                    <span className="ml-auto shrink-0 text-[10px] text-surface-300">
                      {KIND_LABEL[kind]}
                    </span>
                  </div>
                )
              })}
            </div>
          ))
        )}
      </div>

      {/* 底部提示 */}
      <div className="border-t border-surface-100 px-3 py-1.5 text-[10px] text-surface-300">
        <kbd className="font-sans">↑↓</kbd> 选择 · <kbd className="font-sans">Enter</kbd> 引入 · <kbd className="font-sans">Tab</kbd> 切换 · <kbd className="font-sans">ESC</kbd> 关闭
      </div>
    </div>
  )
}
