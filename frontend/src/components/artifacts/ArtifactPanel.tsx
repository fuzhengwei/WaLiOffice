import { Clapperboard, Code2, Download, Eye, GripVertical, Image, Layers3, Maximize2, Minimize2, MessageSquarePlus, PenTool, Pencil, Save, Sparkles } from 'lucide-react'
import { useEffect, useMemo, useRef, useState } from 'react'
import ReactMarkdown from 'react-markdown'
import remarkGfm from 'remark-gfm'
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter'
import { oneLight } from 'react-syntax-highlighter/dist/esm/styles/prism'
import { DrawIoEmbed, type DrawIoEmbedRef } from '@/lib/react-drawio'
import { EChartsView } from '@/components/preview/EChartsView'
import { SlideList } from '@/components/slides/SlideList'
import { SlidePreview } from '@/components/preview/SlidePreview'
import { Toolbar } from '@/components/toolbar/Toolbar'
import { WordPreview } from '@/components/artifacts/WordPreview'
import type { Artifact, ChatMessage, PPTProject, Slide, ToolKind } from '@/types'
import { findArtifactTurnGroup, groupArtifactsByTurn } from '@/lib/artifact-turns'

interface ArtifactPanelProps {
  activeTool: ToolKind
  project: PPTProject | null
  slides: Slide[]
  currentSlideIndex: number
  pptProgress: { current: number; total: number } | null
  isGeneratingPpt: boolean
  isOpen: boolean
  isWide: boolean
  onOpenChange: (open: boolean) => void
  onWideChange: (wide: boolean) => void
  onSelectSlide: (index: number) => void
  onExportPpt: () => void
  onPresent: () => void
  messages: ChatMessage[]
  activeArtifact: Artifact | null
  artifacts: Artifact[]
  onSelectArtifact: (id: string) => void
  onUpdateArtifact: (id: string, updates: Partial<Artifact>) => void
  onExportExcel: (artifact: Artifact) => void
  onExportDocx: (artifact: Artifact) => void
  onExportMarkdown: (artifact: Artifact) => void
  onExportDrawio: (artifact: Artifact) => void
  onInsertArtifact: (artifact: Artifact) => void
  isMobile?: boolean
}

function createFallbackDrawioXml(title = '综合 Agent 工作台流程') {
  return `<mxfile host="embed.diagrams.net"><diagram name="${title}"><mxGraphModel dx="1200" dy="700" grid="1" gridSize="10" guides="1" tooltips="1" connect="1" arrows="1" fold="1" page="1" pageScale="1" pageWidth="1169" pageHeight="827" math="0" shadow="0"><root><mxCell id="0"/><mxCell id="1" parent="0"/><mxCell id="2" value="用户输入" style="rounded=1;whiteSpace=wrap;html=1;fillColor=#eef2ff;strokeColor=#6366f1;fontColor=#111827;" vertex="1" parent="1"><mxGeometry x="80" y="120" width="140" height="60" as="geometry"/></mxCell><mxCell id="3" value="Agent 编排" style="rounded=1;whiteSpace=wrap;html=1;fillColor=#ecfeff;strokeColor=#06b6d4;fontColor=#111827;" vertex="1" parent="1"><mxGeometry x="300" y="120" width="140" height="60" as="geometry"/></mxCell><mxCell id="4" value="工具执行" style="rounded=1;whiteSpace=wrap;html=1;fillColor=#f0fdf4;strokeColor=#22c55e;fontColor=#111827;" vertex="1" parent="1"><mxGeometry x="520" y="120" width="140" height="60" as="geometry"/></mxCell><mxCell id="5" value="动态产物" style="rounded=1;whiteSpace=wrap;html=1;fillColor=#fff7ed;strokeColor=#f97316;fontColor=#111827;" vertex="1" parent="1"><mxGeometry x="740" y="120" width="140" height="60" as="geometry"/></mxCell><mxCell id="6" value="" style="endArrow=block;html=1;rounded=0;strokeColor=#6366f1;" edge="1" parent="1" source="2" target="3"><mxGeometry relative="1" as="geometry"/></mxCell><mxCell id="7" value="" style="endArrow=block;html=1;rounded=0;strokeColor=#06b6d4;" edge="1" parent="1" source="3" target="4"><mxGeometry relative="1" as="geometry"/></mxCell><mxCell id="8" value="" style="endArrow=block;html=1;rounded=0;strokeColor=#22c55e;" edge="1" parent="1" source="4" target="5"><mxGeometry relative="1" as="geometry"/></mxCell></root></mxGraphModel></diagram></mxfile>`
}

function EmptyArtifact({ activeTool }: { activeTool: ToolKind }) {
  const labels: Record<string, string> = {
    general: '综合任务产物', ppt: 'PPT 演示文稿', doc: '文档', drawio: 'draw.io 图表', excel: '在线表格', image: '图象结果', video: '视频结果', code: '代码结果',
  }
  return (
    <div className="max-w-md text-center text-surface-400">
      <div className="w-24 h-24 bg-white rounded-3xl flex items-center justify-center mx-auto mb-4 shadow-sm border border-surface-100">
        <Sparkles className="w-12 h-12 text-surface-300" />
      </div>
      <p className="text-lg font-semibold text-surface-600 mb-1">这里展示{labels[activeTool] || '智能体产物'}</p>
      <p className="text-sm text-surface-400 leading-relaxed">右侧面板默认可关闭，任务需要时再展开；不同工具会使用不同的预览、编辑和导出能力。</p>
    </div>
  )
}

function normalizeMarkdown(markdown: string) {
  return markdown
    .trim()
    .replace(/^```(?:md|markdown)\s*/i, '')
    .replace(/\s*```$/, '')
}

