import { useState, useRef, useEffect } from 'react'
import type React from 'react'
import { useNavigate, useSearchParams } from 'react-router-dom'
import { useAuthStore } from '@/stores/auth-store'
import { usePPTStore } from '@/stores/ppt-store'
import { chatApi, docApi, excelApi, pptApi, sessionApi, projectApi, settingsApi, fileApi } from '@/api'
import { ChatPanel } from '@/components/chat/ChatPanel'
import { SlidePreview } from '@/components/preview/SlidePreview'
import { ConversationSidebar } from '@/components/history/ConversationSidebar'
import { ArtifactPanel } from '@/components/artifacts/ArtifactPanel'
import { SettingsDialog } from '@/components/settings/SettingsDialog'
import { AlertCircle, CheckCircle2, Info, Play, X, PanelRightClose, PanelRight, Files, Menu } from 'lucide-react'
import type { AppSettings, Artifact, ChatAttachment, ConversationRecord, InputRef, LLMProfile, PersistedSession, ProjectMeta, ToolKind, ToolConfigMap } from '@/types'
import { extractArtifactSummary } from '@/lib/artifact-summary'
const LOGO_URL = '/logo.png'

type ToastTone = 'success' | 'error' | 'info'

interface ToastState {
  message: string
  tone: ToastTone
}

function buildRestoredMessages(session: PersistedSession) {
  const restored = (session.messages || [])
    .filter((msg) => (msg.role === 'user' || msg.role === 'assistant') && msg.content?.trim())
    .map((msg) => ({
      role: msg.role as 'user' | 'assistant',
      content: msg.content,
      timestamp: msg.created_at || session.updated_at,
    }))

  const hasAssistant = restored.some((msg) => msg.role === 'assistant')
  if (!hasAssistant && session.summary?.trim()) {
    restored.push({
      role: 'assistant',
      content: session.summary,
      timestamp: session.updated_at,
    })
  }

  return restored
}

function buildHistoryProcessLogs(session: PersistedSession) {
  const logs: string[] = [`已恢复会话：${session.title || session.id}`]

  for (const msg of session.messages || []) {
    if (msg.role === 'assistant' && Array.isArray(msg.tool_calls)) {
      for (const call of msg.tool_calls) {
        const toolName = call?.function?.name
        if (toolName) logs.push(`调用工具：${toolName}`)
      }
      continue
    }

    if (msg.role === 'tool' && msg.content) {
      try {
        const payload = JSON.parse(msg.content)
        const detail = payload?.observation || payload?.error || ''
        if (detail) {
          logs.push(`工具执行：${String(detail).slice(0, 120)}`)
        }
      } catch {
        logs.push(`工具执行：${msg.content.slice(0, 120)}`)
      }
    }
  }

  if (session.artifacts?.length) {
    logs.push(`产物汇总：已生成 ${session.artifacts.length} 个产物`)
  } else if (session.summary?.trim()) {
    logs.push(`对话总结：${session.summary.slice(0, 120)}`)
  }

  return logs
}

const IMAGE_ATTACHMENT_MAX_EDGE = 1600
const IMAGE_ATTACHMENT_TARGET_BYTES = 1.8 * 1024 * 1024
const SIDEBAR_MIN_WIDTH = 280
const SIDEBAR_MAX_WIDTH = 520
const SIDEBAR_DEFAULT_WIDTH = 340
const SIDEBAR_WIDTH_STORAGE_KEY = 'walioffice:sidebar-width'

function getStoredSidebarWidth() {
  if (typeof window === 'undefined') return SIDEBAR_DEFAULT_WIDTH
  const stored = Number(window.localStorage.getItem(SIDEBAR_WIDTH_STORAGE_KEY))
  if (!Number.isFinite(stored)) return SIDEBAR_DEFAULT_WIDTH
  return Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, stored))
}

function playConversationDoneSound() {
  try {
    const AudioContextCtor = window.AudioContext || (window as any).webkitAudioContext
    if (!AudioContextCtor) return

    const audioContext = new AudioContextCtor()
    const oscillator = audioContext.createOscillator()
    const gain = audioContext.createGain()
    const now = audioContext.currentTime

    oscillator.type = 'sine'
    oscillator.frequency.setValueAtTime(880, now)
    oscillator.frequency.exponentialRampToValueAtTime(1320, now + 0.12)
    gain.gain.setValueAtTime(0.0001, now)
    gain.gain.exponentialRampToValueAtTime(0.18, now + 0.02)
    gain.gain.exponentialRampToValueAtTime(0.0001, now + 0.22)

    oscillator.connect(gain)
    gain.connect(audioContext.destination)
    oscillator.start(now)
    oscillator.stop(now + 0.24)
    window.setTimeout(() => audioContext.close().catch(() => {}), 320)
  } catch {
    // 浏览器可能会因自动播放策略静音，忽略即可。
  }
}

function isMediaOnlyModel(model?: string) {
  return /^agnes-(image|video)-/i.test(model || '')
}

function pickChatModel(models: string[], fallback = '') {
  return models.find((model) => !isMediaOnlyModel(model)) || fallback
}

function loadImageElement(src: string) {
  return new Promise<HTMLImageElement>((resolve, reject) => {
    const image = new Image()
    image.onload = () => resolve(image)
    image.onerror = () => reject(new Error('加载图片失败'))
    image.src = src
  })
}

function canvasToBlob(canvas: HTMLCanvasElement, type: string, quality?: number) {
  return new Promise<Blob>((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) resolve(blob)
      else reject(new Error('图片压缩失败'))
    }, type, quality)
  })
}

function blobToDataUrl(blob: Blob) {
  return new Promise<string>((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => resolve(typeof reader.result === 'string' ? reader.result : '')
    reader.onerror = () => reject(reader.error || new Error('图片转换失败'))
    reader.readAsDataURL(blob)
  })
}

