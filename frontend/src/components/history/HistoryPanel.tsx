import { Clock, Download, FileText, LayoutGrid, Paintbrush, Sparkles } from 'lucide-react'
import type { ProjectHistoryEntry } from '@/types'

interface HistoryPanelProps {
  history?: ProjectHistoryEntry[]
}

function getIcon(type: ProjectHistoryEntry['type']) {
  if (type === 'plan') return FileText
  if (type === 'draw') return Paintbrush
  if (type === 'layout') return LayoutGrid
  if (type === 'export') return Download
  return Sparkles
}

function formatTime(value: string) {
  const d = new Date(value)
  if (Number.isNaN(d.getTime())) return ''
  return d.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit', second: '2-digit' })
}

export function HistoryPanel({ history = [] }: HistoryPanelProps) {
  const items = history.slice(-12).reverse()

  return (
    <div className="border-t border-surface-200 bg-white px-4 py-3 shrink-0">
      <div className="mb-2 flex items-center justify-between">
        <div className="flex items-center gap-1.5 text-xs font-semibold text-surface-700">
          <Clock className="h-3.5 w-3.5 text-primary-500" />
          绘制历史
        </div>
        <span className="text-[11px] text-surface-400">{history.length} 条</span>
      </div>
      {items.length === 0 ? (
        <div className="rounded-xl bg-surface-50 px-3 py-2 text-xs text-surface-400">
          暂无历史。生成 PPT 后会记录大纲、绘制、重排和导出动作。
        </div>
      ) : (
        <div className="max-h-36 space-y-1.5 overflow-y-auto pr-1">
          {items.map((item) => {
            const Icon = getIcon(item.type)
            return (
              <div key={item.id} className="flex gap-2 rounded-xl bg-surface-50 px-2.5 py-2 text-xs">
                <div className="mt-0.5 flex h-5 w-5 shrink-0 items-center justify-center rounded-lg bg-white text-primary-500 shadow-sm">
                  <Icon className="h-3 w-3" />
                </div>
                <div className="min-w-0 flex-1">
                  <div className="flex items-center justify-between gap-2">
                    <span className="truncate font-medium text-surface-700">{item.title}</span>
                    <span className="shrink-0 text-[10px] text-surface-400">{formatTime(item.created_at)}</span>
                  </div>
                  {item.detail && <div className="mt-0.5 line-clamp-2 text-[11px] leading-relaxed text-surface-500">{item.detail}</div>}
                </div>
              </div>
            )
          })}
        </div>
      )}
    </div>
  )
}