function MarkdownPreview({ markdown }: { markdown: string }) {
  const content = normalizeMarkdown(markdown)

  return (
    <div className="mx-auto max-w-3xl rounded-2xl border border-surface-200 bg-white p-8 shadow-sm">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          h1: ({ children }) => <h1 className="mb-4 text-3xl font-bold tracking-tight text-surface-950">{children}</h1>,
          h2: ({ children }) => <h2 className="mb-3 mt-8 border-b border-surface-200 pb-2 text-2xl font-semibold text-surface-900">{children}</h2>,
          h3: ({ children }) => <h3 className="mb-2 mt-6 text-lg font-semibold text-surface-900">{children}</h3>,
          h4: ({ children }) => <h4 className="mb-2 mt-4 text-base font-semibold text-surface-800">{children}</h4>,
          p: ({ children }) => <p className="my-3 text-sm leading-7 text-surface-700">{children}</p>,
          strong: ({ children }) => <strong className="font-semibold text-surface-950">{children}</strong>,
          em: ({ children }) => <em className="italic text-surface-800">{children}</em>,
          a: ({ href, children }) => (
            <a
              href={href}
              target="_blank"
              rel="noreferrer"
              className="font-medium text-primary-600 underline decoration-primary-200 underline-offset-4 transition-colors hover:text-primary-700"
            >
              {children}
            </a>
          ),
          ul: ({ children }) => <ul className="my-3 list-disc space-y-2 pl-6 text-sm leading-7 text-surface-700">{children}</ul>,
          ol: ({ children }) => <ol className="my-3 list-decimal space-y-2 pl-6 text-sm leading-7 text-surface-700">{children}</ol>,
          li: ({ children, ...props }) => {
            const hasTaskCheckbox = Array.isArray(props.children) && props.children.some((child: any) => child?.type === 'input')
            return <li className={`${hasTaskCheckbox ? 'flex items-start gap-2 pl-0' : 'pl-1'} marker:text-surface-400`}>{children}</li>
          },
          input: ({ checked, disabled, type }) => {
            if (type !== 'checkbox') return null
            return (
              <input
                type="checkbox"
                checked={checked}
                disabled={disabled ?? true}
                readOnly
                className="mt-1 h-4 w-4 rounded border-surface-300 text-primary-600 focus:ring-primary-500"
              />
            )
          },
          blockquote: ({ children }) => (
            <blockquote className="my-5 rounded-r-2xl border-l-4 border-primary-200 bg-primary-50/70 px-4 py-3 text-sm leading-7 text-surface-700">
              {children}
            </blockquote>
          ),
          hr: () => <hr className="my-8 border-surface-200" />,
          table: ({ children }) => (
            <div className="my-5 overflow-x-auto rounded-2xl border border-surface-200">
              <table className="min-w-full border-collapse bg-white text-sm">{children}</table>
            </div>
          ),
          thead: ({ children }) => <thead className="bg-surface-50 text-surface-800">{children}</thead>,
          tbody: ({ children }) => <tbody>{children}</tbody>,
          tr: ({ children }) => <tr className="border-b border-surface-200 last:border-b-0">{children}</tr>,
          th: ({ children }) => <th className="px-4 py-3 text-left font-semibold">{children}</th>,
          td: ({ children }) => <td className="px-4 py-3 align-top text-surface-700">{children}</td>,
          pre: ({ children }) => <>{children}</>,
          code: ({ className, children }) => {
            const raw = String(children).replace(/\n$/, '')
            const match = /language-([\w-]+)/.exec(className || '')
            const isBlock = Boolean(match) || raw.includes('\n')
            if (isBlock) {
              return (
                <div className="my-5 overflow-hidden rounded-2xl border border-surface-200 shadow-sm">
                  <div className="flex items-center justify-between border-b border-surface-200 bg-surface-50 px-4 py-2 text-[11px] font-medium text-surface-500">
                    <span>{match?.[1] || 'code'}</span>
                    <span>代码块</span>
                  </div>
                  <SyntaxHighlighter
                    language={match?.[1]}
                    style={oneLight}
                    customStyle={{
                      margin: 0,
                      padding: '16px',
                      background: '#fafaf9',
                      fontSize: '12px',
                      lineHeight: '1.7',
                    }}
                    codeTagProps={{ style: { fontFamily: 'SFMono-Regular, ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace' } }}
                    wrapLongLines
                  >
                    {raw}
                  </SyntaxHighlighter>
                </div>
              )
            }
            return (
              <code className="rounded-md bg-surface-100 px-1.5 py-0.5 text-[0.9em] text-surface-900">
                {raw}
              </code>
            )
          },
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  )
}

function DocumentArtifact({ artifact, onExport, onInsert }: { artifact: Artifact, onExport: () => void, onInsert: () => void }) {
  const content = artifact.content || {}
  const isStructured = content.type === 'structured' && Array.isArray(content.sections) && content.sections.length > 0
  const [isExporting, setIsExporting] = useState(false)
  const handleExport = async () => {
    try {
      setIsExporting(true)
      await Promise.resolve(onExport())
    } finally {
      setIsExporting(false)
    }
  }

  return (
    <div className="flex h-full w-full flex-col gap-3">
      {/* 工具栏 */}
      <div className="shrink-0 flex items-center justify-between rounded-2xl border border-surface-200 bg-white px-4 py-3 shadow-sm">
        <div>
          <div className="text-sm font-semibold text-surface-800">{artifact.title || '文档'}</div>
          <div className="text-xs text-surface-400">{isStructured ? 'Word 版式预览 · 导出 DOCX' : 'Markdown 预览 · 导出 DOCX'}</div>
        </div>
        <button
          className="inline-flex items-center gap-1 rounded-full bg-primary-600 px-3 py-1.5 text-xs font-semibold text-white disabled:cursor-not-allowed disabled:opacity-60"
          disabled={isExporting}
          onClick={handleExport}
        >
          <Download className="h-3.5 w-3.5" />{isExporting ? '导出中…' : '导出 DOCX'}
        </button>
        <button
          type="button"
          onClick={onInsert}
          className="inline-flex items-center gap-1.5 rounded-full border border-black/10 bg-white px-3 py-2 text-xs font-semibold text-surface-700 hover:bg-surface-50"
        >
          <MessageSquarePlus className="h-3.5 w-3.5" />
          引用到对话
        </button>
      </div>
      {/* 预览区域 */}
      {isStructured ? (
        <div className="min-h-0 flex-1 overflow-y-auto pb-4">
          <WordPreview content={content} title={artifact.title} />
        </div>
      ) : (
        <MarkdownPreview markdown={content.markdown || '# 文档草稿\n\n暂无内容。'} />
      )}
    </div>
  )
}

function MarkdownArtifact({ artifact, onExport, onInsert }: { artifact: Artifact, onExport: () => void, onInsert: () => void }) {
  const [isExporting, setIsExporting] = useState(false)
  const markdown = artifact.content?.markdown || `# ${artifact.title || 'Markdown 文档'}\n\n暂无内容。`

  const handleExport = async () => {
    try {
      setIsExporting(true)
      await Promise.resolve(onExport())
    } finally {
      setIsExporting(false)
    }
  }

  return (
    <div className="flex h-full w-full flex-col gap-3">
      <div className="shrink-0 flex items-center justify-between rounded-2xl border border-surface-200 bg-white px-4 py-3 shadow-sm">
        <div>
          <div className="text-sm font-semibold text-surface-800">{artifact.title || 'Markdown 文档'}</div>
          <div className="text-xs text-surface-400">Markdown 阅读视图 · 下载 MD</div>
        </div>
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={onInsert}
            className="inline-flex items-center gap-1 rounded-full border border-black/10 bg-white px-3 py-1.5 text-xs font-semibold text-surface-700 hover:bg-surface-50"
          >
            <MessageSquarePlus className="h-3.5 w-3.5" />
            引用
          </button>
          <button
            className="inline-flex items-center gap-1 rounded-full bg-surface-950 px-3 py-1.5 text-xs font-semibold text-white disabled:cursor-not-allowed disabled:opacity-60"
            disabled={isExporting}
            onClick={handleExport}
          >
            <Download className="h-3.5 w-3.5" />{isExporting ? '下载中…' : '下载 MD'}
          </button>
        </div>
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto pb-4">
        <MarkdownPreview markdown={markdown} />
      </div>
    </div>
  )
}

