import { useState, useEffect } from 'react'
import {
  FileText, Image, FileSpreadsheet, FileType, FileCode,
  File as FileIcon, Play,
} from 'lucide-react'
import { fileApi } from '@/api'
import type { FileItem } from '@/types'

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

interface FileThumbnailProps {
  file: FileItem
  size?: 'sm' | 'md' | 'lg'
}

export function FileThumbnail({ file, size = 'md' }: FileThumbnailProps) {
  const [imgUrl, setImgUrl] = useState<string | null>(null)

  const isImage = file.file_type === 'image'
  const Icon = FILE_ICONS[file.file_type] || FileIcon
  const colorClass = FILE_COLORS[file.file_type] || FILE_COLORS.other

  const iconSize = size === 'sm' ? 'h-6 w-6' : size === 'lg' ? 'h-10 w-10' : 'h-8 w-8'
  const containerSize = size === 'sm' ? 'h-20' : size === 'lg' ? 'h-40' : 'h-28'

  useEffect(() => {
    if (!isImage) {
      setImgUrl(null)
      return
    }
    let url: string | null = null
    let cancelled = false
    fileApi.thumbnail(file.id)
      .then(res => {
        if (!cancelled) {
          url = URL.createObjectURL(res.data)
          setImgUrl(url)
        }
      })
      .catch(() => {})
    return () => {
      cancelled = true
      if (url) URL.revokeObjectURL(url)
    }
  }, [file.id, isImage])

  if (isImage && imgUrl) {
    return (
      <div className={`${containerSize} w-full overflow-hidden rounded-xl bg-surface-50`}>
        <img src={imgUrl} alt={file.name} className="h-full w-full object-cover" />
      </div>
    )
  }

  // 非图片：展示图标 + 文件类型色块
  return (
    <div className={`${containerSize} w-full flex items-center justify-center rounded-xl ${colorClass}`}>
      <Icon className={iconSize} />
    </div>
  )
}
