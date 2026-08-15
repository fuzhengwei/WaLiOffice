import type { AgentToolConfig, ToolKind } from '@/types'

export const AGENT_TOOLS: AgentToolConfig[] = [
  {
    id: 'general',
    name: '综合智能体',
    shortName: '综合',
    description: '自动判断任务类型，调度最合适的工具和产物面板',
    artifactLabel: '动态成果',
    promptPlaceholder: '描述你要完成的任务，例如：分析这份材料并生成汇报文档和流程图',
    examples: [
      '帮我把这个想法整理成一份完整方案，并给出执行计划',
      '分析一个产品需求，输出文档、流程图和待办清单',
      '基于这段内容生成汇报材料和数据表结构',
    ],
  },
  {
    id: 'ppt',
    name: 'PPT 生成',
    shortName: 'PPT',
    description: '生成演示文稿、大纲、页面设计与 PPTX 导出',
    artifactLabel: 'PPT 预览',
    promptPlaceholder: '例如：生成一份 8 页的 AI 产品发布会 PPT，科技风',
    examples: [
      '创建一个关于人工智能发展趋势的 PPT',
      '制作一个产品发布会演示文稿',
      '做个关于团队协作的培训 PPT',
    ],
  },
  {
    id: 'doc',
    name: '文档写作',
    shortName: 'docx',
    description: '报告、方案、PRD、纪要、知识库文章与导出',
    artifactLabel: '文档预览',
    promptPlaceholder: '例如：写一份综合 Agent 工作台的产品需求文档',
    examples: [
      '写一份综合 Agent 工作台 PRD',
      '把当前项目整理成技术设计文档',
      '生成一份客户汇报方案，包含背景、目标、路径和里程碑',
    ],
  },
  {
    id: 'drawio',
    name: 'draw.io 绘图',
    shortName: 'draw.io',
    description: '流程图、架构图、泳道图、ER 图，可在右侧编辑',
    artifactLabel: 'draw.io 画布',
    promptPlaceholder: '例如：画一个多工具 Agent 的系统架构图，并用 draw.io 打开',
    examples: [
      '生成一个综合 Agent 工作台的系统架构图',
      '画一个从用户输入到工具调用再到产物展示的流程图',
      '做一张文档、PPT、Excel、draw.io 多工具协作图',
    ],
  },
  {
    id: 'excel',
    name: '在线 Excel',
    shortName: 'Excel',
    description: '在线表格、数据分析、图表、公式和导出 XLSX',
    artifactLabel: '表格工作区',
    promptPlaceholder: '例如：创建一张项目排期表，包含负责人、状态、风险和截止日期',
    examples: [
      '生成一个项目排期表，包含阶段、负责人、状态和截止日期',
      '帮我设计一张销售线索跟进表',
      '分析表格数据并生成图表和结论',
    ],
  },
  {
    id: 'image',
    name: '图像创作',
    shortName: '图像',
    description: '海报、封面、配图、视觉方向和素材生成',
    artifactLabel: '图像结果',
    promptPlaceholder: '例如：为这个 PPT 生成一张科技感封面图',
    examples: [
      '生成一张 AI 工作台的产品封面图',
      '为产品发布会 PPT 设计主视觉方向',
      '做一张适合官网首屏的科技风配图',
    ],
  },
]

export function getAgentTool(id: ToolKind | string): AgentToolConfig {
  return AGENT_TOOLS.find((tool) => tool.id === id) || AGENT_TOOLS[0]
}
