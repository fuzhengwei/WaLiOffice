import { useState, useEffect, useRef } from 'react'
import type { AgentToolConfig, ToolConfigOption, ToolConfigMap } from '@/types'
import { getAgentTool } from '@/config/agent-tools'

interface ToolConfigPanelProps {
  activeTool: string
  config: ToolConfigMap
  onConfigChange: (config: ToolConfigMap) => void
}

export default function ToolConfigPanel({ activeTool, config, onConfigChange }: ToolConfigPanelProps) {
  const [open, setOpen] = useState(false)
  const panelRef = useRef<HTMLDivElement>(null)

  const toolConfig = getAgentTool(activeTool) as AgentToolConfig
  const options = toolConfig?.configOptions

  // 点击外部关闭
  useEffect(() => {
    if (!open) return
    const handleClickOutside = (e: MouseEvent) => {
      if (panelRef.current && !panelRef.current.contains(e.target as Node)) {
        setOpen(false)
      }
    }
    document.addEventListener('mousedown', handleClickOutside)
    return () => document.removeEventListener('mousedown', handleClickOutside)
  }, [open])

  // 切换工具时重置配置为默认值
  useEffect(() => {
    if (!options) return
    const defaults: ToolConfigMap = {}
    for (const opt of options) {
      if (!(opt.key in config)) {
        defaults[opt.key] = opt.defaultValue
      }
    }
    if (Object.keys(defaults).length > 0) {
      onConfigChange({ ...config, ...defaults })
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [activeTool])

  if (!options || options.length === 0) return null

  const handleChange = (key: string, value: string | boolean) => {
    onConfigChange({ ...config, [key]: value })
  }

  const getActiveLabel = (opt: ToolConfigOption): string => {
    const val = config[opt.key] ?? opt.defaultValue
    if (opt.type === 'toggle') return val ? '开' : '关'
    const match = opt.options?.find(o => o.value === val)
    return match?.label ?? String(val)
  }

  const hasNonDefault = options.some(opt => {
    const val = config[opt.key] ?? opt.defaultValue
    return val !== opt.defaultValue
  })

  return (
    <div className="relative" ref={panelRef}>
      <button
        onClick={() => setOpen(!open)}
        className={`
          flex items-center gap-1 px-2 py-1 rounded-md text-xs font-medium transition-colors
          ${open
            ? 'bg-indigo-100 text-indigo-700 dark:bg-indigo-900/40 dark:text-indigo-300'
            : hasNonDefault
              ? 'bg-amber-50 text-amber-700 dark:bg-amber-900/30 dark:text-amber-300'
              : 'bg-gray-100 text-gray-500 hover:bg-gray-200 dark:bg-gray-800 dark:text-gray-400 dark:hover:bg-gray-700'
          }
        `}
        title="调整生成配置"
      >
        <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
        </svg>
        <span>配置</span>
        {hasNonDefault && (
          <span className="w-1.5 h-1.5 rounded-full bg-amber-500" />
        )}
      </button>

      {open && (
        <div className="absolute bottom-full left-0 mb-2 w-72 bg-white dark:bg-gray-800 rounded-xl shadow-xl border border-gray-200 dark:border-gray-700 z-50 overflow-hidden">
          {/* 标题栏 */}
          <div className="flex items-center justify-between px-4 py-2.5 border-b border-gray-100 dark:border-gray-700 bg-gray-50 dark:bg-gray-800/80">
            <div className="flex items-center gap-2">
              <span className="text-sm font-medium text-gray-700 dark:text-gray-200">
                {toolConfig.name}配置
              </span>
            </div>
            <button
              onClick={() => setOpen(false)}
              className="text-gray-400 hover:text-gray-600 dark:hover:text-gray-300 transition-colors"
            >
              <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
              </svg>
            </button>
          </div>

          {/* 配置项 */}
          <div className="p-3 space-y-3">
            {options.map((opt) => (
              <div key={opt.key}>
                <label className="block text-xs font-medium text-gray-500 dark:text-gray-400 mb-1.5">
                  {opt.label}
                </label>

                {opt.type === 'select' && opt.options && (
                  <div className="grid grid-cols-1 gap-1">
                    {opt.options.map((option) => {
                      const currentValue = config[opt.key] ?? opt.defaultValue
                      const isActive = currentValue === option.value
                      return (
                        <button
                          key={option.value}
                          onClick={() => handleChange(opt.key, option.value)}
                          className={`
                            flex items-center justify-between px-3 py-2 rounded-lg text-left transition-all text-xs
                            ${isActive
                              ? 'bg-indigo-50 text-indigo-700 ring-1 ring-indigo-200 dark:bg-indigo-900/30 dark:text-indigo-300 dark:ring-indigo-800'
                              : 'bg-gray-50 text-gray-600 hover:bg-gray-100 dark:bg-gray-700/50 dark:text-gray-300 dark:hover:bg-gray-700'
                            }
                          `}
                        >
                          <div className="flex items-center gap-2">
                            <span className={`w-2 h-2 rounded-full ${isActive ? 'bg-indigo-500' : 'bg-gray-300 dark:bg-gray-500'}`} />
                            <span className="font-medium">{option.label}</span>
                          </div>
                          {option.description && (
                            <span className="text-gray-400 dark:text-gray-500 text-[10px] ml-2 truncate max-w-[100px]">
                              {option.description}
                            </span>
                          )}
                        </button>
                      )
                    })}
                  </div>
                )}

                {opt.type === 'toggle' && (
                  <button
                    onClick={() => handleChange(opt.key, !(config[opt.key] ?? opt.defaultValue))}
                    className={`
                      relative inline-flex h-5 w-9 items-center rounded-full transition-colors
                      ${(config[opt.key] ?? opt.defaultValue)
                        ? 'bg-indigo-500'
                        : 'bg-gray-300 dark:bg-gray-600'
                      }
                    `}
                  >
                    <span
                      className={`
                        inline-block h-3.5 w-3.5 transform rounded-full bg-white transition-transform
                        ${(config[opt.key] ?? opt.defaultValue) ? 'translate-x-4.5' : 'translate-x-0.5'}
                      `}
                    />
                  </button>
                )}
              </div>
            ))}
          </div>

          {/* 底部摘要 */}
          <div className="px-4 py-2 border-t border-gray-100 dark:border-gray-700 bg-gray-50/50 dark:bg-gray-800/50">
            <div className="flex flex-wrap gap-1.5">
              {options.map((opt) => (
                <span
                  key={opt.key}
                  className="inline-flex items-center gap-1 px-1.5 py-0.5 rounded text-[10px] bg-gray-100 dark:bg-gray-700 text-gray-500 dark:text-gray-400"
                >
                  <span className="font-medium">{opt.label}</span>
                  <span className="text-gray-700 dark:text-gray-300">{getActiveLabel(opt)}</span>
                </span>
              ))}
            </div>
          </div>
        </div>
      )}
    </div>
  )
}
