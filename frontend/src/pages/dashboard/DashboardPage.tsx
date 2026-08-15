import { useState, useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import {
  CheckSquare, FolderOpen, FileText, Bell, Clock,
  TrendingUp, ArrowRight, AlertCircle, Plus,
} from 'lucide-react'
import { dashboardApi, taskApi } from '@/api'
import type { DashboardStats } from '@/types'

const TOOL_LABELS: Record<string, string> = {
  general: '综合',
  ppt: 'PPT',
  doc: '文档',
  drawio: '绘图',
  excel: '表格',
  image: '图像',
  code: '代码',
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`
}

function timeAgo(dateStr: string): string {
  const now = Date.now()
  const diff = now - new Date(dateStr).getTime()
  const minutes = Math.floor(diff / 60000)
  if (minutes < 1) return '刚刚'
  if (minutes < 60) return `${minutes} 分钟前`
  const hours = Math.floor(minutes / 60)
  if (hours < 24) return `${hours} 小时前`
  const days = Math.floor(hours / 24)
  return `${days} 天前`
}

export default function DashboardPage() {
  const navigate = useNavigate()
  const [stats, setStats] = useState<DashboardStats | null>(null)
  const [loading, setLoading] = useState(true)

  useEffect(() => {
    const load = async () => {
      try {
        const res = await dashboardApi.stats()
        setStats(res.data)
      } catch (err) {
        console.error('Load dashboard error:', err)
      } finally {
        setLoading(false)
      }
    }
    load()
  }, [])

  const taskStats = stats?.tasks ?? {
    total: 0,
    by_status: {},
    by_priority: {},
    due_soon: 0,
  }
  const projectStats = stats?.projects ?? {
    total: 0,
    by_kind: {},
  }
  const fileStats = stats?.files ?? {
    total: 0,
    by_type: {},
    total_size: 0,
  }
  const notificationStats = stats?.notifications ?? {
    unread: 0,
  }
  const recentSessions = stats?.recent_sessions ?? []

  if (loading) {
    return (
      <div className="flex h-full items-center justify-center">
        <div className="text-surface-400">加载中...</div>
      </div>
    )
  }

  const cards = [
    {
      label: '任务总数',
      value: taskStats.total || 0,
      sub: `${taskStats.by_status?.done || 0} 已完成`,
      icon: CheckSquare,
      color: 'from-blue-500 to-blue-600',
      onClick: () => navigate('/tasks'),
    },
    {
      label: '项目数量',
      value: projectStats.total || 0,
      sub: `${projectStats.by_kind?.ppt || 0} PPT · ${projectStats.by_kind?.doc || 0} 文档`,
      icon: FileText,
      color: 'from-emerald-500 to-emerald-600',
      onClick: () => navigate('/'),
    },
    {
      label: '文件数量',
      value: fileStats.total || 0,
      sub: formatSize(fileStats.total_size || 0),
      icon: FolderOpen,
      color: 'from-amber-500 to-amber-600',
      onClick: () => navigate('/files'),
    },
    {
      label: '未读通知',
      value: notificationStats.unread || 0,
      sub: '待查看',
      icon: Bell,
      color: 'from-violet-500 to-violet-600',
      onClick: () => navigate('/dashboard'),
    },
  ]

  const statusLabels: Record<string, string> = {
    todo: '待办',
    in_progress: '进行中',
    done: '已完成',
    archived: '已归档',
  }

  return (
    <div className="mx-auto max-w-7xl p-4 lg:p-8">
      {/* 标题 */}
      <div className="mb-8">
        <h1 className="text-2xl font-bold text-surface-900">工作台</h1>
        <p className="mt-1 text-sm text-surface-500">欢迎回来，这里是您的工作概览</p>
      </div>

      {/* 统计卡片 */}
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-4">
        {cards.map((card) => {
          const Icon = card.icon
          return (
            <button
              key={card.label}
              onClick={card.onClick}
              className="card group relative overflow-hidden p-5 text-left transition-all hover:shadow-md"
            >
              <div className="flex items-start justify-between">
                <div>
                  <p className="text-sm text-surface-500">{card.label}</p>
                  <p className="mt-2 text-3xl font-bold text-surface-900">{card.value}</p>
                  <p className="mt-1 text-xs text-surface-400">{card.sub}</p>
                </div>
                <div className={`flex h-11 w-11 items-center justify-center rounded-xl bg-gradient-to-br ${card.color} shadow-sm`}>
                  <Icon className="h-5 w-5 text-white" />
                </div>
              </div>
              <ArrowRight className="absolute bottom-4 right-4 h-4 w-4 text-surface-300 opacity-0 transition-opacity group-hover:opacity-100" />
            </button>
          )
        })}
      </div>

      <div className="mt-6 grid grid-cols-1 gap-6 lg:grid-cols-2">
        {/* 任务概览 */}
        <div className="card p-6">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="flex items-center gap-2 text-base font-semibold text-surface-900">
              <CheckSquare className="h-4 w-4 text-surface-400" />
              任务概览
            </h2>
            <button
              onClick={() => navigate('/tasks')}
              className="text-xs text-primary-600 hover:text-primary-700"
            >
              查看全部 →
            </button>
          </div>

          {taskStats.due_soon ? (
            <div className="mb-4 flex items-center gap-2 rounded-lg bg-amber-50 px-3 py-2 text-sm text-amber-700">
              <AlertCircle className="h-4 w-4 shrink-0" />
              <span>{taskStats.due_soon} 个任务即将到期（3 天内）</span>
            </div>
          ) : null}

          <div className="space-y-2.5">
            {Object.entries(statusLabels).map(([status, label]) => {
              const count = taskStats.by_status?.[status] || 0
              const total = taskStats.total || 0
              const pct = total > 0 ? (count / total) * 100 : 0
              const colors: Record<string, string> = {
                todo: 'bg-surface-300',
                in_progress: 'bg-blue-500',
                done: 'bg-emerald-500',
                archived: 'bg-surface-200',
              }
              return (
                <div key={status}>
                  <div className="mb-1 flex items-center justify-between text-sm">
                    <span className="text-surface-600">{label}</span>
                    <span className="font-medium text-surface-900">{count}</span>
                  </div>
                  <div className="h-1.5 overflow-hidden rounded-full bg-surface-100">
                    <div className={`h-full rounded-full ${colors[status]}`} style={{ width: `${pct}%` }} />
                  </div>
                </div>
              )
            })}
          </div>
        </div>

        {/* 最近会话 */}
        <div className="card p-6">
          <div className="mb-4 flex items-center justify-between">
            <h2 className="flex items-center gap-2 text-base font-semibold text-surface-900">
              <Clock className="h-4 w-4 text-surface-400" />
              最近对话
            </h2>
            <button
              onClick={() => navigate('/')}
              className="text-xs text-primary-600 hover:text-primary-700"
            >
              开始对话 →
            </button>
          </div>

          {recentSessions.length > 0 ? (
            <div className="space-y-2">
              {recentSessions.slice(0, 5).map((s: any) => (
                <button
                  key={s.id}
                  onClick={() => navigate('/')}
                  className="flex w-full items-center gap-3 rounded-lg px-3 py-2 text-left transition-colors hover:bg-surface-50"
                >
                  <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg bg-surface-100 text-xs font-medium text-surface-600">
                    {TOOL_LABELS[s.tool_kind] || '综'}
                  </div>
                  <div className="min-w-0 flex-1">
                    <p className="truncate text-sm font-medium text-surface-900">{s.title}</p>
                    <p className="truncate text-xs text-surface-400">
                      {s.summary || `${s.message_count} 条消息`} · {timeAgo(s.updated_at)}
                    </p>
                  </div>
                </button>
              ))}
            </div>
          ) : (
            <div className="flex flex-col items-center justify-center py-8 text-center">
              <p className="text-sm text-surface-400">还没有对话记录</p>
              <button
                onClick={() => navigate('/')}
                className="mt-3 inline-flex items-center gap-1.5 text-sm text-primary-600 hover:text-primary-700"
              >
                <Plus className="h-4 w-4" />
                开始第一次对话
              </button>
            </div>
          )}
        </div>
      </div>

      {/* 项目分布 */}
      <div className="mt-6 card p-6">
        <h2 className="mb-4 flex items-center gap-2 text-base font-semibold text-surface-900">
          <TrendingUp className="h-4 w-4 text-surface-400" />
          项目分布
        </h2>
        <div className="flex flex-wrap gap-3">
          {Object.entries(stats?.projects.by_kind || {}).map(([kind, count]) => (
            <div key={kind} className="flex items-center gap-2 rounded-lg border border-surface-200 px-3 py-2">
              <span className="text-sm font-medium text-surface-900">{TOOL_LABELS[kind] || kind}</span>
              <span className="rounded-full bg-surface-100 px-2 py-0.5 text-xs text-surface-600">{count}</span>
            </div>
          ))}
          {Object.keys(stats?.projects.by_kind || {}).length === 0 && (
            <p className="text-sm text-surface-400">暂无项目</p>
          )}
        </div>
      </div>
    </div>
  )
}
