/**
 * WordPreview — A4 版式预览组件
 * 将结构化 DocSchema 渲染为接近 Word 文档的视觉效果。
 *
 * DocSchema 格式：
 * {
 *   title: string,
 *   sections: [{
 *     heading?: string,
 *     headingLevel?: 1|2|3,
 *     paragraphs?: string[],
 *     bullets?: string[],
 *     table?: { headers: string[], rows: string[][] },
 *     pageBreakBefore?: boolean,
 *   }]
 * }
 */

import { memo } from 'react'

interface DocSection {
  heading?: string
  headingLevel?: 1 | 2 | 3
  paragraphs?: string[]
  bullets?: string[]
  table?: {
    headers: string[]
    rows: string[][]
  }
  pageBreakBefore?: boolean
}

interface DocContent {
  title?: string
  sections?: DocSection[]
  markdown?: string
  type?: string
}

interface WordPreviewProps {
  content: DocContent
  title?: string
}

/** 解析简单内联格式：**粗体** *斜体* `代码` */
function renderInline(text: string): React.ReactNode[] {
  const parts: React.ReactNode[] = []
  const regex = /(\*\*(.+?)\*\*|\*(.+?)\*|`(.+?)`)/g
  let lastIndex = 0
  let match: RegExpExecArray | null
  let key = 0

  while ((match = regex.exec(text)) !== null) {
    if (match.index > lastIndex) {
      parts.push(text.slice(lastIndex, match.index))
    }
    if (match[2] !== undefined) {
      parts.push(<strong key={key++} className="font-bold text-gray-900">{match[2]}</strong>)
    } else if (match[3] !== undefined) {
      parts.push(<em key={key++} className="italic">{match[3]}</em>)
    } else if (match[4] !== undefined) {
      parts.push(<code key={key++} className="rounded bg-gray-100 px-1 py-0.5 font-mono text-[0.85em] text-gray-800">{match[4]}</code>)
    }
    lastIndex = regex.lastIndex
  }
  if (lastIndex < text.length) {
    parts.push(text.slice(lastIndex))
  }
  return parts.length > 0 ? parts : [text]
}

function TableBlock({ table }: { table: { headers: string[]; rows: string[][] } }) {
  return (
    <div className="my-4 overflow-x-auto">
      <table className="w-full border-collapse text-sm">
        <thead>
          <tr>
            {table.headers.map((h, i) => (
              <th key={i} className="border border-gray-300 bg-gray-100 px-3 py-2 text-left font-semibold text-gray-800">
                {h}
              </th>
            ))}
          </tr>
        </thead>
        <tbody>
          {table.rows.map((row, ri) => (
            <tr key={ri} className={ri % 2 === 1 ? 'bg-gray-50' : ''}>
              {row.map((cell, ci) => (
                <td key={ci} className="border border-gray-300 px-3 py-2 text-gray-700">
                  {renderInline(cell)}
                </td>
              ))}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

function SectionBlock({ section, index }: { section: DocSection; index: number }) {
  const level = section.headingLevel || 1
  const headingClass =
    level === 1
      ? 'mb-3 mt-6 text-xl font-bold text-gray-900'
      : level === 2
        ? 'mb-2 mt-5 text-lg font-semibold text-gray-800'
        : 'mb-2 mt-4 text-base font-semibold text-gray-700'

  return (
    <div className={section.pageBreakBefore ? 'break-before-page' : ''}>
      {section.heading && (
        <h2 className={headingClass}>{section.heading}</h2>
      )}
      {section.paragraphs?.map((p, i) => (
        <p key={i} className="mb-3 text-[14px] leading-[1.8] text-gray-700 indent-0">
          {renderInline(p)}
        </p>
      ))}
      {section.bullets && section.bullets.length > 0 && (
        <ul className="mb-3 space-y-1.5 pl-5">
          {section.bullets.map((b, i) => (
            <li key={i} className="list-disc text-[14px] leading-[1.8] text-gray-700">
              {renderInline(b)}
            </li>
          ))}
        </ul>
      )}
      {section.table && <TableBlock table={section.table} />}
    </div>
  )
}

export const WordPreview = memo(function WordPreview({ content, title }: WordPreviewProps) {
  const docTitle = title || content.title || '文档'
  const sections = content.sections || []

  return (
    <div className="mx-auto w-full max-w-[794px]">
      {/* A4 页面模拟 */}
      <div className="bg-white shadow-lg shadow-gray-200 ring-1 ring-gray-200"
           style={{
             padding: '72px 72px 72px 72px',
             minHeight: '1123px', // A4 高度 @96dpi
             fontFamily: '"SimSun", "宋体", "Noto Serif SC", serif',
           }}>
        {/* 文档标题（封面式） */}
        <div className="mb-8 pb-6 text-center" style={{ borderBottom: '2px solid #e5e7eb' }}>
          <h1 className="text-2xl font-bold text-gray-900" style={{ fontFamily: '"SimHei", "黑体", "Noto Sans SC", sans-serif' }}>
            {docTitle}
          </h1>
        </div>

        {/* 正文内容 */}
        <div className="space-y-1">
          {sections.length > 0 ? (
            sections.map((sec, i) => <SectionBlock key={i} section={sec} index={i} />)
          ) : (
            <p className="text-sm text-gray-400">暂无内容</p>
          )}
        </div>

        {/* 页脚模拟 */}
        <div className="mt-12 pt-4 text-center text-[11px] text-gray-400" style={{ borderTop: '1px solid #e5e7eb' }}>
          — WaLiOffice 文档预览 · 导出 DOCX 获取完整排版 —
        </div>
      </div>
    </div>
  )
})

export default WordPreview
