import { useState, useEffect, useCallback } from 'react'
import {
  Plus, Search, Trash2, Calendar, Flag, X, Check,
  LayoutGrid, List as ListIcon,
} from 'lucide-react'
import { taskApi } from '@/api'
import type { Task } from '@/types'

const STATUS_COLUMNS = [
  { id: 'todo', label: '待办', color: 'border-t-surface-400' },
  { id: 'in_progress', label: '进行中', color: 'border-t-blue-500' },
  { id: 'done', label: '已完成', color: 'border-t-emerald-500' },
] as const

const PRIORITY_CONFIG: Record<string, { label: string; color: string; bg: string }> = {
  urgent: { label: '紧急', color: 'text-red-600', bg: 'bg-red-50' },
  high: { label: '高', color: 'text-orange-600', bg: 'bg-orange-50' },
  medium: { label: '中', color: 'text-amber-600', bg: 'bg-amber-50' },
  low: { label: '低', color: 'text-surface-500', bg: 'bg-surface-50' },
}

export default function TasksPage() {
  const [tasks, setTasks] = useState<Task[]>([])
  const [loading, setLoading] = useState(true)
  const [view, setView] = useState<'kanban' | 'list'>('kanban')
  const [searchQuery, setSearchQuery] = useState('')
  const [showCreate, setShowCreate] = useState(false)
  const [editingTask, setEditingTask] = useState<Task | null>(null)

  const loadTasks = useCallback(async () => {
    try {
      const res = await taskApi.list({ q: searchQuery || undefined, page: 1, page_size: 100 })
      setTasks(res.data.tasks || [])
    } catch (err) {
      console.error('Load tasks error:', err)
    } finally {
      setLoading(false)
    }
  }, [searchQuery])

  useEffect(() => {
    const timer = setTimeout(loadTasks, 250)
    return () => clearTimeout(timer)
  }, [loadTasks])

  const handleCreate = async (data: Partial<Task>) => {
    try {
      await taskApi.create({
        title: data.title || '',
        description: data.description,
        priority: data.priority,
        due_date: data.due_date,
        tags: data.tags,
      })
      setShowCreate(false)
      loadTasks()
    } catch (err) {
      console.error('Create task error:', err)
    }
  }

  const handleUpdate = async (id: string, updates: Partial<Task>) => {
    try {
      await taskApi.update(id, updates)
      loadTasks()
    } catch (err) {
      console.error('Update task error:', err)
    }
  }

  const handleDelete = async (id: string) => {
    try {
      await taskApi.delete(id)
      loadTasks()
    } catch (err) {
      console.error('Delete task error:', err)
    }
  }

  const handleStatusChange = async (taskId: string, newStatus: string) => {
    const task = tasks.find(t => t.id === taskId)
    if (!task || task.status === newStatus) return
    await handleUpdate(taskId, { status: newStatus as Task['status'] })
  }

  if (loading) {
    return <div className="flex h-full items-center justify-center text-surface-400">加载中...</div>
  }

  return (
    <div className="flex h-full flex-col">
      {/* 顶部工具栏 */}
      <div className="rounded-[28px] border border-black/[0.06] bg-white/75 px-4 py-4 shadow-[0_18px_50px_rgba(24,24,27,0.06)] backdrop-blur-xl lg:px-6">
        <div className="flex items-center justify-between gap-4">
          <div>
            <h1 className="text-xl font-bold text-surface-900">任务清单</h1>
            <p className="mt-1 text-sm text-surface-500">把需要跟进的事项收在这里，和智能助手配合推进。</p>
          </div>
          <div className="flex items-center gap-2">
            {/* 搜索 */}
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-surface-400" />
              <input
                className="input h-9 w-48 pl-9 text-sm lg:w-64"
                placeholder="搜索任务..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>
            {/* 视图切换 */}
            <div className="flex items-center rounded-lg border border-surface-200 bg-white">
              <button
                onClick={() => setView('kanban')}
                className={`flex h-9 w-9 items-center justify-center rounded-l-lg ${view === 'kanban' ? 'bg-surface-900 text-white' : 'text-surface-500 hover:bg-surface-50'}`}
              >
                <LayoutGrid className="h-4 w-4" />
              </button>
              <button
                onClick={() => setView('list')}
                className={`flex h-9 w-9 items-center justify-center rounded-r-lg ${view === 'list' ? 'bg-surface-900 text-white' : 'text-surface-500 hover:bg-surface-50'}`}
              >
                <ListIcon className="h-4 w-4" />
              </button>
            </div>
            <button
              onClick={() => setShowCreate(true)}
              className="btn-primary h-9"
            >
              <Plus className="h-4 w-4" />
              新建任务
            </button>
          </div>
        </div>
      </div>

      {/* 内容区 */}
      <div className="flex-1 overflow-auto pt-4">
        {view === 'kanban' ? (
          <div className="grid grid-cols-1 gap-4 md:grid-cols-3">
            {STATUS_COLUMNS.map((col) => {
              const colTasks = tasks.filter(t => t.status === col.id)
              return (
                <div key={col.id} className={`card flex flex-col border-t-4 ${col.color}`}>
                  <div className="flex items-center justify-between px-4 py-3">
                    <h3 className="text-sm font-semibold text-surface-900">{col.label}</h3>
                    <span className="rounded-full bg-surface-100 px-2 py-0.5 text-xs text-surface-600">
                      {colTasks.length}
                    </span>
                  </div>
                  <div className="flex-1 space-y-2 px-3 pb-3">
                    {colTasks.map(task => (
                      <TaskCard
                        key={task.id}
                        task={task}
                        onClick={() => setEditingTask(task)}
                        onStatusChange={handleStatusChange}
                        onDelete={handleDelete}
                      />
                    ))}
                    {colTasks.length === 0 && (
                      <div className="rounded-lg border border-dashed border-surface-200 px-3 py-6 text-center text-xs text-surface-400">
                        拖拽或点击任务卡片调整状态
                      </div>
                    )}
                  </div>
                </div>
              )
            })}
          </div>
        ) : (
          <div className="card overflow-hidden">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-surface-200 bg-surface-50 text-left text-xs text-surface-500">
                  <th className="px-4 py-3 font-medium">任务</th>
                  <th className="px-4 py-3 font-medium">状态</th>
                  <th className="px-4 py-3 font-medium">优先级</th>
                  <th className="hidden px-4 py-3 font-medium lg:table-cell">截止日期</th>
                  <th className="px-4 py-3 font-medium">操作</th>
                </tr>
              </thead>
              <tbody>
                {tasks.map(task => (
                  <tr key={task.id} className="border-b border-surface-100 hover:bg-surface-50">
                    <td className="px-4 py-3">
                      <p className="font-medium text-surface-900">{task.title}</p>
                      {task.description && <p className="mt-0.5 text-xs text-surface-400">{task.description}</p>}
                    </td>
                    <td className="px-4 py-3">
                      <select
                        value={task.status}
                        onChange={(e) => handleStatusChange(task.id, e.target.value)}
                        className="rounded-md border border-surface-200 bg-white px-2 py-1 text-xs"
                      >
                        <option value="todo">待办</option>
                        <option value="in_progress">进行中</option>
                        <option value="done">已完成</option>
                        <option value="archived">已归档</option>
                      </select>
                    </td>
                    <td className="px-4 py-3">
                      <span className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-xs ${PRIORITY_CONFIG[task.priority]?.bg}`}>
                        <Flag className="h-3 w-3" />
                        {PRIORITY_CONFIG[task.priority]?.label}
                      </span>
                    </td>
                    <td className="hidden px-4 py-3 text-surface-500 lg:table-cell">
                      {task.due_date ? new Date(task.due_date).toLocaleDateString('zh-CN') : '-'}
                    </td>
                    <td className="px-4 py-3">
                      <div className="flex items-center gap-1">
                        <button onClick={() => setEditingTask(task)} className="text-surface-400 hover:text-surface-900">
                          <Check className="h-4 w-4" />
                        </button>
                        <button onClick={() => handleDelete(task.id)} className="text-surface-400 hover:text-red-600">
                          <Trash2 className="h-4 w-4" />
                        </button>
                      </div>
                    </td>
                  </tr>
                ))}
                {tasks.length === 0 && (
                  <tr>
                    <td colSpan={5} className="px-4 py-12 text-center text-surface-400">
                      暂无任务，点击右上角"新建任务"开始
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        )}
      </div>

      {/* 创建/编辑弹窗 */}
      {(showCreate || editingTask) && (
        <TaskDialog
          task={editingTask}
          onClose={() => { setShowCreate(false); setEditingTask(null) }}
          onSave={(data) => {
            if (editingTask) {
              handleUpdate(editingTask.id, data)
              setEditingTask(null)
            } else {
              handleCreate(data)
            }
          }}
        />
      )}
    </div>
  )
}

function TaskCard({
  task, onClick, onStatusChange, onDelete,
}: {
  task: Task
  onClick: () => void
  onStatusChange: (id: string, status: string) => void
  onDelete: (id: string) => void
}) {
  const [dragging, setDragging] = useState(false)
  const pri = PRIORITY_CONFIG[task.priority]

  return (
    <div
      draggable
      onDragStart={(e) => {
        e.dataTransfer.setData('taskId', task.id)
        setDragging(true)
      }}
      onDragEnd={() => setDragging(false)}
      onClick={onClick}
      className={`cursor-pointer rounded-lg border border-surface-200 bg-white p-3 transition-all hover:shadow-sm ${dragging ? 'opacity-50' : ''}`}
    >
      <div className="flex items-start justify-between gap-2">
        <p className="text-sm font-medium text-surface-900">{task.title}</p>
        <button
          onClick={(e) => { e.stopPropagation(); onDelete(task.id) }}
          className="text-surface-300 hover:text-red-500"
        >
          <Trash2 className="h-3.5 w-3.5" />
        </button>
      </div>
      {task.description && (
        <p className="mt-1 text-xs text-surface-400 line-clamp-2">{task.description}</p>
      )}
      <div className="mt-2 flex items-center gap-2">
        <span className={`inline-flex items-center gap-1 rounded-full px-2 py-0.5 text-[10px] ${pri?.bg} ${pri?.color}`}>
          <Flag className="h-2.5 w-2.5" />
          {pri?.label}
        </span>
        {task.due_date && (
          <span className="inline-flex items-center gap-1 text-[10px] text-surface-400">
            <Calendar className="h-2.5 w-2.5" />
            {new Date(task.due_date).toLocaleDateString('zh-CN')}
          </span>
        )}
        {task.tags && task.tags.length > 0 && (
          <div className="flex gap-1">
            {task.tags.slice(0, 2).map(tag => (
              <span key={tag} className="rounded bg-surface-100 px-1.5 py-0.5 text-[10px] text-surface-500">
                {tag}
              </span>
            ))}
          </div>
        )}
      </div>
      {/* 快速状态切换 */}
      <div className="mt-2 flex gap-1 border-t border-surface-100 pt-2">
        {STATUS_COLUMNS.map(col => (
          <button
            key={col.id}
            onClick={(e) => { e.stopPropagation(); onStatusChange(task.id, col.id) }}
            className={`flex-1 rounded px-1 py-0.5 text-[10px] ${
              task.status === col.id ? 'bg-surface-900 text-white' : 'text-surface-400 hover:bg-surface-100'
            }`}
          >
            {col.label}
          </button>
        ))}
      </div>
    </div>
  )
}

function TaskDialog({
  task, onClose, onSave,
}: {
  task: Task | null
  onClose: () => void
  onSave: (data: Partial<Task>) => void
}) {
  const [title, setTitle] = useState(task?.title || '')
  const [description, setDescription] = useState(task?.description || '')
  const [priority, setPriority] = useState<Task['priority']>(task?.priority || 'medium')
  const [dueDate, setDueDate] = useState(task?.due_date?.split('T')[0] || '')
  const [tags, setTags] = useState((task?.tags || []).join(', '))

  const handleSave = () => {
    if (!title.trim()) return
    onSave({
      title: title.trim(),
      description: description.trim(),
      priority,
      due_date: dueDate ? new Date(dueDate).toISOString() : undefined,
      tags: tags.split(',').map(t => t.trim()).filter(Boolean),
    })
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4" onClick={onClose}>
      <div
        className="w-full max-w-md rounded-2xl bg-white p-6 shadow-xl"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="mb-4 flex items-center justify-between">
          <h3 className="text-lg font-semibold text-surface-900">
            {task ? '编辑任务' : '新建任务'}
          </h3>
          <button onClick={onClose} className="text-surface-400 hover:text-surface-900">
            <X className="h-5 w-5" />
          </button>
        </div>

        <div className="space-y-4">
          <div>
            <label className="mb-1 block text-sm font-medium text-surface-700">标题</label>
            <input
              className="input"
              placeholder="任务标题"
              value={title}
              onChange={(e) => setTitle(e.target.value)}
              autoFocus
            />
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-surface-700">描述</label>
            <textarea
              className="input min-h-[80px] resize-none"
              placeholder="任务描述..."
              value={description}
              onChange={(e) => setDescription(e.target.value)}
            />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className="mb-1 block text-sm font-medium text-surface-700">优先级</label>
              <select
                className="input"
                value={priority}
                onChange={(e) => setPriority(e.target.value as Task['priority'])}
              >
                <option value="low">低</option>
                <option value="medium">中</option>
                <option value="high">高</option>
                <option value="urgent">紧急</option>
              </select>
            </div>
            <div>
              <label className="mb-1 block text-sm font-medium text-surface-700">截止日期</label>
              <input
                type="date"
                className="input"
                value={dueDate}
                onChange={(e) => setDueDate(e.target.value)}
              />
            </div>
          </div>
          <div>
            <label className="mb-1 block text-sm font-medium text-surface-700">标签（逗号分隔）</label>
            <input
              className="input"
              placeholder="例如：重要, 前端, v2.0"
              value={tags}
              onChange={(e) => setTags(e.target.value)}
            />
          </div>
        </div>

        <div className="mt-6 flex justify-end gap-2">
          <button onClick={onClose} className="btn-secondary">取消</button>
          <button onClick={handleSave} className="btn-primary" disabled={!title.trim()}>
            {task ? '保存' : '创建'}
          </button>
        </div>
      </div>
    </div>
  )
}
