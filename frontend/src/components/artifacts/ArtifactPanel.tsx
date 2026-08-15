import { Code2, Download, Eye, GripVertical, Image, Layers3, Maximize2, Minimize2, PenTool, Pencil, Save, Sparkles } from 'lucide-react'
import { useEffect, useRef, useState } from 'react'
import { DrawIoEmbed, type DrawIoEmbedRef } from '@/lib/react-drawio'
import { SlideList } from '@/components/slides/SlideList'
import { SlidePreview } from '@/components/preview/SlidePreview'
import { Toolbar } from '@/components/toolbar/Toolbar'
import { WordPreview } from '@/components/artifacts/WordPreview'
import type { Artifact, PPTProject, Slide, ToolKind } from '@/types'

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
  activeArtifact: Artifact | null
  artifacts: Artifact[]
  onSelectArtifact: (id: string) => void
  onUpdateArtifact: (id: string, updates: Partial<Artifact>) => void
  onExportExcel: (artifact: Artifact) => void
  onExportDocx: (artifact: Artifact) => void
}

function createFallbackDrawioXml(title = '综合 Agent 工作台流程') {
  return `<mxfile host="embed.diagrams.net"><diagram name="${title}"><mxGraphModel dx="1200" dy="700" grid="1" gridSize="10" guides="1" tooltips="1" connect="1" arrows="1" fold="1" page="1" pageScale="1" pageWidth="1169" pageHeight="827" math="0" shadow="0"><root><mxCell id="0"/><mxCell id="1" parent="0"/><mxCell id="2" value="用户输入" style="rounded=1;whiteSpace=wrap;html=1;fillColor=#eef2ff;strokeColor=#6366f1;fontColor=#111827;" vertex="1" parent="1"><mxGeometry x="80" y="120" width="140" height="60" as="geometry"/></mxCell><mxCell id="3" value="Agent 编排" style="rounded=1;whiteSpace=wrap;html=1;fillColor=#ecfeff;strokeColor=#06b6d4;fontColor=#111827;" vertex="1" parent="1"><mxGeometry x="300" y="120" width="140" height="60" as="geometry"/></mxCell><mxCell id="4" value="工具执行" style="rounded=1;whiteSpace=wrap;html=1;fillColor=#f0fdf4;strokeColor=#22c55e;fontColor=#111827;" vertex="1" parent="1"><mxGeometry x="520" y="120" width="140" height="60" as="geometry"/></mxCell><mxCell id="5" value="动态产物" style="rounded=1;whiteSpace=wrap;html=1;fillColor=#fff7ed;strokeColor=#f97316;fontColor=#111827;" vertex="1" parent="1"><mxGeometry x="740" y="120" width="140" height="60" as="geometry"/></mxCell><mxCell id="6" value="" style="endArrow=block;html=1;rounded=0;strokeColor=#6366f1;" edge="1" parent="1" source="2" target="3"><mxGeometry relative="1" as="geometry"/></mxCell><mxCell id="7" value="" style="endArrow=block;html=1;rounded=0;strokeColor=#06b6d4;" edge="1" parent="1" source="3" target="4"><mxGeometry relative="1" as="geometry"/></mxCell><mxCell id="8" value="" style="endArrow=block;html=1;rounded=0;strokeColor=#22c55e;" edge="1" parent="1" source="4" target="5"><mxGeometry relative="1" as="geometry"/></mxCell></root></mxGraphModel></diagram></mxfile>`
}

