import { useEffect, useState } from 'react'
import { X, Download, ChevronLeft, ChevronRight, FileText, FileSpreadsheet, FileType, FileCode, File as FileIcon, ChevronDown } from 'lucide-react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { fileApi } from '@/api'
import type { FileItem } from '@/types'

interface FilePreviewModalProps {
  file: FileItem
  onClose: () => void
  onPrev?: () => void
  onNext?: () => void
  onDownload: (file: FileItem) => void
}

interface PreviewData {
  preview_type: string
  text?: string
  structured?: any
  data_url?: string
  mime_type?: string
  name: string
  file_type: string
  file_size: number
}

// ===== PPT 结构化预览 =====
function PptStructuredPreview({ data }: { data: any }) {
  const slides = data?.slides || []
  const [currentSlide, setCurrentSlide] = useState(0)

  if (slides.length === 0) {
    return <div className="text-center text-surface-400 py-20">此演示文稿暂无可预览的内容</div>
  }

  const slide = slides[currentSlide]
  const elements = slide.elements || []

  return (
    <div className="w-full max-w-5xl mx-auto">
      {/* 幻灯片选择器 */}
      {slides.length > 1 && (
        <div className="mb-4 flex items-center gap-2 overflow-x-auto pb-2">
          {slides.map((s: any, i: number) => (
            <button
              key={i}
              onClick={() => setCurrentSlide(i)}
              className={`shrink-0 rounded-lg px-3 py-1.5 text-xs font-medium transition ${
                i === currentSlide
                  ? 'bg-orange-500 text-white'
                  : 'bg-white/10 text-white/60 hover:bg-white/20'
              }`}
            >
              第 {s.index} 页
            </button>
          ))}
        </div>
      )}

      {/* 幻灯片预览区 */}
      <div
        className="rounded-2xl shadow-2xl overflow-hidden mx-auto"
        style={{
          width: '100%',
          maxWidth: '960px',
          aspectRatio: '16 / 9',
          background: slide.background ? `#${slide.background}` : '#fff',
        }}
      >
        <div className="w-full h-full flex flex-col justify-center p-8 md:p-12">
          {/* 标题 */}
          <div className="mb-4">
            <h3 className="text-xl md:text-2xl font-bold text-surface-900 line-clamp-2">
              {slide.title || `第 ${slide.index} 页`}
            </h3>
          </div>
          {/* 内容元素 */}
          <div className="space-y-2">
            {elements.map((el: any, i: number) => (
              <div
                key={i}
                className="text-sm md:text-base text-surface-700 leading-relaxed"
              >
                {el.text}
              </div>
            ))}
          </div>
        </div>
      </div>

      {/* 导航 */}
      <div className="mt-4 flex items-center justify-center gap-4">
        <button
          onClick={() => setCurrentSlide(Math.max(0, currentSlide - 1))}
          disabled={currentSlide === 0}
          className="flex h-9 w-9 items-center justify-center rounded-full bg-white/10 text-white disabled:opacity-30 hover:bg-white/20 transition"
        >
          <ChevronLeft className="h-4 w-4" />
        </button>
        <span className="text-sm text-white/60">
          {currentSlide + 1} / {slides.length}
        </span>
        <button
          onClick={() => setCurrentSlide(Math.min(slides.length - 1, currentSlide + 1))}
          disabled={currentSlide === slides.length - 1}
          className="flex h-9 w-9 items-center justify-center rounded-full bg-white/10 text-white disabled:opacity-30 hover:bg-white/20 transition"
        >
          <ChevronRight className="h-4 w-4" />
        </button>
      </div>
    </div>
  )
}

