// ===== 工具智能体 =====
export type ToolKind = 'general' | 'ppt' | 'doc' | 'drawio' | 'excel' | 'image' | 'code';

export type ArtifactKind = 'document' | 'markdown' | 'ppt' | 'drawio' | 'sheet' | 'image' | 'code' | 'mixed' | 'search';

export interface ChatAttachment {
  id: string;
  name: string;
  kind: 'text' | 'image';
  mime_type: string;
  size: number;
  text_content?: string;
  data_url?: string;
  original_size?: number;
  width?: number;
  height?: number;
  compressed?: boolean;
}

export interface LLMProfile {
  id: string;
  name: string;
  base_url: string;
  models: string[];
  default_model: string;
  api_key?: string;
  has_api_key?: boolean;
}

export interface BasicSettings {
  app_name: string;
  workspace_title: string;
  brand_tagline: string;
  default_theme: string;
}

export interface MCPServiceConfig {
  id: string;
  name: string;
  transport: string;
  endpoint: string;
  enabled: boolean;
  description?: string;
}

export interface AppSettings {
  llm_profiles: LLMProfile[];
  active_profile_id: string;
  default_model: string;
  active_model: string;
  basic: BasicSettings;
  mcp_servers: MCPServiceConfig[];
  updated_at: string;
}

export interface Artifact {
  id: string;
  kind: ArtifactKind;
  tool_kind: ToolKind;
  title: string;
  status: 'draft' | 'generating' | 'ready' | 'error';
  content: any;
  version: number;
  created_at: string;
  updated_at: string;
}

export interface AgentToolConfig {
  id: ToolKind;
  name: string;
  shortName: string;
  description: string;
  artifactLabel: string;
  promptPlaceholder: string;
  examples: string[];
}

export interface ConversationRecord {
  id: string;
  title: string;
  tool: ToolKind;
  summary?: string;
  updated_at: string;
  message_count: number;
  project_id?: string;
  project_title?: string;
}

export interface ProjectMeta {
  id: string;
  title: string;
  description?: string;
  tool_kind?: ToolKind;
  session_count: number;
  sessions?: ConversationRecord[];
  created_at: string;
  updated_at: string;
}

export interface PersistedSession {
  id: string;
  owner_id: string;
  messages: Array<{
    role: 'user' | 'assistant' | 'system' | 'tool';
    content: string;
    tool_calls?: Array<{ function?: { name?: string; arguments?: string } }>;
    tool_call_id?: string;
    created_at?: string;
  }>;
  artifacts?: Artifact[];
  project_id?: string;
  tool_kind?: ToolKind;
  title: string;
  summary?: string;
  created_at: string;
  updated_at: string;
}

/** 共享类型定义 - aippt.xiaofuge.cn */

// ===== 幻灯片元素 =====
export interface SlideElement {
  type: 'text' | 'image' | 'shape' | 'table' | 'chart';
  x: number;
  y: number;
  w: number;
  h: number;
  text?: string;
  fontSize?: number;
  color?: string;
  bold?: boolean;
  italic?: boolean;
  align?: 'left' | 'center' | 'right';
  valign?: 'top' | 'middle' | 'bottom';
  fill?: string;
  path?: string;
  shape?: string;
  rows?: number;
  cols?: number;
  table_data?: string[][];
  chart_type?: string;
  chart_data?: any;
}

// ===== 幻灯片 =====
export interface Slide {
  id: string;
  index: number;
  layout: 'title' | 'content' | 'two-column' | 'image' | 'chart' | 'section';
  background: string;
  elements: SlideElement[];
  notes?: string;
  title?: string;
}

// ===== 绘制历史 =====
export interface ProjectHistoryEntry {
  id: string;
  type: 'create' | 'plan' | 'draw' | 'layout' | 'export' | 'system';
  title: string;
  detail?: string;
  slide_index?: number;
  slide_title?: string;
  created_at: string;
}

// ===== PPT 项目 =====
export interface PPTProject {
  id: string;
  title: string;
  theme: string;
  slides: Slide[];
  history?: ProjectHistoryEntry[];
  layout: '16x9' | '4x3';
  created_at: string;
  updated_at: string;
  owner_id?: string;
}

// ===== 对话消息 =====
export interface ChatMessage {
  role: 'user' | 'assistant';
  content: string;
  timestamp: string;
  action?: string;
  slide_id?: string;
  attachments?: ChatAttachment[];
}

// ===== 用户 =====
export interface User {
  id: string;
  username: string;
  email?: string;
  avatar?: string;
}

export interface TokenResponse {
  access_token: string;
  token_type: string;
  user: User;
}

// ===== Agent Trace =====
export type AgentTraceKind = 'state' | 'tool' | 'artifact' | 'error';

export interface AgentTraceEvent {
  id: string;
  kind: AgentTraceKind;
  title: string;
  detail?: string;
  status: 'running' | 'success' | 'error';
  tool?: string;
  at: string;
}

// ===== SSE 事件 =====
export interface ChatSSEEvent {
  event: 'message' | 'slide_update' | 'project_update' | 'artifact_update' | 'tool_result' | 'state_update' | 'done' | 'error';
  data: any;
}

// ===== 任务管理 =====
export interface Task {
  id: string;
  title: string;
  description?: string;
  status: 'todo' | 'in_progress' | 'done' | 'archived';
  priority: 'low' | 'medium' | 'high' | 'urgent';
  due_date?: string;
  project_id?: string;
  tags?: string[];
  order: number;
  created_at: string;
  updated_at: string;
}

export interface TaskStats {
  by_status: Record<string, number>;
  by_priority: Record<string, number>;
  due_soon: number;
  total: number;
}

// ===== 文件管理 =====
export interface FileItem {
  id: string;
  name: string;
  file_type: string;
  file_size: number;
  folder_id?: string;
  description?: string;
  metadata?: Record<string, any>;
  created_at: string;
  updated_at: string;
}

export interface Folder {
  id: string;
  owner_id: string;
  name: string;
  parent_id?: string;
  created_at: string;
  updated_at: string;
}

export interface FileStats {
  by_type: Record<string, number>;
  total_size: number;
  total_files: number;
}

// ===== 通知 =====
export interface Notification {
  id: string;
  type: 'system' | 'task' | 'project' | 'message';
  title: string;
  content?: string;
  is_read: boolean;
  link?: string;
  created_at: string;
}

// ===== Dashboard =====
export interface DashboardStats {
  tasks: {
    total: number;
    by_status: Record<string, number>;
    by_priority: Record<string, number>;
    due_soon: number;
  };
  projects: {
    total: number;
    by_kind: Record<string, number>;
  };
  files: {
    total: number;
    by_type: Record<string, number>;
    total_size: number;
  };
  notifications: {
    unread: number;
  };
  recent_sessions: any[];
  recent_projects: any[];
}
