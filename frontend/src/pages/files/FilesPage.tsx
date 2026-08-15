import { useState, useEffect, useCallback, useRef } from 'react'
import {
  Upload, Search, Download, Trash2, Folder as FolderIcon,
  FileText, Image, FileSpreadsheet, FileType, FileCode,
  File as FileIcon, FolderPlus, ChevronRight, HardDrive,
} from 'lucide-react'
import { fileApi, folderApi } from '@/api'
import type { FileItem, Folder } from '@/types'

const FILE_ICONS: Record<string, typeof FileIcon> = {
  ppt: FileType,
  doc: FileText,
  sheet: FileSpreadsheet,
  image: Image,
  drawio: FileCode,
  code: FileCode,
  other: FileIcon,
}

const FILE_COLORS: Record<string, string> = {
  ppt: 'text-orange-500 bg-orange-50',
  doc: 'text-blue-500 bg-blue-50',
  sheet: 'text-emerald-500 bg-emerald-50',
  image: 'text-violet-500 bg-violet-50',
  drawio: 'text-amber-500 bg-amber-50',
  code: 'text-slate-500 bg-slate-50',
  other: 'text-surface-400 bg-surface-50',
}

function formatSize(bytes: number): string {
  if (bytes === 0) return '0 B'
  const units = ['B', 'KB', 'MB', 'GB']
  const i = Math.floor(Math.log(bytes) / Math.log(1024))
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`
}

export default function FilesPage() {
  const [files, setFiles] = useState<FileItem[]>([])
  const [folders, setFolders] = useState<Folder[]>([])
  const [currentFolder, setCurrentFolder] = useState<string | undefined>(undefined)
  const [folderPath, setFolderPath] = useState<Folder[]>([])
  const [searchQuery, setSearchQuery] = useState('')
  const [loading, setLoading] = useState(true)
  const [uploading, setUploading] = useState(false)
  const [showNewFolder, setShowNewFolder] = useState(false)
  const [newFolderName, setNewFolderName] = useState('')
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

  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const selectedFiles = e.target.files
    if (!selectedFiles || selectedFiles.length === 0) return

    setUploading(true)
    try {
      for (const file of Array.from(selectedFiles)) {
        await fileApi.upload(file, currentFolder)
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
    if (!confirm('确定删除此文件？')) return
    try {
      await fileApi.delete(id)
      loadData()
    } catch (err) {
      console.error('Delete error:', err)
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
    setFolderPath([...folderPath, folder])
    setCurrentFolder(folder.id)
  }

  const handleBreadcrumbClick = (index: number) => {
    if (index === -1) {
      setFolderPath([])
      setCurrentFolder(undefined)
    } else {
      const newPath = folderPath.slice(0, index + 1)
      setFolderPath(newPath)
      setCurrentFolder(newPath[newPath.length - 1].id)
    }
  }

  return (
    <div className="flex h-full flex-col">
      {/* 顶部工具栏 */}
      <div className="rounded-[28px] border border-black/[0.06] bg-white/75 px-4 py-4 shadow-[0_18px_50px_rgba(24,24,27,0.06)] backdrop-blur-xl lg:px-6">
        <div className="flex items-center justify-between gap-4">
          <div>
            <h1 className="text-xl font-bold text-surface-900">我的文件</h1>
            <p className="mt-1 text-sm text-surface-500">查看、整理和下载你的办公产物，不再是传统后台式文件中心。</p>
          </div>
          <div className="flex items-center gap-2">
            <div className="relative">
              <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-surface-400" />
              <input
                className="input h-9 w-48 pl-9 text-sm lg:w-64"
                placeholder="搜索文件..."
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
              />
            </div>
            <button
              onClick={() => setShowNewFolder(true)}
              className="btn-secondary h-9"
            >
              <FolderPlus className="h-4 w-4" />
              新建文件夹
            </button>
            <button
              onClick={() => fileInputRef.current?.click()}
              disabled={uploading}
              className="btn-primary h-9"
            >
              <Upload className="h-4 w-4" />
              {uploading ? '上传中...' : '上传文件'}
            </button>
            <input
              ref={fileInputRef}
              type="file"
              multiple
              className="hidden"
              onChange={handleUpload}
            />
          </div>
        </div>
      </div>

      {/* 面包屑 */}
      {!searchQuery && (
        <div className="mt-4 flex items-center gap-1 rounded-2xl border border-black/[0.05] bg-white/60 px-4 py-2 text-sm backdrop-blur-xl">
          <button
            onClick={() => handleBreadcrumbClick(-1)}
            className="flex items-center gap-1 text-surface-500 hover:text-surface-900"
          >
            <HardDrive className="h-3.5 w-3.5" />
            全部文件
          </button>
          {folderPath.map((folder, idx) => (
            <div key={folder.id} className="flex items-center gap-1">
              <ChevronRight className="h-3.5 w-3.5 text-surface-300" />
              <button
                onClick={() => handleBreadcrumbClick(idx)}
                className="text-surface-500 hover:text-surface-900"
              >
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
        ) : files.length === 0 && folders.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-20 text-center">
            <div className="flex h-16 w-16 items-center justify-center rounded-full bg-surface-100">
              <FolderIcon className="h-8 w-8 text-surface-300" />
            </div>
            <p className="mt-4 text-sm text-surface-400">
              {searchQuery ? '未找到匹配的文件' : '此文件夹为空'}
            </p>
            {!searchQuery && (
              <button
                onClick={() => fileInputRef.current?.click()}
                className="mt-4 btn-primary"
              >
                <Upload className="h-4 w-4" />
                上传第一个文件
              </button>
            )}
          </div>
        ) : (
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6">
            {/* 文件夹 */}
            {folders.map(folder => (
              <button
                key={folder.id}
                onClick={() => handleFolderClick(folder)}
                className="card flex flex-col items-center justify-center p-4 transition-all hover:shadow-md"
              >
                <FolderIcon className="h-10 w-10 text-amber-400" />
                <p className="mt-2 w-full truncate text-center text-sm text-surface-700">{folder.name}</p>
              </button>
            ))}
            {/* 文件 */}
            {files.map(file => {
              const Icon = FILE_ICONS[file.file_type] || FileIcon
              const colorClass = FILE_COLORS[file.file_type] || FILE_COLORS.other
              return (
                <div
                  key={file.id}
                  className="card group relative flex flex-col items-center p-4 transition-all hover:shadow-md"
                >
                  <div className={`flex h-12 w-12 items-center justify-center rounded-lg ${colorClass}`}>
                    <Icon className="h-6 w-6" />
                  </div>
                  <p className="mt-2 w-full truncate text-center text-sm text-surface-700" title={file.name}>
                    {file.name}
                  </p>
                  <p className="mt-0.5 text-[10px] text-surface-400">{formatSize(file.file_size)}</p>

                  {/* 操作按钮 */}
                  <div className="absolute right-2 top-2 flex gap-1 opacity-0 transition-opacity group-hover:opacity-100">
                    <button
                      onClick={() => handleDownload(file)}
                      className="flex h-7 w-7 items-center justify-center rounded-lg bg-white/90 text-surface-500 shadow-sm hover:text-surface-900"
                      title="下载"
                    >
                      <Download className="h-3.5 w-3.5" />
                    </button>
                    <button
                      onClick={() => handleDelete(file.id)}
                      className="flex h-7 w-7 items-center justify-center rounded-lg bg-white/90 text-surface-500 shadow-sm hover:text-red-600"
                      title="删除"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
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
    </div>
  )
}
