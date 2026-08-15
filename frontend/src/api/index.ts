import axios from 'axios';
import { useAuthStore } from '@/stores/auth-store';
import type { TokenResponse, ToolKind, Artifact, PersistedSession, AppSettings, MCPServiceConfig, ChatAttachment } from '@/types';

const API_BASE = '/api';

const api = axios.create({
  baseURL: API_BASE,
  headers: { 'Content-Type': 'application/json' },
});

// 请求拦截器：自动添加 token
api.interceptors.request.use((config) => {
  const token = useAuthStore.getState().token;
  if (token) {
    config.headers.Authorization = `Bearer ${token}`;
  }
  return config;
});

// 响应拦截器：处理认证错误
api.interceptors.response.use(
  (res) => res,
  (err) => {
    if (err.response?.status === 401) {
      useAuthStore.getState().logout();
    }
    return Promise.reject(err);
  }
);

// ===== 认证 API =====
export const authApi = {
  login: (username: string, password: string) =>
    api.post<TokenResponse>('/auth/login', { username, password }),
  register: (username: string, email: string, password: string) =>
    api.post<TokenResponse>('/auth/register', { username, email, password }),
  getMe: () => api.get('/auth/me'),
  getDemoAccounts: () => api.get('/auth/demo-accounts'),
  changePassword: (oldPassword: string, newPassword: string) =>
    api.post('/auth/change-password', { old_password: oldPassword, new_password: newPassword }),
};

// ===== PPT API =====
export const pptApi = {
  listProjects: (params?: { q?: string; page?: number; page_size?: number }) =>
    api.get('/ppt/projects', { params }),
  getProject: (id: string) => api.get(`/ppt/project/${id}`),
  createProject: (title: string) => api.post('/ppt/project', { title }),
  updateProject: (id: string, updates: { title?: string; theme?: string }) =>
    api.patch(`/ppt/project/${id}`, updates),
  deleteProject: (id: string) => api.post(`/ppt/project/${id}/delete`),
  getSlides: (id: string) => api.get(`/ppt/project/${id}/slides`),
  exportPptx: (id: string) => api.post(`/ppt/project/${id}/export`, {}, { responseType: 'blob' }),
};

// ===== 会话 API =====
export const sessionApi = {
  listSessions: (params?: { q?: string; page?: number; page_size?: number }) =>
    api.get('/chat/sessions', { params }),
  getSession: (id: string) => api.get<PersistedSession>(`/chat/session/${id}`),
  deleteSession: (id: string) => api.delete(`/chat/session/${id}`),
  clearSession: (id: string) => api.post(`/chat/session/${id}/clear`),
};

// ===== Excel API =====
export const excelApi = {
  exportXlsx: (artifact: Artifact) => api.post('/excel/export', {
    title: artifact.title,
    content: artifact.content,
  }, { responseType: 'blob' }),
};

// ===== Word 文档 API =====
export const docApi = {
  exportDocx: (artifact: Artifact) => api.post('/doc/export', {
    title: artifact.title,
    content: artifact.content,
  }, { responseType: 'blob' }),
};

// ===== 项目 API =====
export const projectApi = {
  listProjects: (params?: { q?: string }) =>
    api.get('/projects', { params }),
  getProject: (id: string) => api.get(`/projects/${id}`),
  createProject: (title: string, tool_kind?: string, description?: string) =>
    api.post('/projects', { title, tool_kind, description }),
  updateProject: (id: string, updates: { title?: string; description?: string; tool_kind?: string }) =>
    api.patch(`/projects/${id}`, updates),
  deleteProject: (id: string) => api.delete(`/projects/${id}`),
  getProjectSessions: (id: string) => api.get(`/projects/${id}/sessions`),
};

// ===== 设置 API =====
export const settingsApi = {
  getSettings: () => api.get<AppSettings>('/settings'),
  saveSettings: (payload: AppSettings) => api.put<AppSettings>('/settings', payload),
  testMcp: (payload: MCPServiceConfig) => api.post('/settings/mcp/test', payload),
};