function DrawIoArtifact({ artifact, onUpdate, onInsert }: { artifact: Artifact, onUpdate: (updates: Partial<Artifact>) => void, onInsert: () => void }) {
  const ref = useRef<DrawIoEmbedRef>(null)
  const xml = artifact.content?.xml || createFallbackDrawioXml(artifact.title)
  const [mode, setMode] = useState<'preview' | 'edit'>('preview')

  return (
    <div className="h-full min-h-[520px] overflow-hidden rounded-2xl border border-surface-200 bg-white shadow-sm">
      <div className="flex h-11 items-center justify-between border-b border-surface-100 bg-surface-50 px-3">
        <div className="flex items-center gap-2 text-xs font-semibold text-surface-700">
          <PenTool className="h-4 w-4 text-primary-500" />
          {artifact.title || 'draw.io 画布'}
        </div>
        <div className="flex items-center gap-2 text-[11px] text-surface-400">
          <div className="flex items-center rounded-full border border-surface-200 bg-white p-0.5">
            <button
              className={`inline-flex items-center gap-1 rounded-full px-2 py-1 ${mode === 'preview' ? 'bg-surface-950 text-white' : 'text-surface-500'}`}
              onClick={() => setMode('preview')}
            >
              <Eye className="h-3 w-3" />预览
            </button>
            <button
              className={`inline-flex items-center gap-1 rounded-full px-2 py-1 ${mode === 'edit' ? 'bg-primary-600 text-white' : 'text-surface-500'}`}
              onClick={() => setMode('edit')}
            >
              <Pencil className="h-3 w-3" />编辑
            </button>
          </div>
          {mode === 'edit' && (
            <button className="inline-flex items-center gap-1 rounded-full bg-primary-50 px-2 py-1 text-primary-700" onClick={() => ref.current?.exportDiagram({ format: 'xmlsvg' })}>
              <Save className="h-3 w-3" />保存
            </button>
          )}
          <button
            type="button"
            onClick={onInsert}
            className="inline-flex items-center gap-1 rounded-full border border-surface-200 bg-white px-2 py-1 text-surface-600 hover:bg-surface-50"
          >
            <MessageSquarePlus className="h-3 w-3" />引用
          </button>
        </div>
      </div>
      <div className="h-[calc(100%-44px)]">
        <DrawIoEmbed
          ref={ref}
          xml={xml}
          autosave={mode === 'edit'}
          exportFormat="xmlsvg"
          urlParameters={mode === 'preview'
            ? { chrome: true, nav: true, layers: false, lightbox: false, spin: true }
            : { ui: 'kennedy', spin: true, libraries: true, saveAndExit: false }}
          configuration={mode === 'edit' ? {
            defaultLibraries: 'general;uml;er;bpmn;flowchart;basic;arrows2',
            enabledLibraries: null,
            defaultVertexStyle: { rounded: '1', whiteSpace: 'wrap', html: '1', fillColor: '#dae8fc', strokeColor: '#6c8ebf' },
            defaultEdgeStyle: { edgeStyle: 'orthogonalEdgeStyle', rounded: '1', orthogonalLoop: '1', jetSize: 'auto', html: '1', strokeColor: '#6c8ebf' },
            presetColors: ['dae8fc', 'd5e8d4', 'fff2cc', 'f8cecc', 'e1d5e7', 'ffe6cc', 'f5f5f5', 'ffffff', '000000', '333333', '666666', '999999'],
            defaultColors: ['f5f5f5', 'e6e6e6', 'd9d9d9', 'cccccc', 'b3b3b3', '999999', '808080', '666666', '4d4d4d', '333333', '1a1a1a', '000000'],
            defaultColorSchemes: [
              { fill: '#dae8fc', stroke: '#6c8ebf', font: '#1e3a5f', title: '蓝色' },
              { fill: '#d5e8d4', stroke: '#82b366', font: '#2d5016', title: '绿色' },
              { fill: '#fff2cc', stroke: '#d6b656', font: '#5c4a04', title: '黄色' },
              { fill: '#f8cecc', stroke: '#b85450', font: '#5a1a17', title: '红色' },
              { fill: '#e1d5e7', stroke: '#9673a6', font: '#3d2060', title: '紫色' },
              { fill: '#ffe6cc', stroke: '#d79b00', font: '#5c3a00', title: '橙色' },
            ],
          } : undefined}
          onAutoSave={mode === 'edit' ? ((data) => onUpdate({ content: { ...artifact.content, xml: data.xml || xml }, status: 'ready' })) : undefined}
          onSave={mode === 'edit' ? ((data) => onUpdate({ content: { ...artifact.content, xml: data.xml || xml }, status: 'ready' })) : undefined}
          onExport={mode === 'edit' ? ((data) => onUpdate({ content: { ...artifact.content, preview: data.data }, status: 'ready' })) : undefined}
        />
      </div>
    </div>
  )
}

