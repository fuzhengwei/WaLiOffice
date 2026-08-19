import type { Artifact } from '@/types'

/**
 * 从产物中提取内容摘要，用于发送给后端让 AI 识别引用的产物内容。
 * 按产物类型提取关键信息，截断到合理长度。
 */
export function extractArtifactSummary(artifact: Artifact): string {
  const { kind, content } = artifact
  if (!content) return ''

  const truncate = (text: string, max = 2000) => text.slice(0, max)

  switch (kind) {
    case 'ppt': {
      const slides = content.slides || content.slide_count
      if (Array.isArray(slides)) {
        const slideSummaries = slides.map((s: any, i: number) => {
          const title = s.title || ''
          const elements = Array.isArray(s.elements) ? s.elements : []
          const textParts = elements
            .filter((e: any) => e.type === 'text' && e.text)
            .map((e: any) => e.text)
          const bodyText = textParts.join(' | ')
          return `第${i + 1}页：${title}${bodyText ? ' — ' + bodyText : ''}`
        })
        return `【PPT产物：${artifact.title}，共 ${slides.length} 页】\n${truncate(slideSummaries.join('\n'), 3000)}`
      }
      const count = content.slide_count || 0
      return `【PPT产物：${artifact.title}，共 ${count} 页】`
    }

    case 'document':
    case 'markdown': {
      const md = content.markdown || ''
      if (md) {
        return `【${kind === 'document' ? '文档' : 'Markdown'}产物：${artifact.title}】\n${truncate(md, 3000)}`
      }
      const sections = content.sections || []
      if (Array.isArray(sections) && sections.length > 0) {
        const sectionTexts = sections.map((s: any) => {
          const heading = s.heading || ''
          const body = s.content || s.body || ''
          return `${heading}\n${body}`
        })
        return `【${kind === 'document' ? '文档' : 'Markdown'}产物：${artifact.title}】\n${truncate(sectionTexts.join('\n\n'), 3000)}`
      }
      return `【${kind === 'document' ? '文档' : 'Markdown'}产物：${artifact.title}】`
    }

    case 'sheet': {
      const tables = content.tables || []
      if (Array.isArray(tables) && tables.length > 0) {
        const tableSummaries = tables.map((t: any, i: number) => {
          const name = t.name || `表格${i + 1}`
          const rows = Array.isArray(t.rows) ? t.rows : []
          const rowCount = rows.length
          // 取前 5 行数据作为预览
          const preview = rows.slice(0, 5).map((row: any) => {
            if (Array.isArray(row)) return row.map((c: any) => String(c ?? '')).join(' | ')
            if (row && typeof row === 'object') {
              const cells = row.cells || row.values || []
              if (Array.isArray(cells)) return cells.map((c: any) => String(c?.value ?? c ?? '')).join(' | ')
            }
            return ''
          })
          return `${name}（${rowCount} 行）\n${preview.join('\n')}`
        })
        return `【表格产物：${artifact.title}】\n${truncate(tableSummaries.join('\n\n'), 3000)}`
      }
      return `【表格产物：${artifact.title}】`
    }

    case 'drawio': {
      const xml = content.xml || ''
      if (xml) {
        // 提取 XML 中的文本标签
        const textMatches = xml.match(/>([^<]+)</g) || []
        const texts = textMatches
          .map((m: string) => m.slice(1, -1).trim())
          .filter((t: string) => t.length > 0 && !t.startsWith('<?xml'))
        return `【图表产物：${artifact.title}】\n节点文本：${truncate(texts.join(' | '), 1500)}`
      }
      return `【图表产物：${artifact.title}】`
    }

    case 'image': {
      const images = content.images || []
      const count = Array.isArray(images) ? images.length : 0
      const desc = content.description || content.prompt || ''
      return `【图片产物：${artifact.title}，${count} 张图片】${desc ? '\n描述：' + truncate(desc, 500) : ''}`
    }

    case 'video': {
      const url = content.video_url || ''
      const desc = content.description || content.prompt || ''
      return `【视频产物：${artifact.title}】${desc ? '\n描述：' + truncate(desc, 500) : ''}${url ? '\n链接：' + url : ''}`
    }

    default:
      return `【产物：${artifact.title}（类型：${kind}）】`
  }
}
