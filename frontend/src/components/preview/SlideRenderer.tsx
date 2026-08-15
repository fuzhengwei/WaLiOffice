import { useEffect, useRef, useState } from 'react'
import { EChartsView } from '@/components/preview/EChartsView'
import type { Slide, SlideElement } from '@/types'

interface SlideRendererProps {
  slide: Slide
}

/**
 * 幻灯片渲染器
 * 逻辑画布固定为 1280 x 720，后续由外层按 16:9 缩放。
 * 后端坐标仍使用 13.33 x 7.5 英寸，这里换算成像素，避免字体/圆角/留白在不同容器下失真。
 */
export function SlideRenderer({ slide }: SlideRendererProps) {
  const W = 13.33
  const H = 7.5
  const PX_W = 1280
  const PX_H = 720

  const toPxX = (x: number) => (x / W) * PX_W
  const toPxY = (y: number) => (y / H) * PX_H
  const toPxW = (w: number) => (w / W) * PX_W
  const toPxH = (h: number) => (h / H) * PX_H

  return (
    <div
      className="absolute inset-0 overflow-hidden"
      style={{
        width: PX_W,
        height: PX_H,
        transformOrigin: 'top left',
        background: `#${slide.background || 'FFFFFF'}`,
      }}
    >
      {slide.elements?.map((el, i) => (
        <ElementRenderer
          key={i}
          element={el}
          slideBackground={slide.background || 'FFFFFF'}
          toPxX={toPxX}
          toPxY={toPxY}
          toPxW={toPxW}
          toPxH={toPxH}
        />
      ))}
    </div>
  )
}

function normalizeColor(color?: string, fallback = '333333') {
  return `#${(color || fallback).replace('#', '')}`
}

function hexToRgb(color?: string) {
  const hex = (color || '').replace('#', '').trim()
  if (!/^[0-9a-fA-F]{6}$/.test(hex)) return null
  return {
    r: parseInt(hex.slice(0, 2), 16),
    g: parseInt(hex.slice(2, 4), 16),
    b: parseInt(hex.slice(4, 6), 16),
  }
}

function relativeLuminance(color?: string) {
  const rgb = hexToRgb(color)
  if (!rgb) return 1
  const channel = (value: number) => {
    const s = value / 255
    return s <= 0.03928 ? s / 12.92 : Math.pow((s + 0.055) / 1.055, 2.4)
  }
  return 0.2126 * channel(rgb.r) + 0.7152 * channel(rgb.g) + 0.0722 * channel(rgb.b)
}

function contrastRatio(foreground?: string, background?: string) {
  const fg = relativeLuminance(foreground)
  const bg = relativeLuminance(background)
  const light = Math.max(fg, bg)
  const dark = Math.min(fg, bg)
  return (light + 0.05) / (dark + 0.05)
}

function readableTextColor(color?: string, background?: string) {
  const fg = normalizeColor(color)
  const bg = normalizeColor(background, 'FFFFFF')
  if (contrastRatio(fg, bg) >= 4.5) return fg
  const dark = '#0F172A'
  const light = '#F8FAFC'
  return contrastRatio(dark, bg) >= contrastRatio(light, bg) ? dark : light
}

function fontSizeForBox(el: SlideElement) {
  const base = el.fontSize || 18
  const text = el.text || ''
  const lines = text.split('\n').length
  const chars = text.length
  const area = Math.max(1, el.w * el.h)
  let factor = 1

  // 经验压缩：长文本、窄框、多行时自动缩小，避免“看不全”。
  if (chars > area * 18) factor *= 0.82
  if (chars > area * 26) factor *= 0.72
  if (chars > area * 34) factor *= 0.62
  if (lines >= 4) factor *= 0.88
  if (lines >= 7) factor *= 0.78
  if (el.h < 0.55) factor *= 0.82

  return Math.max(8, Math.round(base * factor * 1.15))
}

