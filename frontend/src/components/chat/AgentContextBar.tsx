import { BrainCircuit, ChevronDown, Cpu, FileText, History, PanelRightOpen, Sparkles } from 'lucide-react'
import { getAgentTool } from '@/config/agent-tools'
import type { ToolKind } from '@/types'

interface AgentContextBarProps {
  activeTool: ToolKind
  isArtifactOpen: boolean
  messageCount: number
  sessionId: string | null
  onOpenArtifact: () => void
}

export function AgentContextBar({ activeTool, isArtifactOpen, messageCount, sessionId, onOpenArtifact }: AgentContextBarProps) {
  const tool = getAgentTool(activeTool)
  return (
    <div className="shrink-0 border-b border-surface-100 bg-white/80 px-4 py-3">
      <div className="mb-3 flex items-start justify-between gap-3">
        <div className="min-w-0">
          <div className="flex items-center gap-2 text-sm font-semibold text-surface-900">
            <Sparkles className="h-4 w-4 text-primary-500" />
            当前模式：{tool.name}
          </div>
          <div className="mt-1 line-clamp-2 text-xs leading-relaxed text-surface-500">{tool.description}</div>
        </div>
        {!isArtifactOpen && (
          <button className="btn-secondary h-8 shrink-0 px-2.5 text-xs" onClick={onOpenArtifact}>
            <PanelRightOpen className="h-3.5 w-3.5" />
            打开右侧产物
          </button>
        )}
      </div>
      <div className="grid grid-cols-3 gap-2 text-[11px] text-surface-500">
        <div className="rounded-2xl bg-surface-50 px-3 py-2">
          <div className="mb-1 flex items-center gap-1 font-semibold text-surface-700"><BrainCircuit className="h-3.5 w-3.5 text-primary-500" />上下文</div>
          <div>保留当前会话的意图、约束、产物状态和修改历史。</div>
        </div>
        <div className="rounded-2xl bg-surface-50 px-3 py-2">
          <div className="mb-1 flex items-center gap-1 font-semibold text-surface-700"><Cpu className="h-3.5 w-3.5 text-primary-500" />工具调度</div>
          <div>后续由综合 Agent 自动选择文档、PPT、draw.io、Excel 等工具。</div>
        </div>
        <div className="rounded-2xl bg-surface-50 px-3 py-2">
          <div className="mb-1 flex items-center gap-1 font-semibold text-surface-700"><History className="h-3.5 w-3.5 text-primary-500" />历史</div>
          <div>{messageCount} 条消息 · {sessionId ? '已绑定会话' : '新会话'} · 可沉淀摘要</div>
        </div>
      </div>
      <div className="mt-3 flex items-center gap-2 rounded-2xl bg-primary-50 px-3 py-2 text-xs text-primary-700">
        <FileText className="h-3.5 w-3.5" />
        <span className="font-medium">建议提示：</span>
        <span className="truncate">{tool.examples[0]}</span>
        <ChevronDown className="ml-auto h-3.5 w-3.5 opacity-50" />
      </div>
    </div>
  )
}