export default function Studio() {
  const navigate = useNavigate()
  const [searchParams, setSearchParams] = useSearchParams()
  const logout = useAuthStore((s) => s.logout)
  const {
    project, slides, currentSlideIndex, messages,
    isStreaming, sessionId, artifacts, activeArtifactId,
    setProject, setSlides, setCurrentSlide,
    addMessage, setStreaming, setSessionId, reset,
    upsertArtifact, updateArtifact, setActiveArtifact,
    activeTabId, tabs, openTab, closeTab, switchTab, updateTab, restoreState,
  } = usePPTStore()

  const [showArtifactPanel, setShowArtifactPanel] = useState(false)
  const [wideArtifactPanel, setWideArtifactPanel] = useState(false)
  const [activeTool, setActiveTool] = useState<ToolKind>('general')
  const [conversations, setConversations] = useState<ConversationRecord[]>([])
  const [projects, setProjects] = useState<ProjectMeta[]>([])
  const [activeProjectId, setActiveProjectId] = useState<string | null>(null)
  const [conversationQuery, setConversationQuery] = useState('')
  const [showPresent, setShowPresent] = useState(false)
  const [activeView, setActiveView] = useState<'chat' | 'settings'>('chat')
  const [input, setInput] = useState('')
  const [selectedTheme, setSelectedTheme] = useState('default')
  const [followLatestSlide, setFollowLatestSlide] = useState(true)
  const [pptProgress, setPptProgress] = useState<{ current: number; total: number } | null>(null)
  const [streamStatus, setStreamStatus] = useState('空闲')
  const [streamPhase, setStreamPhase] = useState<'idle' | 'thinking' | 'generating' | 'finishing' | 'done' | 'error'>('idle')
  const [processLogs, setProcessLogs] = useState<string[]>([])
  const [attachments, setAttachments] = useState<ChatAttachment[]>([])
  const [inputRefs, setInputRefs] = useState<InputRef[]>([])
  /** 历史会话产物列表（供 @ 引用） */
  const [historyArtifacts, setHistoryArtifacts] = useState<{ artifact: Artifact; sessionTitle: string; sessionId: string }[]>([])
  const [historyArtifactsLoading, setHistoryArtifactsLoading] = useState(false)
  const [toolConfig, setToolConfig] = useState<ToolConfigMap>({})
  const [sidebarWidth, setSidebarWidth] = useState(getStoredSidebarWidth)
  const [mobileSidebarOpen, setMobileSidebarOpen] = useState(false)
  const [toast, setToast] = useState<ToastState | null>(null)
  // 每个 tab 的本地 UI 状态（与 store 中的对话状态分离）
  const [tabUiState, setTabUiState] = useState<Record<string, { input: string; attachments: ChatAttachment[]; inputRefs: InputRef[]; streamStatus: string; streamPhase: typeof streamPhase; processLogs: string[]; activeTool: ToolKind; activeProjectId: string | null; selectedTheme: string; toolConfig: ToolConfigMap; showArtifactPanel: boolean; pptProgress: { current: number; total: number } | null; followLatestSlide: boolean }>>({})
  const tabCounterRef = useRef(0)

  // 设置 & 模型
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [selectedModel, setSelectedModel] = useState<string>('')
  const [modelProfiles, setModelProfiles] = useState<LLMProfile[]>([])

  const activeArtifact = artifacts.find((artifact) => artifact.id === activeArtifactId) || null
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const abortRef = useRef<AbortController | null>(null)
  const autoExportedArtifactIdsRef = useRef<Set<string>>(new Set())
  const autoSavedArtifactIdsRef = useRef<Set<string>>(new Set())
  const attachmentInputRef = useRef<HTMLInputElement>(null)
  const toastTimerRef = useRef<number | null>(null)

  const showToast = (message: string, tone: ToastTone = 'info') => {
    setToast({ message, tone })
    if (toastTimerRef.current) window.clearTimeout(toastTimerRef.current)
    toastTimerRef.current = window.setTimeout(() => {
      setToast(null)
      toastTimerRef.current = null
    }, 3200)
  }

  useEffect(() => {
    return () => {
      if (toastTimerRef.current) window.clearTimeout(toastTimerRef.current)
    }
  }, [])

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  // 加载设置
  const loadSettings = async () => {
    try {
      const res = await settingsApi.getSettings()
      const s = res.data as AppSettings
      setSettings(s)
      const profiles = s.llm_profiles || []
      const activeProfiles = profiles.filter((profile) => profile.id === s.active_profile_id)
      const visibleProfiles = activeProfiles.length > 0 ? activeProfiles : profiles
      setModelProfiles(visibleProfiles)
      const chatModels = Array.from(new Set(visibleProfiles.flatMap((profile) => profile.models || []))).filter((model) => !isMediaOnlyModel(model))
      if (s.active_model && !isMediaOnlyModel(s.active_model)) setSelectedModel(s.active_model)
      else setSelectedModel(pickChatModel(chatModels, selectedModel))
      if (s.basic?.default_theme) setSelectedTheme(s.basic.default_theme)
    } catch (err) {
      console.error('Load settings error:', err)
    }
  }

  // 加载项目列表
  const refreshProjects = async (query = conversationQuery) => {
    try {
      const res = await projectApi.listProjects({ q: query || undefined })
      const projList = (res.data.projects || []).map((item: any) => ({
        id: item.id,
        title: item.title || '未命名项目',
        description: item.description,
        tool_kind: item.tool_kind || 'general',
        session_count: item.session_count || 0,
        sessions: item.sessions || [],
        created_at: item.created_at,
        updated_at: item.updated_at,
      })) as ProjectMeta[]
      setProjects(projList)
    } catch (err) {
      console.error('Load projects error:', err)
      // 如果项目 API 还没就绪，不影响使用
    }
  }

  const refreshConversations = async (query = conversationQuery) => {
    console.log('[refreshConversations] called, query=', query)
    try {
      const res = await sessionApi.listSessions({ q: query || undefined, page: 1, page_size: 50 })
      console.log('[refreshConversations] response:', res.data)
      const rows = (res.data.sessions || []).map((item: any) => ({
        id: item.id,
        title: item.title || '未命名会话',
        tool: item.tool_kind || 'general',
        summary: item.summary,
        updated_at: item.updated_at,
        message_count: item.message_count || 0,
        order_col: item.order_col || 0,
        project_id: item.project_id,
      }))
      console.log('[refreshConversations] parsed rows:', rows.length, rows.slice(0, 2))
      setConversations(rows)
    } catch (err) {
      console.error('[refreshConversations] error:', err)
    }
  }

  useEffect(() => {
    // 初始化第一个 tab
    tabCounterRef.current += 1
    const firstTabId = `tab-${Date.now()}`
    openTab(firstTabId)
    setTabUiState((prev) => ({
      ...prev,
      [firstTabId]: { input: '', attachments: [], inputRefs: [], streamStatus: '空闲', streamPhase: 'idle', processLogs: [], activeTool: 'general', activeProjectId: null, selectedTheme: 'default', toolConfig: {}, showArtifactPanel: false, pptProgress: null, followLatestSlide: true },
    }))

    loadSettings()
    refreshProjects()
    refreshConversations('')
    loadHistoryArtifacts()
    // 从 URL 恢复会话
    const restoreSessionId = searchParams.get('s')
    if (restoreSessionId) {
      handleSelectConversation(restoreSessionId)
    }
  }, [])

  /** 加载历史会话产物供 @ 引用 */
  const loadHistoryArtifacts = async () => {
    try {
      setHistoryArtifactsLoading(true)
      // 获取最近的会话列表
      const res = await sessionApi.listSessions({ page_size: 20 })
      const sessions = (res.data as any)?.sessions || []
      const currentSid = sessionId
      const items: { artifact: Artifact; sessionTitle: string; sessionId: string }[] = []
      // 从每个历史会话中提取产物（排除当前会话）
      for (const session of sessions) {
        if (session.id === currentSid) continue
        if (session.artifacts && Array.isArray(session.artifacts)) {
          for (const artifact of session.artifacts) {
            items.push({
              artifact,
              sessionTitle: session.title || '未命名会话',
              sessionId: session.id,
            })
          }
        } else {
          // 如果会话列表没带产物详情，尝试加载会话详情
          try {
            const detail = await sessionApi.getSession(session.id)
            const detailData = detail.data as any
            const sessionArtifacts = detailData?.artifacts || []
            for (const artifact of sessionArtifacts) {
              items.push({
                artifact,
                sessionTitle: session.title || '未命名会话',
                sessionId: session.id,
              })
            }
          } catch {
            // skip failed sessions
          }
        }
        // 限制最多加载 50 个历史产物
        if (items.length >= 50) break
      }
      setHistoryArtifacts(items)
    } catch (err) {
      console.warn('[historyArtifacts] 加载历史产物失败:', err)
    } finally {
      setHistoryArtifactsLoading(false)
    }
  }

  // sessionId 变化时同步到 URL
  useEffect(() => {
    const current = searchParams.get('s')
    if (sessionId && current !== sessionId) {
      setSearchParams({ s: sessionId }, { replace: true })
    } else if (!sessionId && current) {
      setSearchParams({}, { replace: true })
    }
  }, [sessionId])

  useEffect(() => {
    const timer = window.setTimeout(() => {
      refreshConversations(conversationQuery)
      refreshProjects(conversationQuery)
    }, 250)
    return () => window.clearTimeout(timer)
  }, [conversationQuery])

  const handleToolChange = (tool: ToolKind) => {
    setActiveTool(tool)
  }

  // ====== 多 Tab 管理 ======

  const getCurrentTabUi = () => {
    if (!activeTabId) return null
    return tabUiState[activeTabId] || null
  }

  const updateTabUi = (updates: Partial<NonNullable<ReturnType<typeof getCurrentTabUi>>>) => {
    if (!activeTabId) return
    setTabUiState((prev) => {
      const current = prev[activeTabId] || { input: '', attachments: [], streamStatus: '空闲', streamPhase: 'idle' as const, processLogs: [], activeTool: 'general' as ToolKind, activeProjectId: null, selectedTheme: 'default', toolConfig: {}, showArtifactPanel: false, pptProgress: null, followLatestSlide: true }
      return { ...prev, [activeTabId]: { ...current, ...updates } }
    })
  }

  const handleNewTab = () => {
    // 保存当前 tab 的本地状态
    if (activeTabId) {
      setTabUiState((prev) => {
        const current = prev[activeTabId] || {}
        return { ...prev, [activeTabId]: { ...current, input, attachments, inputRefs, streamStatus, streamPhase, processLogs, activeTool, activeProjectId, selectedTheme, toolConfig, showArtifactPanel, pptProgress, followLatestSlide } }
      })
      updateTab(activeTabId, { input, activeTool, activeProjectId, selectedTheme, toolConfig, streamStatus, streamPhase, processLogs, attachments })
    }

    // 创建新 tab
    tabCounterRef.current += 1
    const newTabId = `tab-${Date.now()}-${tabCounterRef.current}`
    openTab(newTabId)
    setTabUiState((prev) => ({
      ...prev,
      [newTabId]: { input: '', attachments: [], inputRefs: [], streamStatus: '空闲', streamPhase: 'idle', processLogs: [], activeTool: 'general', activeProjectId: null, selectedTheme: selectedTheme, toolConfig: {}, showArtifactPanel: false, pptProgress: null, followLatestSlide: true },
    }))

    // 重置本地 UI 状态
    setInput('')
    setAttachments([])
    setInputRefs([])
    setActiveTool('general')
    setActiveProjectId(null)
    setStreamStatus('空闲')
    setStreamPhase('idle')
    setProcessLogs([])
    setToolConfig({})
    setShowArtifactPanel(false)
    setPptProgress(null)
    setFollowLatestSlide(true)
    setSearchParams({}, { replace: true })
  }

  const handleCloseTab = (tabId: string) => {
    // 如果关闭的是活跃 tab，先保存状态
    if (tabId === activeTabId) {
      setTabUiState((prev) => {
        const current = prev[tabId] || {}
        return { ...prev, [tabId]: { ...current, input, attachments, inputRefs, streamStatus, streamPhase, processLogs, activeTool, activeProjectId, selectedTheme, toolConfig, showArtifactPanel, pptProgress, followLatestSlide } }
      })
    }

    // 如果 tab 正在 streaming，中止
    const tab = tabs[tabId]
    if (tab?.isStreaming) {
      // abortRef 在当前作用域只能控制活跃 tab，后台 tab 的 abort 需要额外管理
      // 简化方案：如果关闭的不是活跃 tab 且正在 streaming，让它后台完成
    }

    closeTab(tabId)
    setTabUiState((prev) => {
      const next = { ...prev }
      delete next[tabId]
      return next
    })

    // 如果关闭的是活跃 tab，恢复到下一个 tab 的状态
    if (tabId === activeTabId) {
      const remainingIds = Object.keys(tabs).filter((id) => id !== tabId)
      if (remainingIds.length > 0) {
        const nextId = remainingIds[remainingIds.length - 1]
        switchTab(nextId)
        const nextUi = tabUiState[nextId]
        if (nextUi) {
          setInput(nextUi.input || '')
          setAttachments(nextUi.attachments || [])
          setInputRefs(nextUi.inputRefs || [])
          setActiveTool(nextUi.activeTool || 'general')
          setActiveProjectId(nextUi.activeProjectId || null)
          setSelectedTheme(nextUi.selectedTheme || 'default')
          setToolConfig(nextUi.toolConfig || {})
          setStreamStatus(nextUi.streamStatus || '空闲')
          setStreamPhase(nextUi.streamPhase || 'idle')
          setProcessLogs(nextUi.processLogs || [])
          setShowArtifactPanel(nextUi.showArtifactPanel || false)
          setPptProgress(nextUi.pptProgress || null)
          setFollowLatestSlide(nextUi.followLatestSlide ?? true)
        }
        const nextTab = tabs[nextId]
        if (nextTab) {
          setSearchParams(nextTab.sessionId ? { s: nextTab.sessionId } : {}, { replace: true })
        }
      }
    }
  }

  const handleSelectTab = (tabId: string) => {
    if (tabId === activeTabId) return

    // 保存当前 tab 的本地 UI 状态
    if (activeTabId) {
      setTabUiState((prev) => {
        const current = prev[activeTabId] || {}
        return { ...prev, [activeTabId]: { ...current, input, attachments, inputRefs, streamStatus, streamPhase, processLogs, activeTool, activeProjectId, selectedTheme, toolConfig, showArtifactPanel, pptProgress, followLatestSlide } }
      })
      updateTab(activeTabId, { input, activeTool, activeProjectId, selectedTheme, toolConfig, streamStatus, streamPhase, processLogs, attachments })
    }

    // 切换到目标 tab
    switchTab(tabId)

    // 恢复目标 tab 的本地 UI 状态
    const targetUi = tabUiState[tabId]
    if (targetUi) {
      setInput(targetUi.input || '')
      setAttachments(targetUi.attachments || [])
      setInputRefs(targetUi.inputRefs || [])
      setActiveTool(targetUi.activeTool || 'general')
      setActiveProjectId(targetUi.activeProjectId || null)
      setSelectedTheme(targetUi.selectedTheme || 'default')
      setToolConfig(targetUi.toolConfig || {})
      setStreamStatus(targetUi.streamStatus || '空闲')
      setStreamPhase(targetUi.streamPhase || 'idle')
      setProcessLogs(targetUi.processLogs || [])
      setShowArtifactPanel(targetUi.showArtifactPanel || false)
      setPptProgress(targetUi.pptProgress || null)
      setFollowLatestSlide(targetUi.followLatestSlide ?? true)
    }

    // 同步 URL
    const targetTab = tabs[tabId]
    if (targetTab) {
      setSearchParams(targetTab.sessionId ? { s: targetTab.sessionId } : {}, { replace: true })
    }
  }

  // 构建传给侧边栏的 tabs 数据
  const conversationTabs = Object.values(tabs).map((tab) => ({
    id: tab.tabId,
    title: tab.tabTitle || (tab.messages.length > 0 ? (tab.messages.find((m) => m.role === 'user')?.content || '新对话').slice(0, 20) : '新对话'),
    tool: tab.activeTool || 'general',
    isStreaming: tab.isStreaming,
    streamPhase: tab.streamPhase,
    streamStatus: tab.streamStatus,
    messageCount: tab.messages.length,
    hasArtifacts: tab.artifacts.length > 0,
  }))

  const handleNewProject = async () => {
    if (isStreaming) return
    setActiveView('chat')
    const title = prompt('请输入项目名称：')
    if (!title?.trim()) return
    try {
      const res = await projectApi.createProject(title.trim(), activeTool)
      const newProj = res.data
      setActiveProjectId(newProj.id)
      setActiveTool(newProj.tool_kind || 'general')
      reset()
      autoExportedArtifactIdsRef.current.clear()
      setShowArtifactPanel(false)
      setFollowLatestSlide(true)
      setPptProgress(null)
      autoSavedArtifactIdsRef.current.clear()
      refreshProjects()
    } catch (err) {
      console.error('Create project error:', err)
      showToast('创建项目失败', 'error')
    }
  }

  const handleSelectProject = async (projectId: string) => {
    // 如果正在 streaming，先中止
    if (isStreaming) {
      abortRef.current?.abort()
      setStreaming(false)
      setStreamPhase('idle')
      setStreamStatus('已停止生成')
      if (activeTabId) updateTab(activeTabId, { isStreaming: false, streamPhase: 'idle', streamStatus: '已停止生成' })
    }
    setActiveView('chat')
    setActiveProjectId(projectId)
    refreshProjects()
  }

  const handleDeleteProject = async (projectId: string) => {
    try {
      await projectApi.deleteProject(projectId)
      if (activeProjectId === projectId) {
        setActiveProjectId(null)
      }
      refreshProjects()
    } catch (err) {
      console.error('Delete project error:', err)
      showToast('删除项目失败', 'error')
    }
  }

  const handleNewConversation = () => {
    // 如果正在 streaming，先中止当前请求再切换
    if (isStreaming) {
      abortRef.current?.abort()
      setStreaming(false)
      setStreamPhase('idle')
      setStreamStatus('已停止生成')
      if (activeTabId) updateTab(activeTabId, { isStreaming: false, streamPhase: 'idle', streamStatus: '已停止生成' })
    }
    setActiveView('chat')
    refreshConversations()
    reset()
    // 重置当前 tab 的标题
    if (activeTabId) {
      updateTab(activeTabId, { tabTitle: '新对话', sessionId: null })
    }
    setSearchParams({}, { replace: true })
    autoExportedArtifactIdsRef.current.clear()
    autoSavedArtifactIdsRef.current.clear()
    setActiveTool('general')
    setShowArtifactPanel(false)
    setFollowLatestSlide(true)
    setPptProgress(null)
    setStreamPhase('idle')
    setStreamStatus('空闲')
    setProcessLogs([])
    setAttachments([])
  }

  const handleSelectConversation = async (id: string) => {
    // 如果正在 streaming，先中止当前请求再切换
    if (isStreaming) {
      abortRef.current?.abort()
      setStreaming(false)
      setStreamPhase('idle')
      setStreamStatus('已停止生成')
      if (activeTabId) updateTab(activeTabId, { isStreaming: false, streamPhase: 'idle', streamStatus: '已停止生成' })
    }
    setActiveView('chat')
    try {
      const res = await sessionApi.getSession(id)
      const session = res.data as PersistedSession
      const tool = session.tool_kind || 'general'
      setActiveTool(tool)
      setSessionId(session.id)
      if (session.project_id) {
        setActiveProjectId(session.project_id)
      }
      usePPTStore.setState({
        messages: buildRestoredMessages(session),
        artifacts: session.artifacts || [],
        activeArtifactId: session.artifacts?.[0]?.id || null,
        isGenerating: false,
        isStreaming: false,
      })
      // project_id 可能是通用项目 ID 也可能是 PPT 项目 ID
      if (session.project_id) {
        try {
          // 先尝试通用项目 API
          await projectApi.getProject(session.project_id)
          setActiveProjectId(session.project_id)
          usePPTStore.setState({ project: null, slides: [], currentSlideIndex: 0 })
        } catch {
          // 可能是 PPT 项目 ID（老数据兼容）
          try {
            const pptRes = await pptApi.getProject(session.project_id)
            setProject(pptRes.data)
            setSlides(pptRes.data.slides || [])
            setCurrentSlide(0)
          } catch {
            usePPTStore.setState({ project: null, slides: [], currentSlideIndex: 0 })
          }
        }
      } else {
        usePPTStore.setState({ project: null, slides: [], currentSlideIndex: 0 })
      }
      setShowArtifactPanel((session.artifacts?.length || 0) > 0 || tool !== 'general')
      setFollowLatestSlide(false)
      setPptProgress(null)
      setStreamPhase('done')
      setStreamStatus('已恢复历史会话')
      setProcessLogs(buildHistoryProcessLogs(session))
      // 更新当前 tab 元信息
      if (activeTabId) {
        updateTab(activeTabId, {
          tabTitle: session.title?.slice(0, 24) || '历史会话',
          sessionId: session.id,
          activeTool: tool,
          activeProjectId: session.project_id || null,
          streamPhase: 'done',
          streamStatus: '已恢复历史会话',
        })
      }
    } catch (err) {
      console.error('Select conversation error:', err)
      const errMsg = err instanceof Error ? err.message : '未知错误'
      showToast(`恢复历史会话失败：${errMsg}`, 'error')
    }
  }

  const handleRenameConversation = async (id: string, title: string) => {
    try {
      await sessionApi.updateSession(id, { title })
      refreshConversations()
    } catch (err) {
      console.error('Rename conversation error:', err)
      showToast('修改对话标题失败', 'error')
    }
  }

  const handleDeleteConversation = async (id: string) => {
    if (!confirm('确认删除该对话？此操作不可恢复。')) return
    try {
      await sessionApi.deleteSession(id)
      if (sessionId === id) {
        reset()
        setSearchParams({}, { replace: true })
      }
      refreshConversations()
      refreshProjects()
    } catch (err) {
      console.error('Delete conversation error:', err)
      showToast('删除对话失败', 'error')
    }
  }

  const handleMoveConversation = async (id: string, projectId: string | null, beforeId?: string | null) => {
    const moving = conversations.find((item) => item.id === id)
    if (!moving) return

    const targetItems = conversations
      .filter((item) => item.id !== id && (item.project_id || null) === projectId)
      .sort((a, b) => (a.order_col || 0) - (b.order_col || 0))
    const beforeIndex = beforeId ? targetItems.findIndex((item) => item.id === beforeId) : -1
    const insertIndex = beforeIndex >= 0 ? beforeIndex : targetItems.length
    targetItems.splice(insertIndex, 0, { ...moving, project_id: projectId || undefined })
    const orderMap = new Map(targetItems.map((item, index) => [item.id, (index + 1) * 1000]))

    const nextConversations = conversations.map((item) => {
      if (item.id === id) return { ...item, project_id: projectId || undefined, order_col: orderMap.get(item.id) || item.order_col }
      if (orderMap.has(item.id)) return { ...item, order_col: orderMap.get(item.id) }
      return item
    })
    setConversations(nextConversations)

    try {
      await Promise.all(targetItems.map((item) => sessionApi.updateSession(item.id, {
        project_id: item.id === id ? projectId : item.project_id || null,
        order_col: orderMap.get(item.id) || 0,
      })))
      refreshConversations()
      refreshProjects()
    } catch (err) {
      console.error('Move conversation error:', err)
      showToast('移动对话失败', 'error')
      refreshConversations()
      refreshProjects()
    }
  }

  const handleSaveSettings = async (newSettings: AppSettings) => {
    try {
      const res = await settingsApi.saveSettings(newSettings)
      const saved = res.data as AppSettings
      setSettings(saved)
      const profiles = saved.llm_profiles || []
      const activeProfiles = profiles.filter((profile) => profile.id === saved.active_profile_id)
      const visibleProfiles = activeProfiles.length > 0 ? activeProfiles : profiles
      setModelProfiles(visibleProfiles)
      const chatModels = Array.from(new Set(visibleProfiles.flatMap((profile) => profile.models || []))).filter((model) => !isMediaOnlyModel(model))
      if (saved.active_model && !isMediaOnlyModel(saved.active_model)) setSelectedModel(saved.active_model)
      else setSelectedModel(pickChatModel(chatModels, selectedModel))
      if (saved.basic?.default_theme) setSelectedTheme(saved.basic.default_theme)
      setActiveView('chat')
    } catch (err: any) {
      console.error('Save settings error:', err)
      showToast(err.response?.data?.detail || err.message || '设置保存失败', 'error')
      throw err
    }
  }

  const handleModelChange = (model: string) => {
    setSelectedModel(model)
  }

  const handleSelectSlide = (index: number) => {
    setCurrentSlide(index)
    const slideCount = usePPTStore.getState().slides.length
    setFollowLatestSlide(index >= Math.max(0, slideCount - 1))
  }

  const readFileAsText = (file: File) =>
    new Promise<string>((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = () => resolve(typeof reader.result === 'string' ? reader.result : '')
      reader.onerror = () => reject(reader.error || new Error(`读取文件失败：${file.name}`))
      reader.readAsText(file, 'utf-8')
    })

  const readFileAsDataUrl = (file: File) =>
    new Promise<string>((resolve, reject) => {
      const reader = new FileReader()
      reader.onload = () => resolve(typeof reader.result === 'string' ? reader.result : '')
      reader.onerror = () => reject(reader.error || new Error(`读取图片失败：${file.name}`))
      reader.readAsDataURL(file)
    })

  const buildImageAttachment = async (file: File): Promise<ChatAttachment> => {
    const originalDataUrl = await readFileAsDataUrl(file)

    if (file.type === 'image/svg+xml' || file.type === 'image/gif') {
      return {
        id: `${Date.now()}-${crypto.randomUUID()}`,
        name: file.name,
        kind: 'image',
        mime_type: file.type || 'image/png',
        size: file.size,
        data_url: originalDataUrl,
        original_size: file.size,
        compressed: false,
      }
    }

    const image = await loadImageElement(originalDataUrl)
    const scale = Math.min(1, IMAGE_ATTACHMENT_MAX_EDGE / Math.max(image.naturalWidth || 1, image.naturalHeight || 1))
    const width = Math.max(1, Math.round((image.naturalWidth || 1) * scale))
    const height = Math.max(1, Math.round((image.naturalHeight || 1) * scale))

    const canvas = document.createElement('canvas')
    canvas.width = width
    canvas.height = height

    const context = canvas.getContext('2d')
    if (!context) {
      return {
        id: `${Date.now()}-${crypto.randomUUID()}`,
        name: file.name,
        kind: 'image',
        mime_type: file.type || 'image/png',
        size: file.size,
        data_url: originalDataUrl,
        original_size: file.size,
        width: image.naturalWidth,
        height: image.naturalHeight,
        compressed: false,
      }
    }

    const preferredMime = file.type === 'image/webp'
      ? 'image/webp'
      : file.type === 'image/png' && file.size <= IMAGE_ATTACHMENT_TARGET_BYTES
        ? 'image/png'
        : 'image/jpeg'

    if (preferredMime === 'image/jpeg') {
      context.fillStyle = '#ffffff'
      context.fillRect(0, 0, width, height)
    }
    context.drawImage(image, 0, 0, width, height)

    const qualities = preferredMime === 'image/png' ? [undefined] : [0.92, 0.86, 0.8, 0.72, 0.64]
    let bestBlob: Blob | null = null

    for (const quality of qualities) {
      const blob = await canvasToBlob(canvas, preferredMime, quality)
      if (!bestBlob || blob.size < bestBlob.size) bestBlob = blob
      if (blob.size <= IMAGE_ATTACHMENT_TARGET_BYTES) {
        bestBlob = blob
        break
      }
    }

    const finalBlob = bestBlob || file
    const finalDataUrl = finalBlob === file ? originalDataUrl : await blobToDataUrl(finalBlob)

    return {
      id: `${Date.now()}-${crypto.randomUUID()}`,
      name: file.name,
      kind: 'image',
      mime_type: finalBlob.type || file.type || 'image/png',
      size: finalBlob.size,
      data_url: finalDataUrl,
      original_size: file.size,
      width: image.naturalWidth,
      height: image.naturalHeight,
      compressed: finalBlob.size < file.size || scale < 1,
    }
  }

  const buildAttachmentFromFile = async (file: File): Promise<ChatAttachment | null> => {
    const lowerName = file.name.toLowerCase()
    const isMarkdown = lowerName.endsWith('.md') || file.type === 'text/markdown'
    const isText = lowerName.endsWith('.txt') || file.type === 'text/plain'
    const isImage = file.type.startsWith('image/')
    const isOfficeText = /\.(docx|xlsx|pptx|pdf|csv|tsv|json)$/i.test(file.name)

    if (isMarkdown || isText) {
      const textContent = await readFileAsText(file)
      return {
        id: `${Date.now()}-${crypto.randomUUID()}`,
        name: file.name,
        kind: 'text',
        mime_type: file.type || (isMarkdown ? 'text/markdown' : 'text/plain'),
        size: file.size,
        text_content: textContent.slice(0, 20000),
      }
    }

    if (isImage) {
      return buildImageAttachment(file)
    }

    if (isOfficeText) {
      const res = await fileApi.extract(file)
      const text = String(res.data?.text || '').trim()
      if (!text) return null
      const parser = res.data?.parser ? `解析器：${res.data.parser}` : '解析器：server'
      const truncated = res.data?.truncated ? '（内容较长，已截断）' : ''
      return {
        id: `${Date.now()}-${crypto.randomUUID()}`,
        name: file.name,
        kind: 'text',
        mime_type: file.type || 'application/octet-stream',
        size: file.size,
        text_content: `【附件：${file.name}】${truncated}\n${parser}\n\n${text}`.slice(0, 50000),
      }
    }

    return null
  }

  const handlePickAttachments = () => {
    if (isStreaming) return
    attachmentInputRef.current?.click()
  }

  const handleAttachmentChange = async (event: React.ChangeEvent<HTMLInputElement>) => {
    const files = Array.from(event.target.files || [])
    event.target.value = ''

    if (files.length === 0) return

    try {
      const nextItems = (await Promise.all(files.map(buildAttachmentFromFile))).filter(Boolean) as ChatAttachment[]
      const unsupported = files.length - nextItems.length
      if (unsupported > 0) {
        showToast('目前支持上传 md、txt、csv、json、docx、xlsx、pptx、pdf 和图片文件。', 'error')
      }
      if (nextItems.length === 0) return

      await Promise.allSettled(
        nextItems.map((item) => {
          const source = files.find((file) => file.name === item.name && file.size === item.size)
          if (!source) return Promise.resolve()
          return fileApi.upload(source, undefined, '聊天上传附件')
        })
      )

      setAttachments((current) => {
        const merged = [...current, ...nextItems]
        const deduped = merged.filter((item, index, arr) => arr.findIndex((target) => target.name === item.name && target.size === item.size) === index)
        return deduped.slice(0, 6)
      })
      showToast(`已添加 ${nextItems.length} 个附件，并保存到我的文件`, 'success')
    } catch (err) {
      console.error('Read attachment error:', err)
      showToast('读取附件失败，请检查文件编码或重新选择文件。', 'error')
    }
  }

  const handleRemoveAttachment = (id: string) => {
    setAttachments((current) => current.filter((item) => item.id !== id))
  }

  const handleInsertArtifactToInput = (artifact: Artifact, fromSessionId?: string) => {
    if (isStreaming) return

    // 图片类型：加入附件 + markdown 引用（保持原有逻辑）
    if (artifact.kind === 'image' && Array.isArray(artifact.content?.images) && artifact.content.images.length > 0) {
      const newAttachments: ChatAttachment[] = artifact.content.images.slice(0, 3).map((url: string, index: number) => ({
        id: `artifact-${artifact.id}-${index}-${Date.now()}`,
        name: `${artifact.title || '图片'}-${index + 1}.png`,
        kind: 'image' as const,
        mime_type: 'image/png',
        size: 0,
        data_url: url,
        from_artifact: true,
      }))
      setAttachments((current) => {
        const merged = [...current, ...newAttachments]
        const deduped = merged.filter((item, index, arr) => arr.findIndex((t) => t.data_url === item.data_url && t.data_url) === index)
        return deduped.slice(0, 6)
      })
      const refText = artifact.content.images.map((url: string, index: number) => `![${artifact.title || '图片'}-${index + 1}](${url})`).join('\n')
      setInput((current) => (current.trim() ? `${current}\n\n${refText}` : refText))
      return
    }

    // 视频类型：走 InputRef chip 标签路径（与其他产物类型一致）
    if (artifact.kind === 'video' && artifact.content?.video_url) {
      const refText = `[视频：${artifact.title || '未命名'}](${artifact.content.video_url})`
      const newRef: InputRef = {
        id: `ref-${artifact.id}-${Date.now()}`,
        artifactId: artifact.id,
        kind: artifact.kind,
        title: artifact.title || '未命名',
        refText,
        sessionId: fromSessionId,
        contentSummary: extractArtifactSummary(artifact),
      }
      setInputRefs((current) => {
        if (current.some((ref) => ref.artifactId === artifact.id)) return current
        return [...current, newRef]
      })
      return
    }

    // 其他产物类型：添加为输入框引用标签 chip
    let refText = ''
    if (artifact.kind === 'ppt') {
      const slideCount = artifact.content?.slides?.length || artifact.content?.slide_count || 0
      refText = `[PPT：${artifact.title || '未命名'}，共 ${slideCount} 页]`
    } else if (artifact.kind === 'document' || artifact.kind === 'markdown') {
      refText = `[${artifact.kind === 'document' ? '文档' : 'MD'}：${artifact.title || '未命名'}]`
    } else if (artifact.kind === 'sheet') {
      refText = `[表格：${artifact.title || '未命名'}]`
    } else if (artifact.kind === 'drawio') {
      refText = `[图表：${artifact.title || '未命名'}]`
    } else {
      refText = `[${artifact.title || artifact.kind || '产物'}]`
    }

    const newRef: InputRef = {
      id: `ref-${artifact.id}-${Date.now()}`,
      artifactId: artifact.id,
      kind: artifact.kind,
      title: artifact.title || '未命名',
      refText,
      sessionId: fromSessionId,
      contentSummary: extractArtifactSummary(artifact),
    }
    setInputRefs((current) => {
      // 去重：同一产物不重复添加
      if (current.some((ref) => ref.artifactId === artifact.id)) return current
      return [...current, newRef]
    })
  }

  const handleRemoveInputRef = (refId: string) => {
    setInputRefs((current) => current.filter((ref) => ref.id !== refId))
  }

  const applyRealtimePptState = (payload: {
    project_id?: string
    title?: string
    theme?: string
    slides?: any[]
    history?: any[]
    slide_count?: number
    total_slides?: number
    current_index?: number
  }) => {
    const currentProject = usePPTStore.getState().project
    const currentSlides = usePPTStore.getState().slides
    const nextSlides = Array.isArray(payload.slides) ? payload.slides : currentProject?.slides || []
    const nextTheme = payload.theme || currentProject?.theme || selectedTheme
    const nextProjectId = payload.project_id || currentProject?.id || `ppt-${Date.now()}`

    if (payload.project_id) {
      setActiveProjectId(payload.project_id)
    }

    setActiveTool('ppt')
    setShowArtifactPanel(true)
    setProject({
      id: nextProjectId,
      title: payload.title || currentProject?.title || '未命名演示文稿',
      theme: nextTheme,
      slides: nextSlides,
      history: payload.history || currentProject?.history || [],
      layout: currentProject?.layout || '16x9',
      created_at: currentProject?.created_at || new Date().toISOString(),
      updated_at: new Date().toISOString(),
      owner_id: currentProject?.owner_id || useAuthStore.getState().user?.id,
    })
    setSlides(nextSlides)
    if (nextSlides.length === 0) {
      setCurrentSlide(0)
    } else if (followLatestSlide && nextSlides.length >= currentSlides.length) {
      setCurrentSlide(nextSlides.length - 1)
    } else {
      setCurrentSlide(Math.min(usePPTStore.getState().currentSlideIndex, nextSlides.length - 1))
    }
    if (typeof payload.total_slides === 'number') {
      setPptProgress({
        current: typeof payload.slide_count === 'number' ? payload.slide_count : nextSlides.length,
        total: payload.total_slides,
      })
    }
  }

  const applyPptArtifact = (artifact: Artifact) => {
    applyRealtimePptState({
      project_id: artifact.content?.project_id,
      title: artifact.content?.title || artifact.title,
      theme: artifact.content?.theme,
      slides: artifact.content?.slides,
      history: artifact.content?.history,
    })
    if (Array.isArray(artifact.content?.slides) && artifact.content.slides.length > 0) {
      setCurrentSlide(0)
    }
  }

  const inferToolFromMessage = (text: string, pendingAttachments: ChatAttachment[] = []): ToolKind => {
    const lower = text.toLowerCase()
    const hits: ToolKind[] = []
    const hasImageAttachment = pendingAttachments.some((item) => item.kind === 'image')
    const hasImageRecognitionIntent = /这是什么|识别|识图|看图|帮我看看|图里|图片里|截图里|读图|ocr|提取文字|解析图片|说明图片|分析图片|描述图片/.test(lower)
    const hasImageGenerationIntent = /生成.*图|做.*图|画.*图|出图|图生图|以图生图|基于.*图.*图|参考.*图.*图|基于图片|基于这张图|基于这个图|基于照片|参考图片|参考这张图|用这张图|按照这张图|改图|修图|重绘|换风格|换背景|换衣服|换装|变装|换发型|去除背景|抠图|扩图|其他穿着|穿着|衣服|服装|造型|换成|改成|海报|封面|logo|配图|主视觉|插画|banner|视觉稿|图象创作|图像创作/.test(lower)
    const hasVideoGenerationIntent = /生成.*视频|做.*视频|制作.*视频|图生视频|以图生视频|基于.*图.*视频|参考.*图.*视频|让.*图.*动|让.*照片.*动|动起来|动态化|短片|短视频|宣传片|动画|视频广告|片头|转场|动态海报|mv|motion/.test(lower)

    if (/draw\.io|drawio|流程图|架构图|泳道图|拓扑图|er图/.test(lower)) hits.push('drawio')
    if (/excel|xlsx|表格|数据分析|公式|在线表/.test(lower)) hits.push('excel')
    if (/文档|报告|prd|方案|纪要|文章|docx|markdown|readme|知识库|说明文档|操作手册|md\b/.test(lower)) hits.push('doc')
    if (/ppt|演示文稿|幻灯片|presentation|做个.*汇报|生成.*汇报|制作.*汇报|汇报材料/.test(lower)) hits.push('ppt')
    if (hasImageGenerationIntent || (/图片|图象|图像/.test(lower) && !hasImageRecognitionIntent && !hasImageAttachment)) hits.push('image')
    if (hasVideoGenerationIntent || /视频|video/.test(lower)) hits.push('video')
    const wantsMultiple = /同时|一起|并且|再来|外加|附上|配一张|再补一个|多个|一套/.test(lower)
    const uniqueHits = Array.from(new Set(hits))
    if (uniqueHits.includes('video') && uniqueHits.includes('image') && hasVideoGenerationIntent && !wantsMultiple) return 'video'
    if (hasImageAttachment && !hasImageGenerationIntent) return 'general'
    if (uniqueHits.length > 1 || (wantsMultiple && uniqueHits.length > 0)) return 'general'
    if (uniqueHits.length === 1) return uniqueHits[0]
    return activeTool
  }

  const handleSend = async () => {
    if ((!input.trim() && attachments.length === 0 && inputRefs.length === 0) || isStreaming) return

    const hasImageAttachment = attachments.some((item) => item.kind === 'image')
    const baseMessage = input.trim() || (hasImageAttachment ? '请识别并说明我上传图片的主要内容。' : '请结合我上传的文件内容继续处理。')
    // 将 @ 引用标签拼接 + 产物内容摘要作为上下文提供给 AI
    const refsText = inputRefs.map((ref) => ref.refText).join('\n')
    const refSummaries = inputRefs
      .filter((ref) => ref.contentSummary)
      .map((ref) => ref.contentSummary!)
    const refContext = refSummaries.length > 0
      ? `\n\n--- 引用产物内容 ---\n${refSummaries.join('\n\n')}\n--- 引用产物内容结束 ---`
      : ''
    const message = (refsText || refContext)
      ? (baseMessage.trim() ? `${baseMessage}\n\n${refsText}${refContext}` : `${refsText}${refContext}`)
      : baseMessage
    const pendingAttachments = attachments
    const inferredTool = inferToolFromMessage(message, pendingAttachments)
    if (inferredTool !== activeTool) setActiveTool(inferredTool)
    setInput('')
    setAttachments([])
    setInputRefs([])
    setStreaming(true)
    setFollowLatestSlide(true)
    setPptProgress(null)
    setStreamPhase('thinking')
    setStreamStatus(pendingAttachments.length > 0 ? '正在整理消息与附件...' : '正在理解需求...')
    setProcessLogs([
      '开始处理请求',
      `识别工具：${inferredTool}`,
      ...(pendingAttachments.length > 0 ? [`附件：已接收 ${pendingAttachments.length} 个文件`] : []),
    ])
    // 同步到 store tab 状态
    if (activeTabId) {
      updateTab(activeTabId, {
        isStreaming: true,
        streamPhase: 'thinking',
        streamStatus: pendingAttachments.length > 0 ? '正在整理消息与附件...' : '正在理解需求...',
        activeTool: inferredTool,
        tabTitle: message.slice(0, 24) || '新对话',
      })
    }
    const abortController = new AbortController()
    abortRef.current = abortController

    addMessage({
      role: 'user',
      content: baseMessage,
      timestamp: new Date().toISOString(),
      attachments: pendingAttachments,
      inputRefs: inputRefs.length > 0 ? inputRefs : undefined,
    })

    // 发给后端的消息仍包含完整引用上下文

    const token = useAuthStore.getState().token
    if (!token) {
      setAttachments(pendingAttachments)
      setStreaming(false)
      setStreamPhase('error')
      setStreamStatus('登录失效')
      addMessage({
        role: 'assistant',
        content: '登录状态已失效，请重新登录后再试。',
        timestamp: new Date().toISOString(),
      })
      useAuthStore.getState().logout()
      return
    }

    let assistantText = ''
    addMessage({
      role: 'assistant',
      content: '',
      timestamp: new Date().toISOString(),
    })

    try {
      await chatApi.stream(
        message,
        activeProjectId,
        sessionId,
        selectedTheme,
        inferredTool,
        selectedModel,
        pendingAttachments,
        (event, data) => {
          switch (event) {
            case 'message':
              setStreamPhase(data.start ? 'thinking' : 'finishing')
              setStreamStatus(data.start ? '正在连接模型...' : '正在整理回复...')
              if (data.text) {
                assistantText += data.text
                usePPTStore.setState((state) => {
                  const msgs = [...state.messages]
                  msgs[msgs.length - 1] = {
                    ...msgs[msgs.length - 1],
                    content: assistantText,
                  }
                  // 同时更新当前 tab 的 messages，防止 done 事件中 syncFromTab 用旧 tab 数据覆盖全局
                  const tabId = state.activeTabId
                  if (tabId && state.tabs[tabId]) {
                    return {
                      messages: msgs,
                      tabs: { ...state.tabs, [tabId]: { ...state.tabs[tabId], messages: msgs } },
                    }
                  }
                  return { messages: msgs }
                })
              }
              if (data.session_id) {
                const prevSessionId = sessionId
                setSessionId(data.session_id)
                // 更新 tab 标题
                if (activeTabId && !prevSessionId) {
                  const userMsg = message.slice(0, 24)
                  updateTab(activeTabId, { tabTitle: userMsg, sessionId: data.session_id })
                  // 首次获得 session_id 时立即刷新列表，让当前对话出现在侧边栏
                  refreshConversations()
                  refreshProjects()
                }
              }
              if (data.project_id && !project) {
                pptApi.getProject(data.project_id).then(({ data: proj }) => {
                  setProject(proj)
                  setSlides(proj.slides || [])
                })
              }
              break

            case 'project_update':
              setActiveTool('ppt')
              setShowArtifactPanel(true)
              setStreamPhase('generating')
              setStreamStatus(
                typeof data.total_slides === 'number'
                  ? `已创建项目，准备生成 1 / ${data.total_slides} 页...`
                  : '已创建项目，正在生成大纲...'
              )
              applyRealtimePptState(data)
              if (data.theme) setSelectedTheme(data.theme)
              break

            case 'slide_update':
              setActiveTool('ppt')
              setShowArtifactPanel(true)
              setStreamPhase('generating')
              setStreamStatus(
                typeof data.slide_count === 'number' && typeof data.total_slides === 'number'
                  ? `正在生成第 ${data.slide_count} / ${data.total_slides} 页...`
                  : `正在更新幻灯片${data.slide_count ? `（${data.slide_count} 页）` : ''}...`
              )
              if (data.slides) {
                applyRealtimePptState(data)
              }
              break

            case 'artifact_update':
              if (data.artifact) {
                upsertArtifact(data.artifact)
                setActiveTool(inferredTool === 'general' ? 'general' : (data.artifact.tool_kind || inferredTool))
                setShowArtifactPanel(true)
                setStreamPhase(data.artifact.status === 'ready' ? 'finishing' : 'generating')
                setStreamStatus(`已更新产物：${data.artifact.title || '未命名产物'}`)
                if (data.artifact.kind === 'ppt') {
                  applyPptArtifact(data.artifact)
                  if (typeof data.artifact.content?.total_slides === 'number') {
                    setPptProgress({
                      current: data.artifact.content?.slide_count || data.artifact.content?.slides?.length || 0,
                      total: data.artifact.content.total_slides,
                    })
                  }
                }
                if (
                  data.artifact.kind === 'sheet' &&
                  data.artifact.content?.export_requested &&
                  !autoExportedArtifactIdsRef.current.has(data.artifact.id)
                ) {
                  autoExportedArtifactIdsRef.current.add(data.artifact.id)
                  setProcessLogs((logs) => [...logs.slice(-8), 'Excel：正在自动导出 XLSX'])
                  handleExportExcel(data.artifact).catch((err) => {
                    console.error('Auto Excel export error:', err)
                    setProcessLogs((logs) => [...logs.slice(-8), 'Excel：自动导出失败，请点击右侧按钮重试'])
                  })
                }
                if (
                  data.artifact.kind === 'document' &&
                  data.artifact.content?.export_requested &&
                  !autoExportedArtifactIdsRef.current.has(data.artifact.id)
                ) {
                  autoExportedArtifactIdsRef.current.add(data.artifact.id)
                  setProcessLogs((logs) => [...logs.slice(-8), 'Word：正在自动导出 DOCX'])
                  handleExportDocx(data.artifact).catch((err) => {
                    console.error('Auto DOCX export error:', err)
                    setProcessLogs((logs) => [...logs.slice(-8), 'Word：自动导出失败，请点击右侧按钮重试'])
                  })
                }
                if (
                  data.artifact.kind === 'markdown' &&
                  data.artifact.content?.export_requested &&
                  !autoExportedArtifactIdsRef.current.has(data.artifact.id)
                ) {
                  autoExportedArtifactIdsRef.current.add(data.artifact.id)
                  setProcessLogs((logs) => [...logs.slice(-8), 'Markdown：正在自动下载 MD'])
                  Promise.resolve(handleExportMarkdown(data.artifact)).catch((err) => {
                    console.error('Auto Markdown export error:', err)
                    setProcessLogs((logs) => [...logs.slice(-8), 'Markdown：自动下载失败，请点击右侧按钮重试'])
                  })
                }
              }
              break

            case 'state_update':
              setStreamPhase(data.phase === 'done' ? 'done' : 'generating')
              setStreamStatus(data.detail || data.step || '正在处理...')
              if (activeTabId) updateTab(activeTabId, { streamPhase: data.phase === 'done' ? ('done' as const) : ('generating' as const), streamStatus: data.detail || data.step || '正在处理...' })
              setProcessLogs((logs) => [...logs.slice(-8), `${data.step || '进度'}：${data.detail || ''}`])
              break

            case 'tool_result': {
              const toolName = data.tool || 'unknown'
              const detail = data.error || data.result?.error || data.result?.observation || ''
              if (data.success) {
                setProcessLogs((logs) => [...logs.slice(-8), `工具 ${toolName} ✓ 完成`])
              } else {
                const message = detail ? String(detail).slice(0, 300) : '未返回具体错误'
                setStreamStatus(`工具 ${toolName} 失败：${message}`)
                setProcessLogs((logs) => [...logs.slice(-8), `工具 ${toolName} ✗ 失败：${message}`])
              }
              break
            }

            case 'done': {
              playConversationDoneSound()
              const doneArtifacts = Array.isArray(data.new_artifacts) ? data.new_artifacts : []
              setStreamPhase('done')
              setStreamStatus(doneArtifacts.length > 0 ? '生成完成' : '回复完成')
              if (activeTabId) updateTab(activeTabId, { isStreaming: false, streamPhase: 'done', streamStatus: doneArtifacts.length > 0 ? '生成完成' : '回复完成' })
              if (data.session_id) setSessionId(data.session_id)
              // 确保对话结束后立即刷新列表，让当前对话出现在侧边栏
              refreshConversations()
              refreshProjects()
              if (Array.isArray(data.artifacts)) {
                data.artifacts.forEach((artifact: Artifact) => upsertArtifact(artifact))
                if (data.artifacts.length > 0) {
                  setShowArtifactPanel(true)
                }
              }
              const pptArtifact = doneArtifacts.find((item: Artifact) => item.kind === 'ppt')
              if (pptArtifact) {
                applyPptArtifact(pptArtifact)
                const total = pptArtifact.content?.total_slides || pptArtifact.content?.slide_count || pptArtifact.content?.slides?.length || 0
                if (total > 0) {
                  setPptProgress({ current: total, total })
                }
              } else if (data.project_id && !project) {
                pptApi.getProject(data.project_id).then(({ data: proj }) => {
                  setProject(proj)
                  setSlides(proj.slides || [])
                })
              }
              break
            }

            case 'error':
              console.error('SSE error:', data)
              setStreamPhase('error')
              setStreamStatus(data.message || data.detail || '生成失败')
              setPptProgress(null)
              if (activeTabId) updateTab(activeTabId, { isStreaming: false, streamPhase: 'error', streamStatus: data.message || data.detail || '生成失败' })
              usePPTStore.setState((state) => {
                const msgs = [...state.messages]
                msgs[msgs.length - 1] = {
                  ...msgs[msgs.length - 1],
                  content: `抱歉，生成失败：${data.message || data.detail || '请稍后重试'}`,
                }
                const tabId = state.activeTabId
                if (tabId && state.tabs[tabId]) {
                  return {
                    messages: msgs,
                    tabs: { ...state.tabs, [tabId]: { ...state.tabs[tabId], messages: msgs } },
                  }
                }
                return { messages: msgs }
              })
              break
          }
        },
        token,
        abortController.signal,
        toolConfig
      )
    } catch (err) {
      console.error('Chat error:', err)
      const aborted = err instanceof DOMException && err.name === 'AbortError'
      const errorMessage = err instanceof Error ? err.message : '发生了未知错误'
      if (!aborted && pendingAttachments.length > 0) {
        setAttachments(pendingAttachments)
      }
      setStreamPhase(aborted ? 'idle' : 'error')
      setStreamStatus(aborted ? '已停止生成' : errorMessage)
      setPptProgress(null)
      usePPTStore.setState((state) => {
        const msgs = [...state.messages]
        msgs[msgs.length - 1] = {
          ...msgs[msgs.length - 1],
          content: aborted
            ? '已停止本次生成。'
            : errorMessage === '未认证'
            ? '登录状态已失效，请重新登录后再试。'
            : `抱歉，发生了错误：${errorMessage}`,
        }
        const tabId = state.activeTabId
        if (tabId && state.tabs[tabId]) {
          return {
            messages: msgs,
            tabs: { ...state.tabs, [tabId]: { ...state.tabs[tabId], messages: msgs } },
          }
        }
        return { messages: msgs }
      })
    } finally {
      setStreaming(false)
      abortRef.current = null
      refreshConversations()
      refreshProjects()
      if (activeTabId) updateTab(activeTabId, { isStreaming: false })
    }
  }

  const handleStop = () => {
    abortRef.current?.abort()
    setStreaming(false)
    setStreamPhase('idle')
    setStreamStatus('已停止生成')
    setPptProgress(null)
    if (activeTabId) updateTab(activeTabId, { isStreaming: false, streamPhase: 'idle', streamStatus: '已停止生成' })
  }


  const handleExport = async (projectId = project?.id, projectTitle = project?.title) => {
    if (!projectId) return
    try {
      const res = await pptApi.exportPptx(projectId)
      const blob = res.data as Blob
      const url = window.URL.createObjectURL(blob)
      const link = document.createElement('a')
      const filename = safeFilename(projectTitle, 'presentation', '.pptx')
      await saveGeneratedBlob(blob, filename)
      link.href = url
      link.download = filename
      document.body.appendChild(link)
      link.click()
      link.remove()
      window.URL.revokeObjectURL(url)
    } catch (err) {
      console.error('Export error:', err)
      showToast('导出失败，请重试', 'error')
    }
  }

  const downloadBlob = (blob: Blob, filename: string) => {
    const url = window.URL.createObjectURL(blob)
    const link = document.createElement('a')
    link.href = url
    link.download = filename
    document.body.appendChild(link)
    link.click()
    link.remove()
    window.URL.revokeObjectURL(url)
  }

  const safeFilename = (title: string | undefined, fallback: string, extension: string) => {
    const base = (title || fallback).replace(/[\\/:*?"<>|]/g, '_').trim() || fallback
    return base.toLowerCase().endsWith(extension.toLowerCase()) ? base : `${base}${extension}`
  }

  const saveGeneratedBlob = async (blob: Blob, filename: string, artifact?: Artifact) => {
    try {
      await fileApi.saveBlob(
        blob,
        filename,
        artifact ? `智能助手生成：${artifact.title || filename}` : '智能助手生成'
      )
      if (artifact?.id) autoSavedArtifactIdsRef.current.add(artifact.id)
    } catch (err) {
      console.warn('Save generated file failed:', err)
    }
  }

  const blobFromUrl = async (url: string) => {
    if (url.startsWith('data:')) {
      const res = await fetch(url)
      return res.blob()
    }
    const res = await fetch(url, { mode: 'cors' })
    if (!res.ok) throw new Error(`下载资源失败：${res.status}`)
    return res.blob()
  }

  const handleExportExcel = async (artifact: Artifact) => {
    try {
      const res = await excelApi.exportXlsx(artifact)
      const blob = res.data as Blob
      const url = window.URL.createObjectURL(blob)
      const link = document.createElement('a')
      const filename = safeFilename(artifact.title, 'spreadsheet', '.xlsx')
      if (!autoSavedArtifactIdsRef.current.has(artifact.id)) await saveGeneratedBlob(blob, filename, artifact)
      link.href = url
      link.download = filename
      document.body.appendChild(link)
      link.click()
      link.remove()
      window.URL.revokeObjectURL(url)
    } catch (err) {
      console.error('Excel export error:', err)
      showToast('Excel 导出失败，请重试', 'error')
    }
  }

  const handleExportDocx = async (artifact: Artifact) => {
    try {
      const res = await docApi.exportDocx(artifact)
      const blob = res.data as Blob
      const url = window.URL.createObjectURL(blob)
      const link = document.createElement('a')
      const filename = safeFilename(artifact.title, 'document', '.docx')
      if (!autoSavedArtifactIdsRef.current.has(artifact.id)) await saveGeneratedBlob(blob, filename, artifact)
      link.href = url
      link.download = filename
      document.body.appendChild(link)
      link.click()
      link.remove()
      window.URL.revokeObjectURL(url)
    } catch (err) {
      console.error('DOCX export error:', err)
      showToast('Word 导出失败，请重试', 'error')
    }
  }

  const handleExportMarkdown = (artifact: Artifact) => {
    try {
      const markdown = artifact.content?.markdown || ''
      if (!markdown.trim()) {
        showToast('当前 Markdown 文档暂无可下载内容', 'info')
        return
      }
      const blob = new Blob([markdown], { type: 'text/markdown;charset=utf-8' })
      const filename = safeFilename(artifact.title, 'document', '.md')
      if (!autoSavedArtifactIdsRef.current.has(artifact.id)) void saveGeneratedBlob(blob, filename, artifact)
      downloadBlob(blob, filename)
    } catch (err) {
      console.error('Markdown export error:', err)
      showToast('Markdown 下载失败，请重试', 'error')
    }
  }

  const handleOpenArtifact = (artifactId: string) => {
    setActiveArtifact(artifactId)
    setShowArtifactPanel(true)
    setWideArtifactPanel(true)
  }

  const handleExportDrawio = async (artifact: Artifact) => {
    try {
      const xml = artifact.content?.xml
      if (!xml) {
        showToast('当前 draw.io 文件暂无可下载内容', 'info')
        return
      }
      const blob = new Blob([xml], { type: 'application/xml;charset=utf-8' })
      const filename = safeFilename(artifact.title, 'diagram', '.drawio')
      if (!autoSavedArtifactIdsRef.current.has(artifact.id)) await saveGeneratedBlob(blob, filename, artifact)
      downloadBlob(blob, filename)
    } catch (err) {
      console.error('Draw.io export error:', err)
      showToast('draw.io 下载失败，请重试', 'error')
    }
  }

  const handleExportArtifact = async (artifact: Artifact) => {
    if (artifact.kind === 'document') {
      await handleExportDocx(artifact)
      return
    }
    if (artifact.kind === 'markdown') {
      handleExportMarkdown(artifact)
      return
    }
    if (artifact.kind === 'sheet') {
      await handleExportExcel(artifact)
      return
    }
    if (artifact.kind === 'ppt') {
      if (artifact.content?.project_id && (!project || project.id !== artifact.content.project_id)) {
        try {
          const pptRes = await pptApi.getProject(artifact.content.project_id)
          setProject(pptRes.data)
          setSlides(pptRes.data.slides || [])
        } catch (err) {
          console.error('Load PPT project before export error:', err)
        }
      }
      await handleExport(artifact.content?.project_id || project?.id, artifact.title || project?.title)
      return
    }
    if (artifact.kind === 'drawio') {
      await handleExportDrawio(artifact)
      return
    }
    if (artifact.kind === 'image') {
      const imageUrl = artifact.content?.images?.[0]
      if (!imageUrl) {
        showToast('当前图片结果暂无可下载内容', 'info')
        return
      }
      const filename = safeFilename(artifact.title, 'image', '.png')
      if (!autoSavedArtifactIdsRef.current.has(artifact.id)) {
        blobFromUrl(imageUrl)
          .then((blob) => saveGeneratedBlob(blob, filename, artifact))
          .catch((err) => console.warn('Save image artifact failed:', err))
      }
      const link = document.createElement('a')
      link.href = imageUrl
      link.download = filename
      link.target = '_blank'
      document.body.appendChild(link)
      link.click()
      link.remove()
      return
    }
    if (artifact.kind === 'video') {
      const videoUrl = artifact.content?.video_url
      if (!videoUrl) {
        showToast('当前视频结果暂无可下载内容', 'info')
        return
      }
      const filename = safeFilename(artifact.title, 'video', '.mp4')
      if (!autoSavedArtifactIdsRef.current.has(artifact.id)) {
        blobFromUrl(videoUrl)
          .then((blob) => saveGeneratedBlob(blob, filename, artifact))
          .catch((err) => console.warn('Save video artifact failed:', err))
      }
      const link = document.createElement('a')
      link.href = videoUrl
      link.download = filename
      link.target = '_blank'
      document.body.appendChild(link)
      link.click()
      link.remove()
    }
  }

  const hasRenderableArtifact = slides.length > 0 || artifacts.length > 0 || !!activeArtifact

  // 构建传给 ChatPanel 的项目列表（用于下拉选择）
  const pptProjects = projects.map((p) => ({
    id: p.id,
    title: p.title,
    theme: 'default',
    slides: [],
    layout: '16x9' as const,
    created_at: p.created_at,
    updated_at: p.updated_at,
  }))

  const handleLogout = () => {
    logout()
    navigate('/login')
  }

  const handleSidebarResizeStart = (event: React.PointerEvent<HTMLDivElement>) => {
    event.preventDefault()
    const startX = event.clientX
    const startWidth = sidebarWidth
    document.body.style.cursor = 'col-resize'
    document.body.style.userSelect = 'none'

    let latestWidth = startWidth

    const handlePointerMove = (moveEvent: PointerEvent) => {
      latestWidth = Math.min(SIDEBAR_MAX_WIDTH, Math.max(SIDEBAR_MIN_WIDTH, startWidth + moveEvent.clientX - startX))
      setSidebarWidth(latestWidth)
    }

    const handlePointerUp = () => {
      document.body.style.cursor = ''
      document.body.style.userSelect = ''
      window.removeEventListener('pointermove', handlePointerMove)
      window.removeEventListener('pointerup', handlePointerUp)
      window.localStorage.setItem(SIDEBAR_WIDTH_STORAGE_KEY, String(latestWidth))
    }

    window.addEventListener('pointermove', handlePointerMove)
    window.addEventListener('pointerup', handlePointerUp, { once: true })
  }

  return (
    <div className="h-screen overflow-hidden bg-[#f6f4ef] text-surface-950">
      <div className="pointer-events-none fixed inset-0 bg-[radial-gradient(circle_at_15%_12%,rgba(255,255,255,0.92),transparent_32%),radial-gradient(circle_at_78%_8%,rgba(226,232,240,0.72),transparent_30%),linear-gradient(135deg,#f7f2e8_0%,#f3f1eb_45%,#ece7dc_100%)]" />

      <div className="relative z-10 flex h-full overflow-hidden">
        {/* 桌面端侧边栏（md 以上显示） */}
        <div className="hidden md:flex h-full">
          <ConversationSidebar
            project={project}
            messages={messages}
            conversations={conversations}
            projects={projects}
            activeProjectId={activeProjectId}
            userName={useAuthStore.getState().user?.username}
            activeTool={activeTool}
            activeConversationId={sessionId}
            isStreaming={isStreaming}
            streamPhase={streamPhase}
            localTabs={conversationTabs.map((t) => ({ id: t.id, title: t.title, tool: t.tool, messageCount: t.messageCount, isStreaming: t.isStreaming, streamPhase: t.streamPhase, sessionId: tabs[t.id]?.sessionId }))}
            activeTabId={activeTabId}
            onSelectTab={handleSelectTab}
            onToolChange={handleToolChange}
            onSelectConversation={handleSelectConversation}
            onSelectProject={handleSelectProject}
            onNewConversation={handleNewTab}
            onRenameConversation={handleRenameConversation}
            onDeleteConversation={handleDeleteConversation}
            onMoveConversation={handleMoveConversation}
            onDeleteProject={handleDeleteProject}
            onLogout={handleLogout}
            searchQuery={conversationQuery}
            onSearchQueryChange={setConversationQuery}
            width={sidebarWidth}
            onResizeStart={handleSidebarResizeStart}
          />
        </div>

        {/* 移动端侧边栏抽屉（md 以下，Overlay 模式） */}
        {mobileSidebarOpen && (
          <div className="fixed inset-0 z-50 md:hidden">
            <div
              className="absolute inset-0 bg-black/40 backdrop-blur-sm"
              onClick={() => setMobileSidebarOpen(false)}
            />
            <div className="absolute inset-y-0 left-0 w-[85vw] max-w-[360px] shadow-2xl">
              <ConversationSidebar
                project={project}
                messages={messages}
                conversations={conversations}
                projects={projects}
                activeProjectId={activeProjectId}
                userName={useAuthStore.getState().user?.username}
                activeTool={activeTool}
                activeConversationId={sessionId}
                isStreaming={isStreaming}
                streamPhase={streamPhase}
                localTabs={conversationTabs.map((t) => ({ id: t.id, title: t.title, tool: t.tool, messageCount: t.messageCount, isStreaming: t.isStreaming, streamPhase: t.streamPhase, sessionId: tabs[t.id]?.sessionId }))}
                activeTabId={activeTabId}
                onSelectTab={(id) => { handleSelectTab(id); setMobileSidebarOpen(false) }}
                onToolChange={handleToolChange}
                onSelectConversation={(id) => { handleSelectConversation(id); setMobileSidebarOpen(false) }}
                onSelectProject={(id) => { handleSelectProject(id); setMobileSidebarOpen(false) }}
                onNewConversation={() => { handleNewTab(); setMobileSidebarOpen(false) }}
                onRenameConversation={handleRenameConversation}
                onDeleteConversation={handleDeleteConversation}
                onMoveConversation={handleMoveConversation}
                onDeleteProject={handleDeleteProject}
                onLogout={handleLogout}
                searchQuery={conversationQuery}
                onSearchQueryChange={setConversationQuery}
                width={360}
                onMobileClose={() => setMobileSidebarOpen(false)}
              />
            </div>
          </div>
        )}

        <main className="relative flex min-w-0 flex-1 flex-col overflow-hidden">
          <header className="relative z-20 flex h-14 shrink-0 items-center justify-between border-b border-black/[0.05] bg-[#f6f4ef]/78 px-3 backdrop-blur-2xl md:px-5">
            <div className="flex min-w-0 items-center gap-2 md:gap-3">
              {/* 移动端汉堡菜单 */}
              <button
                type="button"
                onClick={() => setMobileSidebarOpen(true)}
                className="inline-flex h-9 w-9 shrink-0 items-center justify-center rounded-xl bg-white/60 text-surface-700 transition hover:bg-white/90 md:hidden"
                title="打开菜单"
                aria-label="打开菜单"
              >
                <Menu className="h-5 w-5" />
              </button>
              <div className="flex h-8 w-8 shrink-0 items-center justify-center overflow-hidden rounded-2xl bg-white/70 shadow-sm ring-1 ring-black/[0.04]">
                <img src={LOGO_URL} alt="WaLiOffice logo" className="h-full w-full object-cover" />
              </div>
              <div className="min-w-0">
                <div className="truncate text-sm font-semibold tracking-tight text-surface-950">
                  {settings?.basic?.workspace_title || '智能办公助手'}
                </div>
                <div className="hidden truncate text-[11px] text-surface-500 sm:block">
                  {settings?.basic?.brand_tagline || '分析 · 决策 · 绘制 · 流式反馈'}
                  {project ? ` · ${project.title}` : activeProjectId ? ` · ${projects.find(p => p.id === activeProjectId)?.title || ''}` : ''}
                </div>
              </div>
            </div>

            <div className="mx-2 min-w-0 flex-1 md:mx-4" />

            <div className="flex items-center gap-1.5 md:gap-2">
              <button
                onClick={() => navigate('/files')}
                className="btn-ghost shrink-0 rounded-full bg-white/45 hover:bg-white/75"
                title="我的文件"
              >
                <Files className="w-4 h-4" />
              </button>
              <button
                onClick={() => setShowArtifactPanel(!showArtifactPanel)}
                className="btn-ghost shrink-0 rounded-full bg-white/45 hover:bg-white/75 disabled:cursor-not-allowed disabled:opacity-40"
                title={hasRenderableArtifact ? '切换右侧成果展示' : '暂无成果可展示'}
                disabled={!hasRenderableArtifact}
              >
                {showArtifactPanel ? <PanelRightClose className="w-4 h-4" /> : <PanelRight className="w-4 h-4" />}
              </button>
              <button
                onClick={() => setShowPresent(true)}
                className="btn-secondary shrink-0 rounded-full bg-white/55"
                disabled={slides.length === 0}
              >
                <Play className="w-4 h-4" />
                <span className="hidden sm:inline">演示</span>
              </button>
            </div>
          </header>

          <div className="min-h-0 flex-1 overflow-hidden">
            {activeView === 'settings' ? (
              <SettingsDialog
                open
                settings={settings}
                onClose={() => setActiveView('chat')}
                onSave={handleSaveSettings}
              />
            ) : (
              <ChatPanel
                messages={messages}
                input={input}
                isStreaming={isStreaming}
                streamStatus={streamStatus}
                streamPhase={streamPhase}
                processLogs={processLogs}
                traceEvents={[]}
                selectedTheme={selectedTheme}
                activeTool={activeTool}
                projects={pptProjects}
                selectedProjectId={activeProjectId}
                modelProfiles={modelProfiles}
                selectedModel={selectedModel}
                artifacts={artifacts}
                activeArtifactId={activeArtifactId}
                toolConfig={toolConfig}
                onProjectChange={(pid) => pid ? handleSelectProject(pid) : setActiveProjectId(null)}
                onNewProject={handleNewProject}
                onModelChange={handleModelChange}
                onToolChange={handleToolChange}
                onThemeChange={setSelectedTheme}
                onInputChange={setInput}
                onSend={handleSend}
                onStop={handleStop}
                onToolConfigChange={setToolConfig}
                attachments={attachments}
                onPickAttachments={handlePickAttachments}
                onRemoveAttachment={handleRemoveAttachment}
                inputRefs={inputRefs}
                onRemoveInputRef={handleRemoveInputRef}
                historyArtifacts={historyArtifacts}
                onOpenArtifact={handleOpenArtifact}
                onExportArtifact={handleExportArtifact}
                onInsertArtifact={handleInsertArtifactToInput}
                messagesEndRef={messagesEndRef}
              />
            )}
          </div>
        </main>

        {/* 桌面端产物面板（md 以上内联） */}
        {hasRenderableArtifact && (
          <div className="hidden md:flex h-full">
            <ArtifactPanel
              activeTool={activeTool}
              project={project}
              slides={slides}
              currentSlideIndex={currentSlideIndex}
              isOpen={showArtifactPanel}
              isWide={wideArtifactPanel}
              onOpenChange={setShowArtifactPanel}
              onWideChange={setWideArtifactPanel}
              onSelectSlide={handleSelectSlide}
              onExportPpt={handleExport}
              onPresent={() => setShowPresent(true)}
              messages={messages}
              pptProgress={pptProgress}
              isGeneratingPpt={isStreaming && activeTool === 'ppt'}
              activeArtifact={activeArtifact}
              artifacts={artifacts}
              onSelectArtifact={setActiveArtifact}
              onUpdateArtifact={updateArtifact}
              onExportExcel={handleExportExcel}
              onExportDocx={handleExportDocx}
              onExportMarkdown={handleExportMarkdown}
              onExportDrawio={handleExportDrawio}
              onInsertArtifact={handleInsertArtifactToInput}
            />
          </div>
        )}

        {/* 移动端产物面板（md 以下全屏覆盖） */}
        {hasRenderableArtifact && showArtifactPanel && (
          <div className="fixed inset-0 z-50 md:hidden">
            <ArtifactPanel
              activeTool={activeTool}
              project={project}
              slides={slides}
              currentSlideIndex={currentSlideIndex}
              isOpen={true}
              isWide={true}
              onOpenChange={(open) => setShowArtifactPanel(open)}
              onWideChange={() => {}}
              onSelectSlide={handleSelectSlide}
              onExportPpt={handleExport}
              onPresent={() => setShowPresent(true)}
              messages={messages}
              pptProgress={pptProgress}
              isGeneratingPpt={isStreaming && activeTool === 'ppt'}
              activeArtifact={activeArtifact}
              artifacts={artifacts}
              onSelectArtifact={setActiveArtifact}
              onUpdateArtifact={updateArtifact}
              onExportExcel={handleExportExcel}
              onExportDocx={handleExportDocx}
              onExportMarkdown={handleExportMarkdown}
              onExportDrawio={handleExportDrawio}
              onInsertArtifact={handleInsertArtifactToInput}
              isMobile
            />
          </div>
        )}
      </div>

      {showPresent && slides.length > 0 && (
        <PresentMode
          slides={slides}
          startIndex={currentSlideIndex}
          onClose={() => setShowPresent(false)}
        />
      )}
      {toast && (
        <div className="pointer-events-none fixed right-5 top-5 z-[70] flex justify-end" aria-live="polite">
          <div className={`pointer-events-auto flex max-w-sm items-start gap-3 rounded-3xl border px-4 py-3 shadow-[0_18px_55px_rgba(24,24,27,0.16)] backdrop-blur-2xl ${
            toast.tone === 'error'
              ? 'border-red-200 bg-red-50/95 text-red-700'
              : toast.tone === 'success'
                ? 'border-emerald-200 bg-emerald-50/95 text-emerald-700'
                : 'border-black/[0.06] bg-white/92 text-surface-700'
          }`}
          >
            <div className={`mt-0.5 flex h-8 w-8 shrink-0 items-center justify-center rounded-2xl ${
              toast.tone === 'error'
                ? 'bg-red-100 text-red-600'
                : toast.tone === 'success'
                  ? 'bg-emerald-100 text-emerald-600'
                  : 'bg-surface-100 text-surface-600'
            }`}
            >
              {toast.tone === 'error' ? (
                <AlertCircle className="h-4 w-4" />
              ) : toast.tone === 'success' ? (
                <CheckCircle2 className="h-4 w-4" />
              ) : (
                <Info className="h-4 w-4" />
              )}
            </div>
            <div className="min-w-0 flex-1 pt-1 text-sm font-semibold leading-5">{toast.message}</div>
            <button
              type="button"
              onClick={() => setToast(null)}
              className="mt-0.5 rounded-full p-1 opacity-60 transition hover:bg-white/70 hover:opacity-100"
              aria-label="关闭提示"
            >
              <X className="h-3.5 w-3.5" />
            </button>
          </div>
        </div>
      )}
      <input
        ref={attachmentInputRef}
        type="file"
        accept=".md,.txt,.csv,.tsv,.json,.docx,.xlsx,.pptx,.pdf,text/markdown,text/plain,text/csv,application/json,application/pdf,application/vnd.openxmlformats-officedocument.wordprocessingml.document,application/vnd.openxmlformats-officedocument.spreadsheetml.sheet,application/vnd.openxmlformats-officedocument.presentationml.presentation,image/*"
        multiple
        className="hidden"
        onChange={handleAttachmentChange}
      />
    </div>
  )
}

function PresentMode({
  slides,
  startIndex,
  onClose,
}: {
  slides: any[]
  startIndex: number
  onClose: () => void
}) {
  const [index, setIndex] = useState(startIndex)

  useEffect(() => {
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
      if (e.key === 'ArrowRight' || e.key === ' ') setIndex((i) => Math.min(i + 1, slides.length - 1))
      if (e.key === 'ArrowLeft') setIndex((i) => Math.max(i - 1, 0))
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [slides.length, onClose])

  return (
    <div className="fixed inset-0 z-[60] bg-black flex flex-col items-center justify-center">
      <button
        onClick={onClose}
        className="absolute top-4 right-4 z-10 flex h-10 w-10 items-center justify-center rounded-full bg-white/10 text-white/80 transition hover:bg-white/20 hover:text-white"
        aria-label="关闭演示"
      >
        <X className="w-5 h-5" />
      </button>
      <div className="w-full max-w-5xl aspect-video md:aspect-video">
        <SlidePreview slide={slides[index]} layout="16x9" fullScreen />
      </div>
      {/* 移动端手势按钮 */}
      <div className="absolute bottom-6 left-0 right-0 flex items-center justify-center gap-4 md:hidden">
        <button
          onClick={() => setIndex((i) => Math.max(i - 1, 0))}
          disabled={index === 0}
          className="flex h-12 w-12 items-center justify-center rounded-full bg-white/10 text-white/80 disabled:opacity-30"
          aria-label="上一页"
        >
          ‹
        </button>
        <span className="min-w-[80px] text-center text-white/70 text-sm">{index + 1} / {slides.length}</span>
        <button
          onClick={() => setIndex((i) => Math.min(i + 1, slides.length - 1))}
          disabled={index === slides.length - 1}
          className="flex h-12 w-12 items-center justify-center rounded-full bg-white/10 text-white/80 disabled:opacity-30"
          aria-label="下一页"
        >
          ›
        </button>
      </div>
      <div className="absolute bottom-4 left-1/2 -translate-x-1/2 text-white/50 text-sm hidden md:block">
        {index + 1} / {slides.length}
      </div>
    </div>
  )
}
