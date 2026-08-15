import { useEffect, useRef, useState } from 'react'
import type { Slide } from '@/types'
import { SlideRenderer } from './SlideRenderer'

interface SlidePreviewProps {
  slide: Slide
  layout: '16x9' | '4x3'
  fullScreen?: boolean
}

/** 全尺寸幻灯片预览 */
export function SlidePreview({ slide, layout, fullScreen }: SlidePreviewProps) {
  const aspectClass = layout === '4x3' ? 'aspect-[4/3]' : 'aspect-video'
  const ref = useRef<HTMLDivElement>(null)
  const [scale, setScale] = useState(1)

  useEffect(() => {
    const el = ref.current
    if (!el) return
    const update = () => {
      const rect = el.getBoundingClientRect()
      setScale(Math.min(rect.width / 1280, rect.height / 720))
    }
    update()
    const ro = new ResizeObserver(update)
    ro.observe(el)
    return () => ro.disconnect()
  }, [layout, fullScreen])

  return (
    <div
      ref={ref}
      className={`w-full ${aspectClass} bg-white rounded-xl shadow-xl border border-surface-200 overflow-hidden relative`}
      style={{
        maxWidth: fullScreen ? '100%' : '960px',
        background: `#${slide.background || 'FFFFFF'}`,
      }}
    >
      <div
        className="absolute origin-top-left"
        style={{
          left: `calc(50% - ${640 * scale}px)`,
          top: `calc(50% - ${360 * scale}px)`,
          width: 1280,
          height: 720,
          transform: `scale(${scale})`,
        }}
      >
        <SlideRenderer slide={slide} />
      </div>
    </div>
  )
}