// ===== Excel 结构化预览 =====
function ExcelStructuredPreview({ data }: { data: any }) {
  const sheets = data?.sheets || []
  const [currentSheet, setCurrentSheet] = useState(0)

  if (sheets.length === 0) {
    return <div className="text-center text-surface-400 py-20">此表格暂无可预览的内容</div>
  }

  const sheet = sheets[currentSheet]
  const rows = sheet.rows || []

  return (
    <div className="mx-auto max-w-5xl w-full">
      {/* Sheet 切换 */}
      {sheets.length > 1 && (
        <div className="mb-4 flex items-center gap-2 overflow-x-auto pb-2">
          {sheets.map((s: any, i: number) => (
            <button
              key={i}
              onClick={() => setCurrentSheet(i)}
              className={`shrink-0 rounded-lg px-3 py-1.5 text-xs font-medium transition ${
                i === currentSheet
                  ? 'bg-emerald-500 text-white'
                  : 'bg-white/10 text-white/60 hover:bg-white/20'
              }`}
            >
              {s.name}
            </button>
          ))}
        </div>
      )}

      {/* 表格 */}
      <div className="rounded-2xl border border-surface-200 bg-white shadow-lg overflow-hidden">
        <div className="overflow-auto max-h-[70vh]">
          <table className="w-full border-collapse text-sm">
            <tbody>
              {rows.map((row: string[], ri: number) => (
                <tr key={ri} className={ri === 0 ? 'bg-surface-100 font-semibold' : ri % 2 === 1 ? 'bg-surface-50' : ''}>
                  {row.map((cell: string, ci: number) => (
                    <td
                      key={ci}
                      className="border border-surface-200 px-3 py-2 text-surface-700 whitespace-nowrap"
                    >
                      {cell || ''}
                    </td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>

      {/* 提示 */}
      {sheet.truncated && (
        <p className="mt-3 text-center text-xs text-white/40">
          数据较多，仅展示前 200 行，完整数据请下载查看
        </p>
      )}
      <p className="mt-2 text-center text-xs text-white/40">
        {sheet.name} · {rows.length} 行
      </p>
    </div>
  )
}

// ===== Word 结构化预览 =====
function DocStructuredPreview({ data }: { data: any }) {
  const sections = data?.sections || []

  if (sections.length === 0) {
    return <div className="text-center text-surface-400 py-20">此文档暂无可预览的内容</div>
  }

  return (
    <div className="mx-auto max-w-4xl w-full">
      <div
        className="rounded-2xl bg-white shadow-lg ring-1 ring-surface-200 overflow-hidden"
        style={{
          padding: '56px 64px',
          minHeight: '600px',
          fontFamily: '"SimSun", "宋体", "Noto Serif SC", serif',
        }}
      >
        {/* 文档标题 */}
        {data.title && (
          <div className="mb-8 pb-6 text-center" style={{ borderBottom: '2px solid #e5e7eb' }}>
            <h1 className="text-2xl font-bold text-gray-900" style={{ fontFamily: '"SimHei", "黑体", "Noto Sans SC", sans-serif' }}>
              {data.title}
            </h1>
          </div>
        )}

        {/* 正文 */}
        <div className="space-y-3">
          {sections.map((sec: any, i: number) => {
            if (sec.type === 'heading') {
              const level = sec.level || 1
              const cls = level === 1
                ? 'mb-3 mt-6 text-xl font-bold text-gray-900'
                : level === 2
                  ? 'mb-2 mt-5 text-lg font-semibold text-gray-800'
                  : 'mb-2 mt-4 text-base font-semibold text-gray-700'
              return (
                <h2 key={i} className={cls} style={{ fontFamily: '"SimHei", "黑体", "Noto Sans SC", sans-serif' }}>
                  {sec.text}
                </h2>
              )
            }
            if (sec.type === 'bullet') {
              return (
                <div key={i} className="flex gap-2 pl-5">
                  <span className="text-gray-500 mt-0.5">•</span>
                  <span className="text-[14px] leading-[1.8] text-gray-700">{sec.text}</span>
                </div>
              )
            }
            if (sec.type === 'table') {
              const headers = sec.headers || []
              const bodyRows = sec.rows || []
              return (
                <div key={i} className="my-4 overflow-x-auto">
                  <table className="w-full border-collapse text-sm">
                    <thead>
                      <tr>
                        {headers.map((h: string, ci: number) => (
                          <th key={ci} className="border border-gray-300 bg-gray-100 px-3 py-2 text-left font-semibold text-gray-800">
                            {h}
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {bodyRows.map((row: string[], ri: number) => (
                        <tr key={ri} className={ri % 2 === 1 ? 'bg-gray-50' : ''}>
                          {row.map((cell: string, ci: number) => (
                            <td key={ci} className="border border-gray-300 px-3 py-2 text-gray-700">
                              {cell}
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              )
            }
            // paragraph
            return (
              <p key={i} className="mb-3 text-[14px] leading-[1.8] text-gray-700">
                {sec.text}
              </p>
            )
          })}
        </div>

        {/* 页脚 */}
        <div className="mt-12 pt-4 text-center text-[11px] text-gray-400" style={{ borderTop: '1px solid #e5e7eb' }}>
          — WaLiOffice 文档预览 · 下载获取完整排版 —
        </div>
      </div>
    </div>
  )
}

export function FilePreviewModal({ file, onClose, onPrev, onNext, onDownload }: FilePreviewModalProps) {
  const [preview, setPreview] = useState<PreviewData | null>(null)
  const [loading, setLoading] = useState(true)
  const [error, setError] = useState('')

  useEffect(() => {
    let cancelled = false
    setLoading(true)
    setError('')
    setPreview(null)
    fileApi.preview(file.id)
      .then(res => {
        if (!cancelled) setPreview(res.data)
      })
      .catch(err => {
        if (!cancelled) setError(err?.response?.data?.detail || '预览加载失败')
      })
      .finally(() => {
        if (!cancelled) setLoading(false)
      })
    return () => { cancelled = true }
  }, [file.id])

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
      if (e.key === 'ArrowLeft' && onPrev) onPrev()
      if (e.key === 'ArrowRight' && onNext) onNext()
    }
    window.addEventListener('keydown', handler)
    return () => window.removeEventListener('keydown', handler)
  }, [onClose, onPrev, onNext])

  const fileTypeLabel = (t: string) => {
    const map: Record<string, string> = {
      ppt: 'PPT 演示文稿', doc: 'Word 文档', document: 'Word 文档',
      excel: 'Excel 表格', sheet: 'Excel 表格', image: '图片',
      video: '视频', drawio: 'Draw.io 图表', code: '代码', other: '文件',
    }
    return map[t] || '文件'
  }

  const renderPreview = () => {
    if (loading) return <div className="flex items-center justify-center py-20 text-surface-300">加载预览中...</div>
    if (error) return (
      <div className="flex flex-col items-center justify-center py-20 text-center">
        <FileIcon className="h-12 w-12 text-surface-300 mb-3" />
        <p className="text-sm text-surface-300">{error}</p>
      </div>
    )
    if (!preview) return null

    const pt = preview.preview_type

    // 图片预览
    if (pt === 'image' && preview.data_url) {
      return (
        <div className="flex flex-col items-center">
          <img
            src={preview.data_url}
            alt={preview.name}
            className="max-h-[78vh] max-w-full rounded-xl shadow-2xl object-contain"
          />
        </div>
      )
    }

    // Draw.io 预览
    if (pt === 'drawio' && preview.text) {
      return (
        <div className="w-full">
          <div className="rounded-xl border border-white/15 bg-white/5 p-4">
            <p className="mb-3 text-sm font-medium text-white/70">Draw.io XML 源码</p>
            <pre className="max-h-[70vh] overflow-auto rounded-lg bg-white/90 p-4 text-xs leading-relaxed text-surface-700">
              {preview.text}
            </pre>
          </div>
        </div>
      )
    }

    // Markdown 预览
    if (pt === 'markdown' && preview.text) {
      return (
        <div className="mx-auto max-w-4xl w-full">
          <div className="rounded-2xl border border-surface-200 bg-white p-8 shadow-sm">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{preview.text}</ReactMarkdown>
          </div>
        </div>
      )
    }

    // PPT 结构化预览
    if (pt === 'presentation' && preview.structured) {
      return <PptStructuredPreview data={preview.structured} />
    }

    // Excel 结构化预览
    if (pt === 'spreadsheet' && preview.structured) {
      return <ExcelStructuredPreview data={preview.structured} />
    }

    // Word 结构化预览
    if (pt === 'document' && preview.structured) {
      return <DocStructuredPreview data={preview.structured} />
    }

    // 回退：文本预览
    if (preview.text) {
      const lines = preview.text.split('\n')
      return (
        <div className="mx-auto max-w-4xl w-full">
          <div className="rounded-2xl border border-surface-200 bg-white p-6 shadow-sm">
            <div className="mb-4 flex items-center gap-2 border-b border-surface-200 pb-3">
              {preview.file_type === 'excel' && <FileSpreadsheet className="h-5 w-5 text-emerald-500" />}
              {preview.file_type === 'ppt' && <FileType className="h-5 w-5 text-orange-500" />}
              {(preview.file_type === 'doc' || preview.file_type === 'document') && <FileText className="h-5 w-5 text-blue-500" />}
              {preview.file_type === 'drawio' && <FileCode className="h-5 w-5 text-amber-500" />}
              <span className="text-sm font-semibold text-surface-800">{preview.name}</span>
              <span className="ml-auto text-xs text-surface-400">{lines.length} 行提取文本</span>
            </div>
            <pre className="max-h-[70vh] overflow-auto whitespace-pre-wrap rounded-lg bg-surface-50 p-4 text-sm leading-relaxed text-surface-700">
              {preview.text}
            </pre>
          </div>
        </div>
      )
    }

    return (
      <div className="flex flex-col items-center justify-center py-20 text-center">
        <FileIcon className="h-12 w-12 text-surface-300 mb-3" />
        <p className="text-sm text-surface-300">此文件类型暂不支持在线预览</p>
      </div>
    )
  }

  return (
    <div className="fixed inset-0 z-50 flex flex-col bg-black/70 backdrop-blur-sm" onClick={onClose}>
      {/* 预览区 — 占满主体空间 */}
      <div className="flex-1 overflow-auto flex flex-col items-center justify-start pt-8 px-4" onClick={e => e.stopPropagation()}>
        <div className="mx-auto w-full max-w-6xl flex flex-col items-center">
          {renderPreview()}
        </div>
      </div>

      {/* 底部操作栏 — 始终在底部可见，包含文件名、切换、下载、关闭 */}
      <div
        className="shrink-0 flex items-center justify-center gap-3 px-6 py-4 bg-black/40 backdrop-blur-md"
        onClick={e => e.stopPropagation()}
      >
        {/* 上一个 */}
        {onPrev && (
          <button onClick={onPrev} className="flex h-10 w-10 items-center justify-center rounded-full bg-white/10 text-white hover:bg-white/20 transition" title="上一个 (←)">
            <ChevronLeft className="h-5 w-5" />
          </button>
        )}

        {/* 文件信息 */}
        <div className="flex min-w-0 items-center gap-3 px-3">
          <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-white/10">
            <FileIcon className="h-4.5 w-4.5 text-white" />
          </div>
          <div className="min-w-0 hidden sm:block">
            <p className="truncate max-w-[200px] text-sm font-medium text-white">{file.name}</p>
            <p className="text-[11px] text-white/50">{fileTypeLabel(file.file_type)} · {(file.file_size / 1024).toFixed(1)} KB</p>
          </div>
        </div>

        {/* 下载 */}
        <button
          onClick={() => onDownload(file)}
          className="flex h-10 items-center gap-1.5 rounded-full bg-white/15 px-4 text-sm font-medium text-white hover:bg-white/25 transition"
          title="下载"
        >
          <Download className="h-4 w-4" /> 下载
        </button>

        {/* 关闭 */}
        <button
          onClick={onClose}
          className="flex h-10 items-center gap-1.5 rounded-full bg-white text-surface-900 px-4 text-sm font-semibold shadow-lg hover:bg-surface-100 transition"
          title="关闭预览 (ESC)"
        >
          <X className="h-4 w-4" strokeWidth={2.5} /> 关闭
        </button>

        {/* 下一个 */}
        {onNext && (
          <button onClick={onNext} className="flex h-10 w-10 items-center justify-center rounded-full bg-white/10 text-white hover:bg-white/20 transition" title="下一个 (→)">
            <ChevronRight className="h-5 w-5" />
          </button>
        )}
      </div>
    </div>
  )
}