// ===== 任务 API =====
export const taskApi = {
  list: (params?: { status?: string; priority?: string; project_id?: string; q?: string; page?: number; page_size?: number }) =>
    api.get('/tasks', { params }),
  stats: () => api.get('/tasks/stats'),
  create: (data: { title: string; description?: string; priority?: string; due_date?: string; project_id?: string; tags?: string[] }) =>
    api.post('/tasks', data),
  get: (id: string) => api.get(`/tasks/${id}`),
  update: (id: string, data: { title?: string; description?: string; status?: string; priority?: string; due_date?: string; project_id?: string; tags?: string[]; order?: number }) =>
    api.patch(`/tasks/${id}`, data),
  delete: (id: string) => api.delete(`/tasks/${id}`),
  reorder: (orders: Array<{ id: string; order: number }>) =>
    api.post('/tasks/reorder', { orders }),
};

// ===== 文件 API =====
export const fileApi = {
  list: (folderId?: string) => api.get('/files', { params: { folder_id: folderId } }),
  search: (q: string) => api.get('/files/search', { params: { q } }),
  stats: () => api.get('/files/stats'),
  upload: (file: File, folderId?: string, description?: string) => {
    const formData = new FormData();
    formData.append('file', file);
    const headers: Record<string, string> = { 'Content-Type': 'multipart/form-data' };
    headers['x-filename'] = file.name;
    if (folderId) headers['x-folder-id'] = folderId;
    if (description) headers['x-description'] = description;
    return api.post('/files/upload', formData, { headers });
  },
  get: (id: string) => api.get(`/files/${id}`),
  download: (id: string) => api.get(`/files/${id}/download`, { responseType: 'blob' }),
  delete: (id: string) => api.delete(`/files/${id}`),
};

// ===== 文件夹 API =====
export const folderApi = {
  list: (parentId?: string) => api.get('/files/folders/list', { params: { parent_id: parentId } }),
  create: (name: string, parentId?: string) => api.post('/folders', { name, parent_id: parentId }),
  delete: (id: string) => api.delete(`/folders/${id}`),
};

// ===== 通知 API =====
export const notificationApi = {
  list: (params?: { unread_only?: boolean; page?: number; page_size?: number }) =>
    api.get('/notifications', { params }),
  unreadCount: () => api.get('/notifications/unread'),
  markAsRead: (id: string) => api.post(`/notifications/${id}/read`),
  markAllAsRead: () => api.post('/notifications/read-all'),
  delete: (id: string) => api.delete(`/notifications/${id}`),
};

// ===== Dashboard API =====
export const dashboardApi = {
  stats: () => api.get('/dashboard/stats'),
};

// ===== 对话 API (SSE) =====
export const chatApi = {
  stream: async (
    message: string,
    projectId: string | null,
    sessionId: string | null,
    theme: string | null,
    toolKind: ToolKind,
    model: string | null,
    attachments: ChatAttachment[],
    onEvent: (event: string, data: any) => void,
    token: string,
    signal?: AbortSignal
  ) => {
    const response = await fetch(`${API_BASE}/chat/stream`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${token}`,
      },
      body: JSON.stringify({
        message,
        project_id: projectId,
        session_id: sessionId,
        theme,
        tool_kind: toolKind,
        model,
        attachments,
      }),
      signal,
    });

    if (!response.ok) {
      let detail = `HTTP ${response.status}`;
      try {
        const err = await response.json();
        detail = err.detail || err.message || detail;
      } catch {
        try {
          detail = await response.text();
        } catch {
          detail = `HTTP ${response.status}`;
        }
      }
      if (response.status === 401) {
        useAuthStore.getState().logout();
      }
      throw new Error(detail);
    }

    const reader = response.body?.getReader();
    const decoder = new TextDecoder();

    if (!reader) return;

    let buffer = '';
    let currentEvent = 'message';
    try {
      while (true) {
        const { done, value } = await reader.read();
        if (done) break;

        buffer += decoder.decode(value, { stream: true });
        const lines = buffer.split('\n');
        buffer = lines.pop() || '';

        for (const line of lines) {
          if (line.startsWith('event:')) {
            currentEvent = line.slice(6).trim();
          } else if (line.startsWith('data:')) {
            const dataStr = line.slice(5).trim();
            if (dataStr) {
              try {
                const data = JSON.parse(dataStr);
                onEvent(currentEvent, data);
              } catch {
                onEvent(currentEvent, dataStr);
              }
            }
          } else if (line.trim() === '') {
            currentEvent = 'message';
          }
        }
      }
    } finally {
      reader.releaseLock();
    }
  },
};
