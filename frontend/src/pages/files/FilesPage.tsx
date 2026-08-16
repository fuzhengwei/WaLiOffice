import { useState, useEffect, useCallback, useRef } from 'react'
import type React from 'react'
import {
  Upload, Search, Download, Trash2, Folder as FolderIcon,
  FileText, Image, FileSpreadsheet, FileType, FileCode,
  File as FileIcon, FolderPlus, ChevronRight, HardDrive, Sparkles,
  LayoutGrid, List, Eye, Play,
} from 'lucide-react'
import { fileApi, folderApi } from '@/api'
import type { FileItem, Folder } from '@/types'
import { FileThumbnail } from './FileThumbnail'
import { FilePreviewModal } from './FilePreviewModal'

const FILE_ICONS: Record<string, typeof FileIcon> = {
  ppt: FileType,
  doc: FileText,
  document: FileText,
  excel: FileSpreadsheet,
  sheet: FileSpreadsheet,
  image: Image,
  video: Play,
  drawio: FileCode,
  code: FileCode,
  other: FileIcon,
}

const FILE_COLORS: Record<string, string> = {
  ppt: 'text-orange-500 bg-orange-50',
  doc: 'text-blue-500 bg-blue-50',
  document: 'text-blue-500 bg-blue-50',
  excel: 'text-emerald-500 bg-emerald-50',
  sheet: 'text-emerald-500 bg-emerald-50',
  image: 'text-violet-500 bg-violet-50',
  video: 'text-rose-500 bg-rose-50',
  drawio: 'text-amber-500 bg-amber-50',
  code: 'text-slate-500 bg-slate-50',
  other: 'text-surface-400 bg-surface-50',
}

const FILTERS = [
  { id: 'all', label: '全部文件' },
  { id: 'generated', label: '生成产物' },
  { id: 'uploaded', label: '上传文件' },
] as const