function AutoFitText({
  element: el,
  initialFontSize,
  background,
}: {
  element: SlideElement
  initialFontSize: number
  background: string
}) {
  const ref = useRef<HTMLDivElement>(null)
  const [fontSize, setFontSize] = useState(initialFontSize)

  useEffect(() => {
    const node = ref.current
    if (!node) return

    let next = initialFontSize
    const min = Math.max(7, Math.min(12, Math.round((el.fontSize || initialFontSize) * 0.55)))
    node.style.fontSize = `${next}px`

    // DOM 实测兜底：后端已经重排，这里处理浏览器字体差异/缩放差异导致的残余裁切。
    while ((node.scrollHeight > node.clientHeight + 1 || node.scrollWidth > node.clientWidth + 1) && next > min) {
      next -= 1
      node.style.fontSize = `${next}px`
    }

    setFontSize(next)
  }, [el.text, el.w, el.h, el.fontSize, initialFontSize])

  return (
    <div
      ref={ref}
      style={{
        fontSize,
        color: readableTextColor(el.color, el.fill || background),
        fontWeight: el.bold ? 700 : 400,
        fontStyle: el.italic ? 'italic' : 'normal',
        textAlign: (el.align || 'left') as any,
        lineHeight: fontSize >= 30 ? 1.1 : fontSize <= 12 ? 1.18 : 1.25,
        whiteSpace: 'pre-wrap',
        wordBreak: 'break-word',
        overflowWrap: 'anywhere',
        width: '100%',
        height: '100%',
        maxHeight: '100%',
        overflow: 'hidden',
        letterSpacing: el.bold && fontSize >= 28 ? '-0.02em' : 0,
      }}
    >
      {el.text}
    </div>
  )
}

function ElementRenderer({
  element: el,
  slideBackground,
  toPxX,
  toPxY,
  toPxW,
  toPxH,
}: {
  element: SlideElement
  slideBackground: string
  toPxX: (x: number) => number
  toPxY: (y: number) => number
  toPxW: (w: number) => number
  toPxH: (h: number) => number
}) {
  const baseStyle: React.CSSProperties = {
    position: 'absolute',
    left: toPxX(el.x),
    top: toPxY(el.y),
    width: toPxW(el.w),
    height: toPxH(el.h),
    boxSizing: 'border-box',
    display: 'flex',
    flexDirection: 'column',
    justifyContent:
      el.valign === 'middle' ? 'center' : el.valign === 'bottom' ? 'flex-end' : 'flex-start',
    alignItems: el.align === 'center' ? 'center' : el.align === 'right' ? 'flex-end' : 'flex-start',
    background: el.fill ? normalizeColor(el.fill) : 'transparent',
  }

  if (el.type === 'text' && el.text) {
    const fs = fontSizeForBox(el)
    return (
      <div
        style={{
          ...baseStyle,
          padding: Math.max(4, Math.min(18, fs * 0.28)),
          overflow: 'hidden',
        }}
      >
        <AutoFitText element={el} initialFontSize={fs} background={slideBackground} />
      </div>
    )
  }

  if (el.type === 'image' && el.path) {
    return (
      <div style={baseStyle}>
        <img src={el.path} alt="" style={{ width: '100%', height: '100%', objectFit: 'contain' }} />
      </div>
    )
  }

  if (el.type === 'shape') {
    return (
      <div
        style={{
          ...baseStyle,
          background: el.fill ? normalizeColor(el.fill, '4A90D9') : '#4A90D9',
          borderRadius:
            el.shape === 'roundRect' ? Math.min(28, Math.max(10, toPxH(el.h) * 0.08)) :
            el.shape === 'ellipse' ? '50%' : '0',
          boxShadow: el.shape === 'roundRect' && el.w > 2 && el.h > 1
            ? '0 18px 42px rgba(15, 23, 42, 0.08)'
            : undefined,
        }}
      />
    )
  }

  if (el.type === 'table' && el.table_data) {
    return (
      <div style={baseStyle} className="overflow-hidden">
        <table className="w-full h-full border-collapse">
          <tbody>
            {el.table_data.map((row, ri) => (
              <tr key={ri}>
                {row.map((cell, ci) => (
                  <td
                    key={ci}
                    className="border border-surface-200 px-2 py-1"
                    style={{
                      fontSize: fontSizeForBox(el) * 0.85,
                      color: readableTextColor(el.color, ri === 0 ? 'F4F4F5' : 'FFFFFF'),
                      textAlign: (el.align || 'left') as any,
                      fontWeight: ri === 0 ? 'bold' : 'normal',
                      background: ri === 0 ? '#f4f4f5' : 'white',
                    }}
                  >
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

  if (el.type === 'chart') {
    return (
      <div style={baseStyle} className="overflow-hidden rounded-xl bg-white p-2 shadow-sm ring-1 ring-black/[0.06]">
        <EChartsView
          title={el.text || el.chart_type || '图表'}
          chartType={el.chart_type}
          chartData={el.chart_data}
          option={el.chart_data?.option}
          style={{ minHeight: 0 }}
        />
      </div>
    )
  }

  return null
}
