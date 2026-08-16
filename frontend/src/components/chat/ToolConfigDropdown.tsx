import { useState, useRef, useEffect } from 'react'
import { ChevronDown, Settings2 } from 'lucide-react'
import { getAgentTool } from '@/config/agent-tools'
import type { ToolKind, ToolConfigMap, ToolConfigOption } from '@/types'

interface ToolConfigDropdownProps {
  activeTool: ToolKind
  toolConfig: ToolConfigMap
  onToolConfigChange: (config: ToolConfigMap) => void
  disabled?: boolean
}

/**
 * 工具配置下拉菜单，替代原来的主题选择器。
 * 根据当前 activeTool 显示对应的配置项。
 */
export function ToolConfigDropdown({
  activeTool,
  toolConfig,
  onToolConfigChange,
  disabled = false,
}: ToolConfigDropdownProps) {
  const [open, setOpen] = useState(false)
  const dropdownRef = useRef<HTMLDivElement>(null)

  const tool = getAgentTool(activeTool)
  const options = tool.configOptions
  const hasConfig = options && options.length > 0

  // 点击外部关闭
  useEffect(() => {
    if (!open) return
    const handleClickOutside = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [open])

  // 切换工具时自动填充默认值
  useEffect(() => {
    if (!options) return
    const defaults: ToolConfigMap = {}
    for (const opt of options) {
      if (!(opt.key in toolConfig)) {
        defaults[opt.key] = opt.defaultValue
      }
    }
    if (Object.keys(defaults).length > 0) {
      onToolConfigChange({ ...toolConfig, ...defaults })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTool])

  const handleChange = (key: string, value: string | boolean) => {
    onToolConfigChange({ ...toolConfig, [key]: value })
  }

  const getActiveLabel = (opt: ToolConfigOption): string => {
    const val = toolConfig[opt.key] ?? opt.defaultValue
    if (opt.type === 'toggle') return val ? '开' : '关'
    const match = opt.options?.find((o) => o.value === val)
    return match?.label ?? String(val)
  }

  // 检查是否有非默认配置
  const hasNonDefault = hasConfig
    ? options!.some((opt) => {
        const val = toolConfig[opt.key] ?? opt.defaultValue
        return val !== opt.defaultValue
      })
    : false

  // 按钮上只显示第一项配置摘要，保持简短
  const firstLabel = hasConfig ? getActiveLabel(options![0]) : '默认'

  // 没有配置项时，显示简化按钮
  if (!hasConfig) {
    return (
      <div className="inline-flex h-9 items-center gap-1.5 rounded-full border border-black/[0.06] bg-white px-2.5 text-[11px] font-medium text-surface-700 shadow-[0_1px_2px_rgba(15,23,42,0.04)]">
        <Settings2 className="h-3.5 w-3.5 text-surface-400" />
        <span className="text-surface-500">配置</span>
        <span className="font-semibold text-surface-900">默认</span>
      </div>
    )
  }

  return (
    <div className="relative" ref={dropdownRef}>
      <button
        type="button"
        disabled={disabled}
        onClick={() => setOpen(!open)}
        className={`
          inline-flex h-9 items-center gap-1.5 rounded-full border px-2.5 text-[11px] font-medium
          transition-all disabled:opacity-50
          ${
            open
              ? 'border-indigo-200 bg-indigo-50 text-indigo-700 shadow-sm'
              : hasNonDefault
                ? 'border-amber-200 bg-amber-50 text-amber-700 shadow-sm hover:bg-amber-100'
                : 'border-black/[0.06] bg-white text-surface-700 shadow-[0_1px_2px_rgba(15,23,42,0.04)] hover:-translate-y-[0.5px] hover:shadow-[0_6px_14px_rgba(15,23,42,0.08)]'
          }
        `}
      >
        <Settings2 className={`h-3.5 w-3.5 ${open ? 'text-indigo-500' : hasNonDefault ? 'text-amber-500' : 'text-surface-400'}`} />
        <span className={open ? 'text-indigo-500' : hasNonDefault ? 'text-amber-500' : 'text-surface-500'}>配置</span>
        <span className={`font-semibold ${open ? 'text-indigo-700' : 'text-surface-900'}`}>{firstLabel}</span>
        {hasNonDefault && !open && (
          <span className="h-1.5 w-1.5 rounded-full bg-amber-500" />
        )}
        <ChevronDown className={`h-3 w-3 ${open ? 'text-indigo-400' : 'text-surface-400'} transition-transform ${open ? 'rotate-180' : ''}`} />
      </button>

      {open && (
        <div className="absolute bottom-full right-0 mb-2 w-72 rounded-2xl border border-gray-200 bg-white p-0 shadow-lg z-50 overflow-hidden">
          {/* 标题栏 */}
          <div className="flex items-center justify-between px-4 py-2.5 border-b border-gray-100 bg-gray-50">
            <div className="flex items-center gap-2">
              <Settings2 className="h-4 w-4 text-indigo-500" />
              <span className="text-xs font-semibold text-surface-900">
                {tool.name}配置
              </span>
            </div>
            <button
              onClick={() => setOpen(false)}
              className="text-gray-400 hover:text-gray-600 transition-colors"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          {/* 配置项列表 */}
          <div className="p-3 space-y-3">
            {options!.map((opt) => (
              <div key={opt.key}>
                <label className="block text-[11px] font-semibold text-surface-500 mb-1.5">
                  {opt.label}
                </label>

                {opt.type === 'select' && opt.options && (
                  <div className="flex flex-wrap gap-1.5">
                    {opt.options.map((option) => {
                      const currentValue = toolConfig[opt.key] ?? opt.defaultValue
                      const isActive = currentValue === option.value
                      return (
                        <button
                          key={option.value}
                          onClick={() => handleChange(opt.key, option.value)}
                          className={`
                            flex items-center gap-1 px-2.5 py-1.5 rounded-lg text-[11px] font-medium transition-all
                            ${isActive
                              ? 'bg-indigo-500 text-white shadow-sm'
                              : 'bg-gray-50 text-surface-600 hover:bg-indigo-50 border border-gray-100'
                            }
                          `}
                          title={option.description}
                        >
                          {option.label}
                        </button>
                      )
                    })}
                  </div>
                )}

                {opt.type === 'toggle' && (
                  <button
                    onClick={() => handleChange(opt.key, !(toolConfig[opt.key] ?? opt.defaultValue))}
                    className={`
                      relative inline-flex h-5 w-9 items-center rounded-full transition-colors
                      ${(toolConfig[opt.key] ?? opt.defaultValue) ? 'bg-indigo-500' : 'bg-gray-300'}
                    `}
                  >
                    <span
                      className={`
                        inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform
                        ${(toolConfig[opt.key] ?? opt.defaultValue) ? 'translate-x-4.5' : 'translate-x-0.5'}
                      `}
                    />
                  </button>
                )}
              </div>
            ))}
          </div>

          {/* 底部摘要 */}
          <div className="px-4 py-2 border-t border-gray-100 bg-gray-50">
            <div className="flex flex-wrap gap-1.5">
              {options!.map((opt) => (
                <span
                  key={opt.key}
                  className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] bg-gray-100 text-surface-500"
                >
                  <span className="font-medium">{opt.label}</span>
                  <span className="text-surface-700">{getActiveLabel(opt)}</span>
                </span>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
