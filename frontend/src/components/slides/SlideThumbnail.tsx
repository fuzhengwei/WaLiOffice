import type { Slide } from '@/types'

interface SlideThumbnailProps {
  slide: Slide
}

/** 幻灯片缩略图 - 简化版渲染 */
export function SlideThumbnail({ slide }: SlideThumbnailProps) {
  // 16:9 画布：13.33 x 7.5 inches → 缩放为百分比
  const scale = 100 / 13.33

  return (
    <div
      className="w-full h-full relative overflow-hidden"
      style={{ background: `#${slide.background}` }}
    >
      {slide.elements?.map((el, i) => (
        <div
          key={i}
          className="absolute overflow-hidden"
          style={{
            left: `${el.x * scale}%`,
            top: `${el.y * (100 / 7.5)}%`,
            width: `${el.w * scale}%`,
            height: `${el.h * (100 / 7.5)}%`,
            background: el.fill ? `#${el.fill}` : 'transparent',
          }}
        >
          {el.type === 'text' && el.text && (
            <span
              style={{
                fontSize: `${Math.max(4, (el.fontSize || 14) * 0.18)}px`,
                color: `#${el.color || '333333'}`,
                fontWeight: el.bold ? 'bold' : 'normal',
                fontStyle: el.italic ? 'italic' : 'normal',
                textAlign: el.align || 'left',
                display: 'block',
                lineHeight: 1.3,
                whiteSpace: 'pre-wrap',
                wordBreak: 'break-word',
              }}
            >
              {el.text}
            </span>
          )}
          {el.type === 'shape' && (
            <div
              style={{
                width: '100%',
                height: '100%',
                background: el.fill ? `#${el.fill}` : '#4A90D9',
                borderRadius: el.shape === 'roundRect' ? '8px' : el.shape === 'ellipse' ? '50%' : '0',
              }}
            />
          )}
        </div>
      ))}
    </div>
  )
}