function SheetArtifact({ artifact, onUpdate, onExport, onInsert }: { artifact: Artifact, onUpdate: (updates: Partial<Artifact>) => void, onExport: () => void, onInsert: () => void }) {
  const tables: Array<{ title?: string; headers?: string[]; rows?: string[][]; summary?: string }> =
    Array.isArray(artifact.content?.tables) && artifact.content.tables.length > 0
      ? artifact.content.tables
      : [{
          title: artifact.title || '默认表',
          headers: Array.isArray(artifact.content?.rows?.[0]) ? artifact.content.rows[0] : ['字段', '说明'],
          rows: Array.isArray(artifact.content?.rows) ? artifact.content.rows.slice(1) : [['暂无数据', '等待 Agent 生成']],
          summary: artifact.content?.summary,
        }]
  const [activeTableIndex, setActiveTableIndex] = useState(0)
  const activeTable = tables[Math.min(activeTableIndex, tables.length - 1)] || tables[0]
  const headers = activeTable?.headers?.length ? activeTable.headers : ['字段', '说明']
  const bodyRows = activeTable?.rows?.length ? activeTable.rows : [['暂无数据', '等待 Agent 生成']]
  const [isExporting, setIsExporting] = useState(false)

  useEffect(() => {
    setActiveTableIndex((current) => Math.min(current, Math.max(0, tables.length - 1)))
  }, [tables.length])

  const handleExport = async () => {
    try {
      setIsExporting(true)
      await Promise.resolve(onExport())
    } finally {
      setIsExporting(false)
    }
  }

  const updateHeaderCell = (c: number, value: string) => {
    const nextTables = tables.map((table, index) => {
      if (index !== activeTableIndex) return table
      const nextHeaders = [...(table.headers || headers)]
      nextHeaders[c] = value
      return { ...table, headers: nextHeaders }
    })
    onUpdate({ content: { ...artifact.content, tables: nextTables }, status: 'ready' })
  }

  const updateBodyCell = (r: number, c: number, value: string) => {
    const nextTables = tables.map((table, index) => {
      if (index !== activeTableIndex) return table
      const nextRows = (table.rows || bodyRows).map((row) => [...row])
      nextRows[r][c] = value
      return { ...table, rows: nextRows }
    })
    onUpdate({ content: { ...artifact.content, tables: nextTables }, status: 'ready' })
  }

  return (
    <div className="overflow-hidden rounded-2xl border border-surface-200 bg-white shadow-sm">
      <div className="flex h-11 items-center justify-between border-b border-surface-100 bg-emerald-50 px-3 text-xs text-emerald-700">
        <span className="font-semibold">{artifact.title || '在线 Excel 工作区'}</span>
        <div className="flex items-center gap-2">
          <span>可编辑表格 / ExcelJS XLSX 导出</span>
          <button
            type="button"
            onClick={onInsert}
            className="inline-flex items-center gap-1 rounded-full border border-emerald-200 bg-white px-2.5 py-1 text-emerald-700 hover:bg-emerald-50"
          >
            <MessageSquarePlus className="h-3 w-3" />引用
          </button>
          <button
            className="inline-flex items-center gap-1 rounded-full bg-emerald-600 px-2.5 py-1 text-white disabled:cursor-not-allowed disabled:opacity-60"
            disabled={isExporting}
            onClick={handleExport}
          >
            <Download className="h-3 w-3" />{isExporting ? '导出中' : '导出 XLSX'}
          </button>
        </div>
      </div>
      {tables.length > 1 && (
        <div className="flex flex-wrap gap-2 border-b border-surface-100 bg-white px-4 py-3">
          {tables.map((table, index) => (
            <button
              key={`${table.title || 'sheet'}-${index}`}
              type="button"
              onClick={() => setActiveTableIndex(index)}
              className={`rounded-full px-3 py-1.5 text-xs transition-colors ${
                index === activeTableIndex ? 'bg-emerald-600 text-white' : 'bg-emerald-50 text-emerald-700 hover:bg-emerald-100'
              }`}
            >
              {table.title || `表 ${index + 1}`}
            </button>
          ))}
        </div>
      )}
      <div className="overflow-auto p-4">
        {activeTable?.summary && (
          <div className="mb-3 rounded-2xl bg-surface-50 px-3 py-2 text-xs leading-6 text-surface-500">
            {activeTable.summary}
          </div>
        )}
        <table className="w-full min-w-[680px] border-collapse text-sm">
          <thead>
            <tr>
              {headers.map((header, c) => (
                <th key={`h-${c}`} className="border border-surface-200 bg-surface-100 p-0 font-semibold text-surface-700">
                  <input
                    className="h-full w-full bg-transparent px-3 py-2 outline-none focus:bg-primary-50"
                    value={header}
                    onChange={(e) => updateHeaderCell(c, e.target.value)}
                  />
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {bodyRows.map((row, r) => (
              <tr key={r}>
                {row.map((cell, c) => (
                  <td key={`${r}-${c}`} className="border border-surface-200 p-0 text-surface-600">
                    <input
                      className="h-full w-full bg-transparent px-3 py-2 outline-none focus:bg-primary-50"
                      value={cell}
                      onChange={(e) => updateBodyCell(r, c, e.target.value)}
                    />
                  </td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  )
}

function downloadFromUrl(url: string, filename: string) {
  const link = document.createElement('a')
  link.href = url
  link.download = filename
  link.target = '_blank'
  document.body.appendChild(link)
  link.click()
  link.remove()
}

function ImageArtifact({ artifact, onInsert }: { artifact: Artifact, onInsert: () => void }) {
  const prompt = artifact.content?.prompt || '等待图象 Agent 生成提示词或图片。'
  const images: string[] = artifact.content?.images || []
  const variants: Array<{ style?: string; prompt?: string; url?: string }> = artifact.content?.variants || artifact.content?.data?.prompts || []
  const [previewImage, setPreviewImage] = useState<string | null>(null)

  useEffect(() => {
    if (!previewImage) return
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === 'Escape') setPreviewImage(null)
    }
    window.addEventListener('keydown', handleKeyDown)
    return () => window.removeEventListener('keydown', handleKeyDown)
  }, [previewImage])

  return (
    <div className="w-full max-w-3xl space-y-4">
      {images.length > 0 ? (
        <div className="grid grid-cols-2 gap-4">
          {images.map((src, index) => (
            <div key={src} className="space-y-2">
              <button
                type="button"
                onClick={() => setPreviewImage(src)}
                className="block w-full cursor-zoom-in rounded-3xl text-left focus:outline-none focus:ring-2 focus:ring-primary-500 focus:ring-offset-2"
                aria-label={`放大查看图片 ${index + 1}`}
              >
                <img src={src} className="aspect-video w-full rounded-3xl border border-surface-200 object-cover" />
              </button>
              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={() => onInsert()}
                  className="inline-flex items-center gap-1 rounded-full border border-black/10 bg-white px-3 py-1.5 text-xs font-semibold text-surface-700 hover:bg-surface-50"
                >
                  <MessageSquarePlus className="h-3.5 w-3.5" />
                  引用
                </button>
                <button
                  type="button"
                  onClick={() => downloadFromUrl(src, `${artifact.title || 'image'}-${index + 1}.png`)}
                  className="inline-flex items-center gap-1 rounded-full border border-black/10 bg-white px-3 py-1.5 text-xs font-semibold text-surface-700 hover:bg-surface-50"
                >
                  <Download className="h-3.5 w-3.5" />
                  下载
                </button>
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="aspect-video rounded-3xl border border-surface-200 bg-gradient-to-br from-violet-100 via-white to-sky-100 flex items-center justify-center text-surface-400"><Image className="h-10 w-10" /></div>
      )}
      {previewImage && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center bg-black/80 p-4 backdrop-blur-sm"
          onClick={() => setPreviewImage(null)}
          role="dialog"
          aria-modal="true"
        >
          <button
            type="button"
            onClick={() => setPreviewImage(null)}
            className="absolute right-4 top-4 flex h-10 w-10 items-center justify-center rounded-full bg-white/90 text-2xl leading-none text-surface-900 shadow-lg hover:bg-white"
            aria-label="关闭图片预览"
          >
            ×
          </button>
          <img
            src={previewImage}
            className="max-h-[90vh] max-w-[92vw] rounded-2xl object-contain shadow-2xl"
            onClick={(event) => event.stopPropagation()}
          />
        </div>
      )}
      {artifact.content?.description && (
        <div className="rounded-2xl border border-surface-200 bg-white p-4 text-sm leading-7 text-surface-600">
          <div className="mb-1 text-xs font-semibold text-surface-400">创意说明</div>
          {artifact.content.description}
        </div>
      )}
      <div className="rounded-2xl border border-surface-200 bg-white p-4 text-sm leading-7 text-surface-600">
        <div className="mb-1 text-xs font-semibold text-surface-400">图象提示词</div>
        {prompt}
      </div>
      {variants.length > 0 && (
        <div className="grid gap-3 md:grid-cols-3">
          {variants.map((variant, index) => (
            <div key={`${variant.style || 'style'}-${index}`} className="rounded-2xl border border-surface-200 bg-white p-4 text-xs leading-6 text-surface-600 shadow-sm">
              <div className="mb-2 text-sm font-semibold text-surface-900">{variant.style || `风格 ${index + 1}`}</div>
              <div className="line-clamp-6">{variant.prompt || '暂无提示词'}</div>
            </div>
          ))}
        </div>
      )}
    </div>
  )
}

function VideoArtifact({ artifact, onInsert }: { artifact: Artifact, onInsert: () => void }) {
  const videoUrl = artifact.content?.video_url || ''
  const prompt = artifact.content?.prompt || '等待视频 Agent 生成镜头脚本。'
  const shots = artifact.content?.shots as Array<{
    index: number; title: string; description: string;
    prompt: string; mode: string; seconds: number;
    first_frame?: string | null; last_frame?: string | null;
    reference_images?: string[]; transition?: string | null;
  }> | undefined
  const isStoryboard = !videoUrl && shots && shots.length > 0
  return (
    <div className="w-full max-w-3xl space-y-4">
      {videoUrl ? (
        <div className="overflow-hidden rounded-3xl border border-surface-200 bg-black shadow-sm">
          <video src={videoUrl} controls className="aspect-video w-full bg-black" />
        </div>
      ) : isStoryboard ? (
        <div className="rounded-3xl border border-surface-200 bg-gradient-to-br from-violet-50 via-white to-sky-50 p-6">
          <div className="mb-4 flex items-center gap-2">
            <Clapperboard className="h-5 w-5 text-violet-500" />
            <span className="text-sm font-semibold text-surface-700">分镜方案 · {shots.length} 个镜头 · 约 {artifact.content?.total_seconds || 0}s</span>
          </div>
          <div className="space-y-3">
            {shots.map((shot) => (
              <div key={shot.index} className="rounded-2xl border border-surface-200 bg-white p-4">
                <div className="mb-2 flex items-center justify-between">
                  <span className="text-sm font-semibold text-surface-900">镜头{shot.index}：{shot.title}</span>
                  <div className="flex items-center gap-2">
                    <span className="rounded-full bg-violet-50 px-2.5 py-1 text-xs font-medium text-violet-600">{shot.mode}</span>
                    <span className="rounded-full bg-surface-50 px-2.5 py-1 text-xs font-medium text-surface-500">{shot.seconds}s</span>
                  </div>
                </div>
                <p className="mb-2 text-sm leading-6 text-surface-600">{shot.description}</p>
                <div className="rounded-lg bg-surface-50 px-3 py-2 text-xs leading-5 text-surface-500">
                  <span className="font-medium text-surface-400">Prompt: </span>{shot.prompt}
                </div>
                {shot.transition && (
                  <div className="mt-2 text-xs text-surface-400">→ 转场：{shot.transition}</div>
                )}
              </div>
            ))}
          </div>
        </div>
      ) : (
        <div className="aspect-video rounded-3xl border border-surface-200 bg-gradient-to-br from-rose-100 via-white to-orange-100 flex items-center justify-center text-surface-400">
          <Clapperboard className="h-10 w-10" />
        </div>
      )}
      <div className="flex flex-wrap items-center gap-2">
        <button
          type="button"
          onClick={onInsert}
          className="inline-flex items-center gap-1.5 rounded-full border border-black/10 bg-white px-3 py-2 text-xs font-semibold text-surface-700 hover:bg-surface-50"
        >
          <MessageSquarePlus className="h-3.5 w-3.5" />
          引用到对话
        </button>
        {videoUrl && (
          <button
            type="button"
            onClick={() => downloadFromUrl(videoUrl, `${artifact.title || 'video'}.mp4`)}
            className="inline-flex items-center gap-1.5 rounded-full bg-surface-950 px-3 py-2 text-xs font-semibold text-white hover:bg-surface-800"
          >
            <Download className="h-3.5 w-3.5" />
            下载 MP4
          </button>
        )}
        {artifact.content?.size && <span className="rounded-full bg-white px-3 py-1.5 text-xs font-medium text-surface-600 ring-1 ring-black/[0.05]">分辨率：{artifact.content.size}</span>}
        {artifact.content?.seconds && <span className="rounded-full bg-white px-3 py-1.5 text-xs font-medium text-surface-600 ring-1 ring-black/[0.05]">时长：{artifact.content.seconds}s</span>}
        {artifact.content?.aspect_ratio && <span className="rounded-full bg-white px-3 py-1.5 text-xs font-medium text-surface-600 ring-1 ring-black/[0.05]">比例：{artifact.content.aspect_ratio}</span>}
      </div>
      {artifact.content?.description && (
        <div className="rounded-2xl border border-surface-200 bg-white p-4 text-sm leading-7 text-surface-600">
          <div className="mb-1 text-xs font-semibold text-surface-400">成片说明</div>
          {artifact.content.description}
        </div>
      )}
      {artifact.content?.fallback_reason && (
        <div className="rounded-2xl border border-amber-200 bg-amber-50 p-4 text-sm leading-7 text-amber-800">
          <div className="mb-1 text-xs font-semibold text-amber-600">本地兜底说明</div>
          远程视频服务暂不可用，已改用本地 MP4 合成。原因：{artifact.content.fallback_reason}
        </div>
      )}
      {!isStoryboard && (
        <div className="rounded-2xl border border-surface-200 bg-white p-4 text-sm leading-7 text-surface-600">
          <div className="mb-1 text-xs font-semibold text-surface-400">视频提示词</div>
          {prompt}
        </div>
      )}
    </div>
  )
}

function ChartArtifact({ artifact }: { artifact: Artifact }) {
  const content = artifact.content || {}
  return (
    <div className="flex h-full w-full flex-col gap-3">
      <div className="shrink-0 rounded-2xl border border-surface-200 bg-white px-4 py-3 shadow-sm">
        <div className="text-sm font-semibold text-surface-900">{artifact.title || content.title || '数据图表'}</div>
        <div className="mt-1 text-xs text-surface-500">ECharts 动态渲染 · 支持普通对话、数据分析和 PPT 嵌入</div>
      </div>
      {content.summary && (
        <div className="shrink-0 rounded-2xl border border-primary-100 bg-primary-50 px-4 py-3 text-sm leading-7 text-primary-900">
          {content.summary}
        </div>
      )}
      <div className="min-h-[420px] flex-1 overflow-hidden rounded-2xl border border-surface-200 bg-white p-4 shadow-sm">
        <EChartsView
          title={artifact.title || content.title}
          chartType={content.chart_type || content.type}
          chartData={content.chart_data || content.data || content}
          option={content.option}
        />
      </div>
    </div>
  )
}

function SearchArtifact({ artifact }: { artifact: Artifact }) {
  const provider = artifact.content?.provider_label || artifact.content?.provider || '未知来源'
  const query = artifact.content?.query || artifact.title
  const providersTried: string[] = artifact.content?.providers_tried || []
  const results: Array<{ title: string; url: string; snippet?: string; source?: string }> = artifact.content?.results || []
  const chain = providersTried.join(' -> ')

  return (
    <div className="flex h-full w-full flex-col gap-3">
      <div className="rounded-2xl border border-surface-200 bg-white px-4 py-4 shadow-sm">
        <div className="flex flex-wrap items-start justify-between gap-3">
          <div>
            <div className="text-sm font-semibold text-surface-900">{query}</div>
            <div className="mt-1 text-xs text-surface-500">搜索来源：{provider}</div>
          </div>
          <div className="rounded-2xl bg-surface-50 px-3 py-2 text-right">
            <div className="text-[11px] text-surface-400">结果数量</div>
            <div className="text-lg font-semibold text-surface-900">{results.length}</div>
          </div>
        </div>
        {providersTried.length > 0 && (
          <div className="mt-3">
            <div className="mb-2 text-[11px] font-medium text-surface-400">检索链路</div>
            <div className="mb-2 text-xs text-surface-500">{chain}</div>
            <div className="flex flex-wrap gap-2">
            {providersTried.map((item) => (
              <span key={item} className="rounded-full bg-surface-100 px-2.5 py-1 text-[11px] font-medium text-surface-600">{item}</span>
            ))}
          </div>
          </div>
        )}
      </div>
      <div className="min-h-0 flex-1 space-y-3 overflow-y-auto pb-4">
        {results.length > 0 ? results.map((item, index) => (
          <a
            key={`${item.url}-${index}`}
            href={item.url}
            target="_blank"
            rel="noreferrer"
            className="block rounded-2xl border border-surface-200 bg-white p-4 shadow-sm transition-all hover:-translate-y-0.5 hover:border-surface-300 hover:shadow-md"
          >
            <div className="mb-2 flex items-center gap-2 text-[11px] text-surface-500">
              <span className="rounded-full bg-surface-100 px-2 py-1 font-medium">{item.source || provider}</span>
              <span>结果 {index + 1}</span>
            </div>
            <div className="text-sm font-semibold leading-6 text-surface-900">{item.title}</div>
            {item.snippet && (
              <div className="mt-2 line-clamp-4 text-xs leading-6 text-surface-600">{item.snippet}</div>
            )}
            <div className="mt-3 truncate text-xs text-primary-600">{item.url}</div>
          </a>
        )) : (
          <div className="rounded-2xl border border-dashed border-surface-200 bg-white/70 px-4 py-8 text-center text-sm text-surface-500">
            当前没有可展示的搜索结果。
          </div>
        )}
      </div>
    </div>
  )
}

function CodeArtifact({ artifact }: { artifact: Artifact }) {
  const steps: string[] = artifact.content?.steps || []
  return (
    <div className="w-full max-w-3xl rounded-2xl border border-surface-200 bg-slate-950 p-5 text-slate-100 shadow-sm">
      <div className="mb-3 flex items-center gap-2 text-sm font-semibold"><Code2 className="h-4 w-4" />{artifact.title}</div>
      <ol className="space-y-2 text-sm text-slate-300">
        {steps.map((step, i) => <li key={step}>{i + 1}. {step}</li>)}
      </ol>
    </div>
  )
}

function MixedArtifact({ artifact }: { artifact: Artifact }) {
  const markdown = artifact.content?.markdown || `# ${artifact.title || '综合办公产物'}\n\n暂无内容。`
  const rows: string[][] = artifact.content?.rows || []
  const needs: string[] = artifact.content?.needs || []
  return (
    <div className="w-full max-w-4xl space-y-4">
      <div className="rounded-2xl border border-primary-100 bg-primary-50/70 p-4 text-sm text-primary-900">
        <div className="mb-2 flex items-center gap-2 font-semibold"><Sparkles className="h-4 w-4" />综合办公 Agent 工作台</div>
        <div className="text-xs leading-6 text-primary-800">已识别产物类型：{needs.length ? needs.join(' / ') : 'document'}。这个 Artifact 是任务总控视图，用于统一目标、行动项和后续交付。</div>
      </div>
      <MarkdownPreview markdown={markdown} />
      {rows.length > 0 && (
        <div className="overflow-hidden rounded-2xl border border-surface-200 bg-white shadow-sm">
          <div className="border-b border-surface-100 bg-surface-50 px-4 py-3 text-xs font-semibold text-surface-700">行动项 / 交付清单</div>
          <div className="overflow-auto p-4">
            <table className="w-full min-w-[760px] border-collapse text-sm">
              <tbody>
                {rows.map((row, r) => (
                  <tr key={r}>
                    {row.map((cell, c) => (
                      <td key={`${r}-${c}`} className={`border border-surface-200 px-3 py-2 ${r === 0 ? 'bg-surface-100 font-semibold text-surface-700' : 'text-surface-600'}`}>{cell}</td>
                    ))}
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  )
}

function ArtifactBody({ artifact, activeTool, onUpdate, onExportExcel, onExportDocx, onExportMarkdown, onInsertArtifact }: { artifact: Artifact | null, activeTool: ToolKind, onUpdate: (id: string, updates: Partial<Artifact>) => void, onExportExcel: (artifact: Artifact) => void, onExportDocx: (artifact: Artifact) => void, onExportMarkdown: (artifact: Artifact) => void, onInsertArtifact: (artifact: Artifact) => void }) {
  if (!artifact) return <EmptyArtifact activeTool={activeTool} />
  if (artifact.kind === 'document') return <DocumentArtifact artifact={artifact} onExport={() => onExportDocx(artifact)} onInsert={() => onInsertArtifact(artifact)} />
  if (artifact.kind === 'markdown') return <MarkdownArtifact artifact={artifact} onExport={() => onExportMarkdown(artifact)} onInsert={() => onInsertArtifact(artifact)} />
  if (artifact.kind === 'drawio') return <DrawIoArtifact artifact={artifact} onUpdate={(updates) => onUpdate(artifact.id, updates)} onInsert={() => onInsertArtifact(artifact)} />
  if (artifact.kind === 'sheet') return <SheetArtifact artifact={artifact} onUpdate={(updates) => onUpdate(artifact.id, updates)} onExport={() => onExportExcel(artifact)} onInsert={() => onInsertArtifact(artifact)} />
  if (artifact.kind === 'image') return <ImageArtifact artifact={artifact} onInsert={() => onInsertArtifact(artifact)} />
  if (artifact.kind === 'video') return <VideoArtifact artifact={artifact} onInsert={() => onInsertArtifact(artifact)} />
  if (artifact.kind === 'chart') return <ChartArtifact artifact={artifact} />
  if (artifact.kind === 'search') return <SearchArtifact artifact={artifact} />
  if (artifact.kind === 'code') return <CodeArtifact artifact={artifact} />
  if (artifact.kind === 'mixed') return <MixedArtifact artifact={artifact} />
  return <EmptyArtifact activeTool={activeTool} />
}

export function ArtifactPanel({
  activeTool,
  project,
  slides,
  currentSlideIndex,
  pptProgress,
  isGeneratingPpt,
  isOpen,
  isWide,
  onOpenChange,
  onWideChange,
  onSelectSlide,
  onExportPpt,
  onPresent,
  messages,
  activeArtifact,
  artifacts,
  onSelectArtifact,
  onUpdateArtifact,
  onExportExcel,
  onExportDocx,
  onExportMarkdown,
  onExportDrawio,
  onInsertArtifact,
  isMobile,
}: ArtifactPanelProps) {
  const [panelWidth, setPanelWidth] = useState(isWide ? 760 : 560)
  const draggingRef = useRef(false)
  const artifactTurnGroups = useMemo(() => groupArtifactsByTurn(artifacts, messages), [artifacts, messages])
  const activeArtifactTurn = findArtifactTurnGroup(activeArtifact?.id || null, artifactTurnGroups)

  useEffect(() => {
    if (!draggingRef.current) setPanelWidth(isWide ? 760 : 560)
  }, [isWide])

  useEffect(() => {
    const handleMove = (event: MouseEvent) => {
      if (!draggingRef.current) return
      const next = Math.min(980, Math.max(420, window.innerWidth - event.clientX))
      setPanelWidth(next)
    }
    const handleUp = () => { draggingRef.current = false }
    window.addEventListener('mousemove', handleMove)
    window.addEventListener('mouseup', handleUp)
    return () => {
      window.removeEventListener('mousemove', handleMove)
      window.removeEventListener('mouseup', handleUp)
    }
  }, [])

  if (!isOpen) return null

  const titleMap: Record<string, string> = {
    general: '动态成果展示', ppt: 'PPT 预览', doc: '文档预览', drawio: 'draw.io 画布', excel: '在线 Excel', image: '图象结果', video: '视频结果', code: '代码结果', search: '搜索结果',
  }
  const artifactKindLabel: Record<string, string> = {
    document: 'Word',
    markdown: 'MD',
    drawio: 'draw.io',
    sheet: 'Excel',
    ppt: 'PPT',
    image: '图片',
    video: '视频',
    chart: '图表',
    search: '搜索',
    code: '代码',
    mixed: '综合',
  }

  const effectiveTool = activeArtifact?.tool_kind || activeTool
  const headerTitle = activeArtifact?.title || titleMap[effectiveTool] || '成果展示'
  const canExportActiveArtifact = activeArtifact?.kind === 'document' || activeArtifact?.kind === 'markdown' || activeArtifact?.kind === 'sheet' || activeArtifact?.kind === 'drawio'

  // 移动端全屏覆盖模式
  if (isMobile) {
    return (
      <div className="flex h-full w-full flex-col bg-white">
        {/* 移动端顶栏 */}
        <div className="flex h-14 shrink-0 items-center justify-between border-b border-surface-100 bg-white/90 px-3">
          <div className="flex min-w-0 items-center gap-2">
            <div className="flex h-8 w-8 shrink-0 items-center justify-center rounded-xl bg-primary-50 text-primary-600">
              <Layers3 className="h-4 w-4" />
            </div>
            <div className="min-w-0">
              <div className="truncate text-sm font-semibold text-surface-800">{headerTitle}</div>
              {activeArtifactTurn && <div className="truncate text-[11px] text-surface-400">{activeArtifactTurn.title}</div>}
            </div>
          </div>
          <div className="flex shrink-0 items-center gap-1.5">
            {effectiveTool === 'ppt' && (
              <>
                <button className="btn-secondary h-8 px-2.5 text-xs" disabled={!project} onClick={onPresent}>演示</button>
                <button className="btn-secondary h-8 px-2.5 text-xs" disabled={!project} onClick={onExportPpt}><Download className="h-3.5 w-3.5" /></button>
              </>
            )}
            {canExportActiveArtifact && activeArtifact?.kind === 'document' && (
              <button className="btn-secondary h-8 px-2.5 text-xs" onClick={() => onExportDocx(activeArtifact)}><Download className="h-3.5 w-3.5" /></button>
            )}
            {canExportActiveArtifact && activeArtifact?.kind === 'markdown' && (
              <button className="btn-secondary h-8 px-2.5 text-xs" onClick={() => onExportMarkdown(activeArtifact)}><Download className="h-3.5 w-3.5" /></button>
            )}
            {canExportActiveArtifact && activeArtifact?.kind === 'sheet' && (
              <button className="btn-secondary h-8 px-2.5 text-xs" onClick={() => onExportExcel(activeArtifact)}><Download className="h-3.5 w-3.5" /></button>
            )}
            {canExportActiveArtifact && activeArtifact?.kind === 'drawio' && (
              <button className="btn-secondary h-8 px-2.5 text-xs" onClick={() => onExportDrawio(activeArtifact)}><Download className="h-3.5 w-3.5" /></button>
            )}
            <button className="btn-ghost h-8 px-2" onClick={() => onOpenChange(false)} title="关闭">关闭</button>
          </div>
        </div>

        {/* 产物切换区 */}
        {artifacts.length > 0 && (
          <div className="shrink-0 border-b border-surface-100 bg-white/80 px-3 py-2">
            <div className="flex gap-2 overflow-x-auto pb-1">
              {artifactTurnGroups.map((group) =>
                group.artifacts.map((artifact) => (
                  <button
                    key={artifact.id}
                    onClick={() => onSelectArtifact(artifact.id)}
                    className={`shrink-0 rounded-full px-3 py-1 text-xs ${activeArtifact?.id === artifact.id ? 'bg-primary-600 text-white' : 'bg-white text-surface-500 hover:bg-surface-200'}`}
                    title={artifact.title}
                  >
                    {(artifactKindLabel[artifact.kind] || artifact.kind)} · {artifact.title.slice(0, 12)}
                  </button>
                ))
              )}
            </div>
          </div>
        )}

        {/* PPT 生成进度 */}
        {effectiveTool === 'ppt' && isGeneratingPpt && pptProgress && pptProgress.total > 0 && (
          <div className="shrink-0 border-b border-surface-200 bg-amber-50/80 px-4 py-3">
            <div className="flex items-center justify-between gap-2">
              <div className="text-sm font-medium text-amber-900">生成中 {pptProgress.current}/{pptProgress.total}</div>
              <div className="text-xs text-amber-700">已生成 {slides.length} 页</div>
            </div>
            <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-amber-100">
              <div className="h-full rounded-full bg-amber-500 transition-all" style={{ width: `${Math.max(6, Math.min(100, (pptProgress.current / pptProgress.total) * 100))}%` }} />
            </div>
          </div>
        )}

        {/* 内容区 */}
        <div className="min-h-0 flex-1 overflow-hidden flex flex-col">
          {effectiveTool === 'ppt' && <Toolbar />}
          <div className={`flex-1 overflow-auto flex ${effectiveTool === 'ppt' ? 'items-center justify-center p-2' : 'items-start justify-center p-3'}`}>
            {effectiveTool === 'ppt' ? (
              slides.length > 0 ? <SlidePreview slide={slides[currentSlideIndex]} layout="16x9" /> : <EmptyArtifact activeTool={effectiveTool} />
            ) : (
              <ArtifactBody artifact={activeArtifact} activeTool={effectiveTool} onUpdate={onUpdateArtifact} onExportExcel={onExportExcel} onExportDocx={onExportDocx} onExportMarkdown={onExportMarkdown} onInsertArtifact={onInsertArtifact} />
            )}
          </div>
        </div>
      </div>
    )
  }

  return (
    <aside
      className="relative shrink-0 overflow-hidden border-l border-black/10 bg-white shadow-[0_0_45px_rgba(24,24,27,0.10)] flex flex-col transition-[width]"
      style={{ width: panelWidth }}
    >
      <button
        type="button"
        aria-label="拖拽调整成果区宽度"
        onMouseDown={(event) => {
          event.preventDefault()
          draggingRef.current = true
        }}
        className="absolute left-0 top-0 z-20 flex h-full w-2 cursor-col-resize items-center justify-center bg-transparent text-surface-300 hover:bg-surface-100 hover:text-surface-700"
      >
        <GripVertical className="h-4 w-4" />
      </button>
      <div className="h-14 shrink-0 border-b border-surface-100 bg-white/90 px-4 flex items-center justify-between">
        <div className="flex items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-primary-50 text-primary-600">
            <Layers3 className="h-4 w-4" />
          </div>
          <div>
            <div className="text-sm font-semibold text-surface-800">{headerTitle}</div>
            <div className="text-[11px] text-surface-400">
              {activeArtifactTurn ? `${activeArtifactTurn.title} · ` : ''}随 Agent 产物动态展开 · 可关闭 · 可编辑
            </div>
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          {effectiveTool === 'ppt' && (
            <>
              <button className="btn-secondary h-8 px-2.5 text-xs" disabled={!project} onClick={onPresent}>演示</button>
              <button className="btn-secondary h-8 px-2.5 text-xs" disabled={!project} onClick={onExportPpt}><Download className="h-3.5 w-3.5" />导出</button>
            </>
          )}
          {canExportActiveArtifact && activeArtifact?.kind === 'document' && (
            <button className="btn-secondary h-8 px-2.5 text-xs" onClick={() => onExportDocx(activeArtifact)}>
              <Download className="h-3.5 w-3.5" />
              导出 DOCX
            </button>
          )}
          {canExportActiveArtifact && activeArtifact?.kind === 'markdown' && (
            <button className="btn-secondary h-8 px-2.5 text-xs" onClick={() => onExportMarkdown(activeArtifact)}>
              <Download className="h-3.5 w-3.5" />
              下载 MD
            </button>
          )}
          {canExportActiveArtifact && activeArtifact?.kind === 'sheet' && (
            <button className="btn-secondary h-8 px-2.5 text-xs" onClick={() => onExportExcel(activeArtifact)}>
              <Download className="h-3.5 w-3.5" />
              导出 XLSX
            </button>
          )}
          {canExportActiveArtifact && activeArtifact?.kind === 'drawio' && (
            <button className="btn-secondary h-8 px-2.5 text-xs" onClick={() => onExportDrawio(activeArtifact)}>
              <Download className="h-3.5 w-3.5" />
              下载 draw.io
            </button>
          )}
          <button className="btn-ghost h-8 px-2" onClick={() => onWideChange(!isWide)} title="切换宽度">
            {isWide ? <Minimize2 className="h-4 w-4" /> : <Maximize2 className="h-4 w-4" />}
          </button>
          <button className="btn-ghost h-8 px-2" onClick={() => onOpenChange(false)} title="关闭右侧面板">关闭</button>
        </div>
      </div>

      {artifacts.length > 0 && (
        <div className="shrink-0 border-b border-surface-100 bg-white/80 px-3 py-2">
          <div className="space-y-2 overflow-y-auto">
            {artifactTurnGroups.map((group) => (
              <div key={group.key} className="rounded-2xl border border-surface-100 bg-surface-50/70 px-2.5 py-2">
                <div className="mb-2 flex items-center justify-between gap-2 px-1">
                  <div className="rounded-full bg-white px-2.5 py-1 text-[11px] font-semibold text-surface-700 ring-1 ring-black/[0.05]">
                    {group.title}
                  </div>
                  <div className="text-[11px] text-surface-400">{group.timeLabel}</div>
                </div>
                <div className="flex gap-2 overflow-x-auto">
                  {group.artifacts.map((artifact) => (
                    <button
                      key={artifact.id}
                      onClick={() => onSelectArtifact(artifact.id)}
                      className={`shrink-0 rounded-full px-3 py-1 text-xs ${activeArtifact?.id === artifact.id ? 'bg-primary-600 text-white' : 'bg-white text-surface-500 hover:bg-surface-200'}`}
                      title={artifact.title}
                    >
                      {(artifactKindLabel[artifact.kind] || artifact.kind)} · {artifact.title.slice(0, 16)}
                    </button>
                  ))}
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="flex-1 overflow-hidden bg-surface-100 flex">
        {effectiveTool === 'ppt' && slides.length > 0 && (
          <aside className="w-52 shrink-0 overflow-y-auto border-r border-surface-200 bg-white/90">
            <SlideList slides={slides} currentIndex={currentSlideIndex} onSelect={onSelectSlide} />
          </aside>
        )}

        <div className="min-w-0 flex-1 overflow-hidden flex flex-col">
          {effectiveTool === 'ppt' && <Toolbar />}
          {effectiveTool === 'ppt' && isGeneratingPpt && pptProgress && pptProgress.total > 0 && (
            <div className="shrink-0 border-b border-surface-200 bg-amber-50/80 px-6 py-3">
              <div className="flex items-center justify-between gap-3">
                <div className="text-sm font-medium text-amber-900">
                  正在生成第 {Math.min(pptProgress.current, pptProgress.total)} / {pptProgress.total} 页
                </div>
                <div className="text-xs text-amber-700">
                  已生成 {slides.length} 页，预览会实时更新
                </div>
              </div>
              <div className="mt-2 h-1.5 overflow-hidden rounded-full bg-amber-100">
                <div
                  className="h-full rounded-full bg-amber-500 transition-all"
                  style={{ width: `${Math.max(6, Math.min(100, (pptProgress.current / pptProgress.total) * 100))}%` }}
                />
              </div>
            </div>
          )}
          <div className={`flex-1 overflow-auto p-6 flex ${effectiveTool === 'ppt' ? 'items-center justify-center' : 'items-start justify-center'}`}>
            {effectiveTool === 'ppt' ? (
              slides.length > 0 ? <SlidePreview slide={slides[currentSlideIndex]} layout="16x9" /> : <EmptyArtifact activeTool={effectiveTool} />
            ) : (
              <ArtifactBody artifact={activeArtifact} activeTool={effectiveTool} onUpdate={onUpdateArtifact} onExportExcel={onExportExcel} onExportDocx={onExportDocx} onExportMarkdown={onExportMarkdown} onInsertArtifact={onInsertArtifact} />
            )}
          </div>
        </div>

      </div>
    </aside>
  )
}