type FileFilter = typeof FILTERS[number]['id']
type ViewMode = 'grid' | 'list'

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.min(units.length - 1, Math.floor(Math.log(bytes) / Math.log(1024)))
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`
}

function formatDate(value?: string) {
  if (!value) return '刚刚'
  const date = new Date(value)
  if (Number.isNaN(date.getTime())) return '刚刚'
  return date.toLocaleDateString('zh-CN', { month: '2-digit', day: '2-digit' })
}

function isGeneratedFile(file: FileItem) {
  return (file.description || '').includes('智能助手生成')
}

function fileSourceLabel(file: FileItem) {
  if (isGeneratedFile(file)) return '生成'
  if ((file.description || '').includes('聊天上传')) return '聊天上传'
  return '上传'
}

export default function FilesPage() {
  const [files, setFiles] = useState<FileItem[]>([])
  const [folders, setFolders] = useState<Folder[]>([])
  const [currentFolder, setCurrentFolder] = useState<string | undefined>(undefined)
  const [folderPath, setFolderPath] = useState<Folder[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [filter, setFilter] = useState<FileFilter>('all')
  const [viewMode, setViewMode] = useState<ViewMode>('grid')
  const [loading, setLoading] = useState(true)
  const [uploading, setUploading] = useState(false)
  const [showNewFolder, setShowNewFolder] = useState(false)
  const [newFolderName, setNewFolderName] = useState('')
  const [previewIndex, setPreviewIndex] = useState<number | null>(null)
  const fileInputRef = useRef<HTMLInputElement>(null)

  const loadData = useCallback(async () => {
    setLoading(true)
    try {
      if (searchQuery) {
        const res = await fileApi.search(searchQuery)
        setFiles(res.data.files || [])
        setFolders([])
      } else {
        const [filesRes, foldersRes] = await Promise.all([
          fileApi.list(currentFolder),
          folderApi.list(currentFolder),
        ])
        setFiles(filesRes.data.files || [])
        setFolders(foldersRes.data.folders || [])
      }
    } catch (err) {
      console.error('Load files error:', err)
    } finally {
      setLoading(false)
    }
  }, [currentFolder, searchQuery])

  useEffect(() => {
    const timer = setTimeout(loadData, 250)
    return () => clearTimeout(timer)
  }, [loadData])

  const generatedCount = files.filter(isGeneratedFile).length
  const uploadedCount = files.length - generatedCount
  const visibleFiles = files.filter((file) => {
    if (filter === 'generated') return isGeneratedFile(file)
    if (filter === 'uploaded') return !isGeneratedFile(file)
    return true
  })
  const totalSize = files.reduce((sum, file) => sum + (file.file_size || 0), 0)

  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFiles = e.target.files
    if (!selectedFiles || selectedFiles.length === 0) return

    setUploading(true)
    try {
      for (const file of Array.from(selectedFiles)) {
        await fileApi.upload(file, currentFolder, '手动上传文件')
      }
      loadData()
    } catch (err) {
      console.error('Upload error:', err)
    } finally {
      setUploading(false)
      if (fileInputRef.current) fileInputRef.current.value = ''
    }
  }

  const handleDownload = async (file: FileItem) => {
    try {
      const res = await fileApi.download(file.id)
      const url = URL.createObjectURL(res.data)
      const a = document.createElement('a')
      a.href = url
      a.download = file.name
      a.click()
      URL.revokeObjectURL(url)
    } catch (err) {
      console.error('Download error:', err)
    }
  }

  const handleDelete = async (id: string) => {
    if (!confirm('确定删除此文件？删除后文件中心将不再展示。')) return
    try {
      await fileApi.delete(id)
      setPreviewIndex(null)
      loadData()
    } catch (err) {
      console.error('Delete error:', err)
    }
  }

  const handleDeleteFolder = async (folder: Folder) => {
    if (!confirm(`确定删除文件夹「${folder.name}」及其中所有文件？`)) return
    try {
      await folderApi.delete(folder.id)
      loadData()
    } catch (err) {
      console.error('Delete folder error:', err)
    }
  }

  const handleCreateFolder = async () => {
    if (!newFolderName.trim()) return
    try {
      await folderApi.create(newFolderName.trim(), currentFolder)
      setNewFolderName('')
      setShowNewFolder(false)
      loadData()
    } catch (err) {
      console.error('Create folder error:', err)
    }
  }

  const handleFolderClick = (folder: Folder) => {
    setFilter('all')
    setFolderPath([...folderPath, folder])
    setCurrentFolder(folder.id)
  }

  const handleBreadcrumbClick = (index: number) => {
    setFilter('all')
    if (index === -1) {
      setFolderPath([])
      setCurrentFolder(undefined)
    } else {
      const newPath = folderPath.slice(0, index + 1)
      setFolderPath(newPath)
      setCurrentFolder(newPath[newPath.length - 1].id)
    }
  }

  const previewFile = previewIndex !== null ? visibleFiles[previewIndex] : null

  return (
    <div className="flex h-full flex-col">
      {/* 头部 */}
      <div className="rounded-[28px] border border-black/[0.06] bg-white/75 px-4 py-4 shadow-[0_18px_50px_rgba(24,24,27,0.06)] backdrop-blur-xl lg:px-6">
        <div className="flex flex-col gap-4 lg:flex-row lg:items-center lg:justify-between">
          <div>
            <h1 className="text-xl font-bold text-surface-900">我的文件</h1>
            <p className="mt-1 text-sm text-surface-500">生成过的办公产物和上传过的附件都会汇总在这里，可预览、下载、整理和删除。</p>
          </div>
          <div className="flex flex-wrap items-center gap-2">
            {/* 视图切换 */}
            <div className="flex items-center rounded-lg border border-black/[0.06] bg-white/60 p-0.5">
              <button
                onClick={() => setViewMode('grid')}
                className={`flex h-8 w-8 items-center justify-center rounded-md transition ${viewMode === 'grid' ? 'bg-surface-900 text-white' : 'text-surface-500 hover:text-surface-900'}`}
                title="网格视图"
              >
                <LayoutGrid className="h-4 w-4" />
              </button>
              <button
                onClick={() => setViewMode('list')}
                className={`flex h-8 w-8 items-center justify-center rounded-md transition ${viewMode === 'list' ? 'bg-surface-900 text-white' : 'text-surface-500 hover:text-surface-900'}`}
                title="列表视图"
              >
                <List className="h-4 w-4" />
              </button>
            </div>
            <div className="relative">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-surface-400" />
              <input
                className="input h-9 w-48 pl-9 text-sm lg:w-64"
                placeholder="搜索文件..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>
            <button onClick={() => setShowNewFolder(true)} className="btn-secondary h-9">
              <FolderPlus className="h-4 w-4" />
              新建文件夹
            </button>
            <button onClick={() => fileInputRef.current?.click()} disabled={uploading} className="btn-primary h-9">
              <Upload className="h-4 w-4" />
              {uploading ? '上传中...' : '上传文件'}
            </button>
            <input ref={fileInputRef} type="file" multiple className="hidden" onChange={handleUpload} />
          </div>
        </div>

        {/* 统计卡片 */}
        <div className="mt-4 grid grid-cols-1 gap-3 sm:grid-cols-3">
          <button
            type="button"
            onClick={() => setFilter('all')}
            className={`rounded-2xl border px-4 py-3 text-left transition ${filter === 'all' ? 'border-surface-950 bg-surface-950 text-white' : 'border-black/[0.06] bg-white/55 text-surface-700 hover:bg-white'}`}
          >
            <div className="text-xs opacity-70">全部文件</div>
            <div className="mt-1 text-2xl font-black">{files.length}</div>
            <div className="mt-1 text-[11px] opacity-65">{formatSize(totalSize)} · {folders.length} 个文件夹</div>
          </button>
          <button
            type="button"
            onClick={() => setFilter('generated')}
            className={`rounded-2xl border px-4 py-3 text-left transition ${filter === 'generated' ? 'border-primary-600 bg-primary-600 text-white' : 'border-black/[0.06] bg-white/55 text-surface-700 hover:bg-white'}`}
          >
            <div className="flex items-center gap-1.5 text-xs opacity-75"><Sparkles className="h-3.5 w-3.5" />生成产物</div>
            <div className="mt-1 text-2xl font-black">{generatedCount}</div>
            <div className="mt-1 text-[11px] opacity-65">PPT、Word、Excel、图表等</div>
          </button>
          <button
            type="button"
            onClick={() => setFilter('uploaded')}
            className={`rounded-2xl border px-4 py-3 text-left transition ${filter === 'uploaded' ? 'border-emerald-600 bg-emerald-600 text-white' : 'border-black/[0.06] bg-white/55 text-surface-700 hover:bg-white'}`}
          >
            <div className="flex items-center gap-1.5 text-xs opacity-75"><Upload className="h-3.5 w-3.5" />上传文件</div>
            <div className="mt-1 text-2xl font-black">{uploadedCount}</div>
            <div className="mt-1 text-[11px] opacity-65">文件页上传和聊天附件</div>
          </button>
        </div>
      </div>

      {/* 面包屑 */}
      {!searchQuery && (
        <div className="mt-4 flex items-center gap-1 rounded-2xl border border-black/[0.05] bg-white/60 px-4 py-2 text-sm backdrop-blur-xl">
          <button onClick={() => handleBreadcrumbClick(-1)} className="flex items-center gap-1 text-surface-500 hover:text-surface-900">
            <HardDrive className="h-3.5 w-3.5" />
            全部文件
          </button>
          {folderPath.map((folder, idx) => (
            <div key={folder.id} className="flex items-center gap-1">
              <ChevronRight className="h-3.5 w-3.5 text-surface-300" />
              <button onClick={() => handleBreadcrumbClick(idx)} className="text-surface-500 hover:text-surface-900">
                {folder.name}
              </button>
            </div>
          ))}
        </div>
      )}

      {/* 文件列表 */}
      <div className="flex-1 overflow-auto pt-4">
        {loading ? (
          <div className="text-center text-surface-400">加载中...</div>
        ) : visibleFiles.length === 0 && folders.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <div className="flex h-16 w-16 items-center justify-center rounded-full bg-surface-100">
              <FolderIcon className="h-8 w-8 text-surface-300" />
            </div>
            <p className="mt-4 text-sm text-surface-400">
              {searchQuery ? '未找到匹配的文件' : filter === 'generated' ? '还没有生成产物，去智能助手生成后会自动出现在这里' : filter === 'uploaded' ? '还没有上传文件' : '此文件夹为空'}
            </p>
            {!searchQuery && filter !== 'generated' && (
              <button onClick={() => fileInputRef.current?.click()} className="mt-4 btn-primary">
                <Upload className="h-4 w-4" />
                上传第一个文件
              </button>
            )}
          </div>
        ) : viewMode === 'grid' ? (
          /* ===== 网格视图（缩略图） ===== */
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5">
            {filter === 'all' && folders.map(folder => (
              <div key={folder.id} className="card group relative flex flex-col p-3 transition-all hover:-translate-y-0.5 hover:shadow-md">
                <button type="button" onClick={() => handleFolderClick(folder)} className="flex flex-col items-center gap-2 text-left">
                  <div className="flex h-28 w-full items-center justify-center rounded-xl bg-amber-50">
                    <FolderIcon className="h-12 w-12 text-amber-400" />
                  </div>
                  <div className="w-full">
                    <p className="truncate text-sm font-semibold text-surface-800">{folder.name}</p>
                    <p className="mt-0.5 text-xs text-surface-400">文件夹 · {formatDate(folder.updated_at)}</p>
                  </div>
                </button>
                <button
                  type="button"
                  onClick={() => handleDeleteFolder(folder)}
                  className="absolute right-2 top-2 flex h-7 w-7 items-center justify-center rounded-lg text-surface-300 opacity-0 transition hover:bg-red-50 hover:text-red-600 group-hover:opacity-100"
                  title="删除文件夹"
                >
                  <Trash2 className="h-3.5 w-3.5" />
                </button>
              </div>
            ))}

            {visibleFiles.map((file, idx) => {
              const generated = isGeneratedFile(file)
              return (
                <div
                  key={file.id}
                  className="card group relative flex cursor-pointer flex-col p-3 transition-all hover:-translate-y-0.5 hover:shadow-md"
                  onClick={() => setPreviewIndex(idx)}
                >
                  <FileThumbnail file={file} size="md" />
                  <div className="mt-2 flex-1">
                    <div className="flex items-start gap-1">
                      <p className="min-w-0 flex-1 truncate text-sm font-semibold text-surface-800" title={file.name}>{file.name}</p>
                      <span className={`shrink-0 rounded-full px-1.5 py-0.5 text-[9px] font-bold ${generated ? 'bg-primary-50 text-primary-700' : 'bg-emerald-50 text-emerald-700'}`}>
                        {fileSourceLabel(file)}
                      </span>
                    </div>
                    <p className="mt-0.5 truncate text-xs text-surface-400">
                      {formatSize(file.file_size)} · {formatDate(file.updated_at)}
                    </p>
                  </div>
                  {/* 悬浮操作栏 */}
                  <div className="mt-2 flex items-center gap-1.5 opacity-0 transition group-hover:opacity-100">
                    <button
                      onClick={(e) => { e.stopPropagation(); setPreviewIndex(idx) }}
                      className="flex h-7 flex-1 items-center justify-center gap-1 rounded-lg bg-surface-100 text-xs font-medium text-surface-600 hover:bg-surface-200 transition"
                    >
                      <Eye className="h-3.5 w-3.5" /> 预览
                    </button>
                    <button
                      onClick={(e) => { e.stopPropagation(); handleDownload(file) }}
                      className="flex h-7 w-7 items-center justify-center rounded-lg bg-surface-100 text-surface-600 hover:bg-surface-200 transition"
                      title="下载"
                    >
                      <Download className="h-3.5 w-3.5" />
                    </button>
                    <button
                      onClick={(e) => { e.stopPropagation(); handleDelete(file.id) }}
                      className="flex h-7 w-7 items-center justify-center rounded-lg bg-red-50 text-red-500 hover:bg-red-100 transition"
                      title="删除"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                </div>
              )
            })}
          </div>
        ) : (
          /* ===== 列表视图 ===== */
          <div className="grid grid-cols-1 gap-3 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
            {filter === 'all' && folders.map(folder => (
              <div key={folder.id} className="card group relative flex items-center gap-3 p-4 transition-all hover:shadow-md">
                <button type="button" onClick={() => handleFolderClick(folder)} className="flex min-w-0 flex-1 items-center gap-3 text-left">
                  <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl bg-amber-50 text-amber-500">
                    <FolderIcon className="h-7 w-7" />
                  </div>
                  <div className="min-w-0">
                    <p className="truncate text-sm font-semibold text-surface-800">{folder.name}</p>
                    <p className="mt-1 text-xs text-surface-400">文件夹 · {formatDate(folder.updated_at)}</p>
                  </div>
                </button>
                <button
                  type="button"
                  onClick={() => handleDeleteFolder(folder)}
                  className="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl text-surface-300 opacity-0 transition hover:bg-red-50 hover:text-red-600 group-hover:opacity-100"
                  title="删除文件夹"
                >
                  <Trash2 className="h-4 w-4" />
                </button>
              </div>
            ))}

            {visibleFiles.map((file, idx) => {
              const Icon = FILE_ICONS[file.file_type] || FileIcon
              const colorClass = FILE_COLORS[file.file_type] || FILE_COLORS.other
              const generated = isGeneratedFile(file)
              return (
                <div
                  key={file.id}
                  className="card group relative flex cursor-pointer items-start gap-3 p-4 transition-all hover:-translate-y-0.5 hover:shadow-md"
                  onClick={() => setPreviewIndex(idx)}
                >
                  <div className={`flex h-12 w-12 shrink-0 items-center justify-center rounded-2xl ${colorClass}`}>
                    <Icon className="h-6 w-6" />
                  </div>
                  <div className="min-w-0 flex-1">
                    <div className="flex items-start gap-2">
                      <p className="min-w-0 flex-1 truncate text-sm font-semibold text-surface-800" title={file.name}>{file.name}</p>
                      <span className={`shrink-0 rounded-full px-2 py-0.5 text-[10px] font-bold ${generated ? 'bg-primary-50 text-primary-700' : 'bg-emerald-50 text-emerald-700'}`}>
                        {fileSourceLabel(file)}
                      </span>
                    </div>
                    <p className="mt-1 truncate text-xs text-surface-400">
                      {formatSize(file.file_size)} · {formatDate(file.updated_at)} · {file.file_type || 'file'}
                    </p>
                    {file.description && <p className="mt-2 line-clamp-1 text-[11px] text-surface-400">{file.description}</p>}
                    <div className="mt-3 flex gap-2">
                      <button
                        onClick={(e) => { e.stopPropagation(); setPreviewIndex(idx) }}
                        className="btn-secondary h-8 px-3 text-xs"
                      >
                        <Eye className="h-3.5 w-3.5" />
                        预览
                      </button>
                      <button
                        onClick={(e) => { e.stopPropagation(); handleDownload(file) }}
                        className="inline-flex h-8 items-center gap-1.5 rounded-lg border border-surface-200 bg-white px-3 text-xs font-semibold text-surface-600 transition hover:bg-surface-50"
                      >
                        <Download className="h-3.5 w-3.5" />
                        下载
                      </button>
                      <button
                        onClick={(e) => { e.stopPropagation(); handleDelete(file.id) }}
                        className="inline-flex h-8 items-center gap-1.5 rounded-lg border border-red-100 bg-red-50 px-3 text-xs font-semibold text-red-600 transition hover:bg-red-100"
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </button>
                    </div>
                  </div>
                </div>
              )
            })}
          </div>
        )}
      </div>

      {/* 新建文件夹弹窗 */}
      {showNewFolder && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/30 p-4" onClick={() => setShowNewFolder(false)}>
          <div className="w-full max-w-sm rounded-2xl bg-white p-6 shadow-xl" onClick={(e) => e.stopPropagation()}>
            <h3 className="mb-4 text-lg font-semibold text-surface-900">新建文件夹</h3>
            <input
              className="input"
              placeholder="文件夹名称"
              value={newFolderName}
              onChange={(e) => setNewFolderName(e.target.value)}
              autoFocus
              onKeyDown={(e) => e.key === 'Enter' && handleCreateFolder()}
            />
            <div className="mt-4 flex justify-end gap-2">
              <button onClick={() => setShowNewFolder(false)} className="btn-secondary">取消</button>
              <button onClick={handleCreateFolder} className="btn-primary" disabled={!newFolderName.trim()}>
                创建
              </button>
            </div>
          </div>
        </div>
      )}

      {/* 文件预览弹窗 */}
      {previewFile && (
        <FilePreviewModal
          file={previewFile}
          onClose={() => setPreviewIndex(null)}
          onPrev={previewIndex !== null && previewIndex > 0 ? () => setPreviewIndex(previewIndex - 1) : undefined}
          onNext={previewIndex !== null && previewIndex < visibleFiles.length - 1 ? () => setPreviewIndex(previewIndex + 1) : undefined}
          onDownload={handleDownload}
        />
      )}
    </div>
  )
}
