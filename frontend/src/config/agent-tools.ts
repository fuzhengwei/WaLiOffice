import type { AgentToolConfig, ToolKind } from '@/types'

export const AGENT_TOOLS: AgentToolConfig[] = [
  {
    id: 'general',
    name: '综合智能体',
    shortName: '综合',
    description: '自动判断任务类型，调度最合适的工具和产物面板',
    artifactLabel: '动态成果',
    promptPlaceholder: '描述你的业务场景、受众和想要的产物，例如：产品方案、运营复盘、销售汇报、技术设计、培训材料',
    examples: [
      '把“企业知识库 Agent”这个想法整理成一套正式交付材料，包含管理层汇报 PPT、实施方案文档和系统架构图',
      '分析一个会员增长产品需求，输出 PRD、业务流程图和首期迭代表格清单',
      '基于这段销售材料，生成年度经营汇报 PPT、关键数据表和一份 Markdown 调研整理',
      '整理一套新人培训材料，包含培训 PPT、操作手册和学习进度表',
    ],
  },
  {
    id: 'ppt',
    name: 'PPT 生成',
    shortName: 'PPT',
    description: '生成演示文稿、大纲、页面设计与 PPTX 导出',
    artifactLabel: 'PPT 预览',
    promptPlaceholder: '例如：按业务场景生成 PPT，如产品方案、运营复盘、销售汇报、技术架构、培训课件',
    examples: [
      '面向投资人制作一份 AI 办公平台产品发布会 PPT，突出市场机会、产品能力和商业模式',
      '生成一份年度经营分析汇报 PPT，包含目标达成、问题诊断、改进动作和下季度计划',
      '做一份团队协作培训 PPT，适合新人培训，包含原则、流程、案例和落地建议',
      '输出一份技术架构汇报 PPT，讲清模块边界、调用链路、稳定性和实施计划',
    ],
  },
  {
    id: 'doc',
    name: '文档写作',
    shortName: '文档',
    description: '报告、方案、PRD、纪要、知识库文章，支持 Word 与 Markdown 产物',
    artifactLabel: '文档预览',
    promptPlaceholder: '例如：写一份产品 PRD、技术设计文档、运营复盘报告，或输出 Markdown 接入说明',
    examples: [
      '写一份综合 Agent 工作台 PRD，包含背景、目标用户、功能设计、流程、优先级和验收标准',
      '把当前项目整理成技术设计文档，要求适合研发评审和后续实施落地',
      '生成一份客户汇报方案，包含背景、目标、实施路径、里程碑、风险和预期收益',
      '整理一份运营活动复盘报告，包含指标口径、动作拆解、问题诊断和改进建议',
    ],
  },
  {
    id: 'drawio',
    name: 'draw.io 绘图',
    shortName: 'draw.io',
    description: '流程图、架构图、泳道图、ER 图，可在右侧编辑',
    artifactLabel: 'draw.io 画布',
    promptPlaceholder: '例如：画一个产品流程图、运营漏斗图、销售流程图或技术架构图，体现关键关系',
    examples: [
      '生成一个综合 Agent 工作台的系统架构图，体现前端、后端、LLM、工具和产物链路',
      '画一个从用户输入到工具调用再到产物展示的完整流程图，适合方案汇报',
      '做一张文档、PPT、Excel、draw.io 多工具协作图，体现调度关系和数据流转',
      '画一张销售线索到签约的泳道流程图，体现销售、方案、交付三方协同',
    ],
  },
  {
    id: 'excel',
    name: '在线 Excel',
    shortName: 'Excel',
    description: '在线表格、数据分析、图表、公式和导出 XLSX',
    artifactLabel: '表格工作区',
    promptPlaceholder: '例如：创建项目排期表、运营周报表、销售跟进表或需求迭代表，字段要真实可用',
    examples: [
      '生成一个项目排期表，包含阶段、负责人、状态、开始时间、截止日期和风险等级',
      '帮我设计一张销售线索跟进表，包含客户信息、跟进阶段、成交概率和下一步动作',
      '生成一套运营周报表格，包含明细表、汇总表和关键指标分析字段',
      '整理一套产品版本规划表，包含需求池、优先级、版本归属、负责人和验收状态',
    ],
  },
  {
    id: 'image',
    name: '图像创作',
    shortName: '图像',
    description: '海报、封面、配图、视觉方向和素材生成',
    artifactLabel: '图像结果',
    promptPlaceholder: '例如：为产品发布、运营活动、销售方案、技术品牌或培训课程生成主视觉',
    examples: [
      '生成一张 AI 工作台的产品封面图，突出智能协同、效率提升和未来感',
      '为产品发布会 PPT 设计主视觉方向，分别给出写实、插画和 3D 三种风格',
      '做一张适合官网首屏的科技风配图，突出企业办公、数据流和智能体协作',
      '为新人培训课程生成一张封面图，突出学习氛围、办公场景和成长感',
    ],
  },
  {
    id: 'video',
    name: '视频制作',
    shortName: '视频',
    description: '品牌短片、宣传片、产品演示视频和动态主视觉生成',
    artifactLabel: '视频结果',
    promptPlaceholder: '例如：生成产品发布短片、活动宣传视频、官网头图视频或短视频广告，说明场景、镜头和风格',
    examples: [
      '制作一条 AI 工作台产品宣传短片，突出多智能体协作、效率提升和未来科技感',
      '生成一条适合官网首屏的横版视频，展示智能办公、数据流和团队协同场景',
      '做一个活动宣传短视频，适合社媒投放，突出节奏感、氛围感和品牌主视觉',
      '生成一条产品功能演示视频，展示上传资料、生成文档、PPT 和图表的完整链路',
    ],
  },
]

export function getAgentTool(id: ToolKind | string): AgentToolConfig {
  return AGENT_TOOLS.find((tool) => tool.id === id) || AGENT_TOOLS[0]
}