function EmptyArtifact({ activeTool }: { activeTool: ToolKind }) {
  const labels: Record<string, string> = {
    general: '综合任务产物', ppt: 'PPT 演示文稿', doc: '文档', drawio: 'draw.io 图表', excel: '在线表格', image: '图像结果', code: '代码结果',
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

function MarkdownPreview({ markdown }: { markdown: string }) {
  return (
    <div className="mx-auto max-w-3xl rounded-2xl border border-surface-200 bg-white p-8 shadow-sm">
      {markdown.split('\n').map((line, idx) => {
        if (line.startsWith('# ')) return <h1 key={idx} className="mb-4 text-2xl font-bold text-surface-900">{line.slice(2)}</h1>
        if (line.startsWith('## ')) return <h2 key={idx} className="mb-2 mt-5 text-lg font-semibold text-surface-800">{line.slice(3)}</h2>
        if (line.startsWith('- ')) return <li key={idx} className="ml-5 list-disc text-sm leading-7 text-surface-600">{line.slice(2)}</li>
        if (/^\d+\. /.test(line)) return <div key={idx} className="text-sm leading-7 text-surface-600">{line}</div>
        if (!line) return <div key={idx} className="h-2" />
        return <p key={idx} className="text-sm leading-7 text-surface-600">{line}</p>
      })}
    </div>
  )
}

function DocumentArtifact({ artifact, onExport }: { artifact: Artifact, onExport: () => void }) {
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

function DrawIoArtifact({ artifact, onUpdate }: { artifact: Artifact, onUpdate: (updates: Partial<Artifact>) => void }) {
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
          onAutoSave={mode === 'edit' ? ((data) => onUpdate({ content: { ...artifact.content, xml: data.xml || xml }, status: 'ready' })) : undefined}
          onSave={mode === 'edit' ? ((data) => onUpdate({ content: { ...artifact.content, xml: data.xml || xml }, status: 'ready' })) : undefined}
          onExport={mode === 'edit' ? ((data) => onUpdate({ content: { ...artifact.content, preview: data.data }, status: 'ready' })) : undefined}
        />
      </div>
    </div>
  )
}

function SheetArtifact({ artifact, onUpdate, onExport }: { artifact: Artifact, onUpdate: (updates: Partial<Artifact>) => void, onExport: () => void }) {
  const rows: string[][] = artifact.content?.rows || [['字段', '说明'], ['暂无数据', '等待 Agent 生成']]
  const [isExporting, setIsExporting] = useState(false)
  const handleExport = async () => {
    try {
      setIsExporting(true)
      await Promise.resolve(onExport())
    } finally {
      setIsExporting(false)
    }
  }
  const updateCell = (r: number, c: number, value: string) => {
    const next = rows.map((row) => [...row])
    next[r][c] = value
    onUpdate({ content: { ...artifact.content, rows: next }, status: 'ready' })
  }

  return (
    <div className="overflow-hidden rounded-2xl border border-surface-200 bg-white shadow-sm">
      <div className="flex h-11 items-center justify-between border-b border-surface-100 bg-emerald-50 px-3 text-xs text-emerald-700">
        <span className="font-semibold">{artifact.title || '在线 Excel 工作区'}</span>
        <div className="flex items-center gap-2">
          <span>可编辑表格 / ExcelJS XLSX 导出</span>
          <button
            className="inline-flex items-center gap-1 rounded-full bg-emerald-600 px-2.5 py-1 text-white disabled:cursor-not-allowed disabled:opacity-60"
            disabled={isExporting}
            onClick={handleExport}
          >
            <Download className="h-3 w-3" />{isExporting ? '导出中' : '导出 XLSX'}
          </button>
        </div>
      </div>
      <div className="overflow-auto p-4">
        <table className="w-full min-w-[680px] border-collapse text-sm">
          <tbody>
            {rows.map((row, r) => (
              <tr key={r}>
                {row.map((cell, c) => (
                  <td key={`${r}-${c}`} className={`border border-surface-200 p-0 ${r === 0 ? 'bg-surface-100 font-semibold text-surface-700' : 'text-surface-600'}`}>
                    <input
                      className="h-full w-full bg-transparent px-3 py-2 outline-none focus:bg-primary-50"
                      value={cell}
                      onChange={(e) => updateCell(r, c, e.target.value)}
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

function ImageArtifact({ artifact }: { artifact: Artifact }) {
  const prompt = artifact.content?.prompt || '等待图像 Agent 生成提示词或图片。'
  const images: string[] = artifact.content?.images || []
  return (
    <div className="w-full max-w-3xl space-y-4">
      {images.length > 0 ? (
        <div className="grid grid-cols-2 gap-4">
          {images.map((src) => <img key={src} src={src} className="aspect-video rounded-3xl border border-surface-200 object-cover" />)}
        </div>
      ) : (
        <div className="aspect-video rounded-3xl border border-surface-200 bg-gradient-to-br from-violet-100 via-white to-sky-100 flex items-center justify-center text-surface-400"><Image className="h-10 w-10" /></div>
      )}
      <div className="rounded-2xl border border-surface-200 bg-white p-4 text-sm leading-7 text-surface-600">
        <div className="mb-1 text-xs font-semibold text-surface-400">图像提示词</div>
        {prompt}
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

function ArtifactBody({ artifact, activeTool, onUpdate, onExportExcel, onExportDocx }: { artifact: Artifact | null, activeTool: ToolKind, onUpdate: (id: string, updates: Partial<Artifact>) => void, onExportExcel: (artifact: Artifact) => void, onExportDocx: (artifact: Artifact) => void }) {
  if (!artifact) return <EmptyArtifact activeTool={activeTool} />
  if (artifact.kind === 'document') return <DocumentArtifact artifact={artifact} onExport={() => onExportDocx(artifact)} />
  if (artifact.kind === 'drawio') return <DrawIoArtifact artifact={artifact} onUpdate={(updates) => onUpdate(artifact.id, updates)} />
  if (artifact.kind === 'sheet') return <SheetArtifact artifact={artifact} onUpdate={(updates) => onUpdate(artifact.id, updates)} onExport={() => onExportExcel(artifact)} />
  if (artifact.kind === 'image') return <ImageArtifact artifact={artifact} />
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
  activeArtifact,
  artifacts,
  onSelectArtifact,
  onUpdateArtifact,
  onExportExcel,
  onExportDocx,
}: ArtifactPanelProps) {
  const [panelWidth, setPanelWidth] = useState(isWide ? 760 : 560)
  const draggingRef = useRef(false)

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
    general: '动态成果展示', ppt: 'PPT 预览', doc: '文档预览', drawio: 'draw.io 画布', excel: '在线 Excel', image: '图像结果', code: '代码结果',
  }

  const effectiveTool = activeArtifact?.tool_kind || activeTool
  const headerTitle = activeArtifact?.title || titleMap[effectiveTool] || '成果展示'

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
            <div className="text-[11px] text-surface-400">随 Agent 产物动态展开 · 可关闭 · 可编辑</div>
          </div>
        </div>
        <div className="flex items-center gap-1.5">
          {effectiveTool === 'ppt' && (
            <>
              <button className="btn-secondary h-8 px-2.5 text-xs" disabled={!project} onClick={onPresent}>演示</button>
              <button className="btn-secondary h-8 px-2.5 text-xs" disabled={!project} onClick={onExportPpt}><Download className="h-3.5 w-3.5" />导出</button>
            </>
          )}
          <button className="btn-ghost h-8 px-2" onClick={() => onWideChange(!isWide)} title="切换宽度">
            {isWide ? <Minimize2 className="h-4 w-4" /> : <Maximize2 className="h-4 w-4" />}
          </button>
          <button className="btn-ghost h-8 px-2" onClick={() => onOpenChange(false)} title="关闭右侧面板">关闭</button>
        </div>
      </div>

      {artifacts.length > 0 && (
        <div className="shrink-0 border-b border-surface-100 bg-white/80 px-3 py-2">
          <div className="flex gap-2 overflow-x-auto">
            {artifacts.map((artifact) => (
              <button
                key={artifact.id}
                onClick={() => onSelectArtifact(artifact.id)}
                className={`shrink-0 rounded-full px-3 py-1 text-xs ${activeArtifact?.id === artifact.id ? 'bg-primary-600 text-white' : 'bg-surface-100 text-surface-500 hover:bg-surface-200'}`}
                title={artifact.title}
              >
                {artifact.kind} · {artifact.title.slice(0, 16)}
              </button>
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
              <ArtifactBody artifact={activeArtifact} activeTool={effectiveTool} onUpdate={onUpdateArtifact} onExportExcel={onExportExcel} onExportDocx={onExportDocx} />
            )}
          </div>
        </div>

      </div>
    </aside>
  )
}
