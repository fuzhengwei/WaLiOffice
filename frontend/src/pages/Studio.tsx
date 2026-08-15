import { useState, useRef, useEffect } from 'react'
import { useAuthStore } from '@/stores/auth-store'
import { usePPTStore } from '@/stores/ppt-store'
import { chatApi, docApi, excelApi, pptApi, sessionApi, projectApi, settingsApi } from '@/api'
import { ChatPanel } from '@/components/chat/ChatPanel'
import { SlidePreview } from '@/components/preview/SlidePreview'
import { ConversationSidebar } from '@/components/history/ConversationSidebar'
import { ArtifactPanel } from '@/components/artifacts/ArtifactPanel'
import { SettingsDialog } from '@/components/settings/SettingsDialog'
import { Sparkles, Play, X, PanelRightClose, PanelRight } from 'lucide-react'
import type { Artifact, ConversationRecord, ToolKind, ProjectMeta, AppSettings, LLMProfile, PersistedSession, ChatAttachment } from '@/types'

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
  const {
    project, slides, currentSlideIndex, messages,
    isStreaming, sessionId, artifacts, activeArtifactId,
    setProject, setSlides, setCurrentSlide,
    addMessage, setStreaming, setSessionId, reset,
    upsertArtifact, updateArtifact, setActiveArtifact,
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

  // 设置 & 模型
  const [settings, setSettings] = useState<AppSettings | null>(null)
  const [selectedModel, setSelectedModel] = useState<string>('gpt-5.5')
  const [modelProfiles, setModelProfiles] = useState<LLMProfile[]>([])

  const activeArtifact = artifacts.find((artifact) => artifact.id === activeArtifactId) || null
  const messagesEndRef = useRef<HTMLDivElement>(null)
  const abortRef = useRef<AbortController | null>(null)
  const autoExportedArtifactIdsRef = useRef<Set<string>>(new Set())
  const attachmentInputRef = useRef<HTMLInputElement>(null)

  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' })
  }, [messages])

  // 加载设置
  const loadSettings = async () => {
    try {
      const res = await settingsApi.getSettings()
      const s = res.data as AppSettings
      setSettings(s)
      setModelProfiles(s.llm_profiles || [])
      if (s.active_model) setSelectedModel(s.active_model)
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
    try {
      const res = await sessionApi.listSessions({ q: query || undefined, page: 1, page_size: 50 })
      const rows = (res.data.sessions || []).map((item: any) => ({
        id: item.id,
        title: item.title || '未命名会话',
        tool: item.tool_kind || 'general',
        summary: item.summary,
        updated_at: item.updated_at,
        message_count: item.message_count || 0,
        project_id: item.project_id,
      }))
      setConversations(rows)
    } catch (err) {
      console.error('Load conversations error:', err)
    }
  }

  useEffect(() => {
    loadSettings()
    refreshProjects()
    refreshConversations('')
  }, [])

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
      refreshProjects()
    } catch (err) {
      console.error('Create project error:', err)
      alert('创建项目失败')
    }
  }

  const handleSelectProject = async (projectId: string) => {
    if (isStreaming) return
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
      alert('删除项目失败')
    }
  }

  const handleNewConversation = () => {
    if (isStreaming) return
    setActiveView('chat')
    refreshConversations()
    reset()
    autoExportedArtifactIdsRef.current.clear()
    setActiveTool('general')
    setShowArtifactPanel(false)
    setFollowLatestSlide(true)
    setPptProgress(null)
  }

  const handleSelectConversation = async (id: string) => {
    if (isStreaming) return
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
    } catch (err) {
      console.error('Select conversation error:', err)
      const errMsg = err instanceof Error ? err.message : '未知错误'
      alert(`恢复历史会话失败：${errMsg}`)
    }
  }

  const handleClearConversation = async (id: string) => {
    if (!confirm('确认清空该对话的消息内容？')) return
    try {
      await sessionApi.clearSession(id)
      refreshConversations()
      if (sessionId === id) {
        reset()
      }
    } catch (err) {
      console.error('Clear conversation error:', err)
      alert('清空对话失败')
    }
  }

  const handleDeleteConversation = async (id: string) => {
    if (!confirm('确认删除该对话？此操作不可恢复。')) return
    try {
      await sessionApi.deleteSession(id)
      if (sessionId === id) {
        reset()
      }
      refreshConversations()
      refreshProjects()
    } catch (err) {
      console.error('Delete conversation error:', err)
      alert('删除对话失败')
    }
  }

  const handleSaveSettings = async (newSettings: AppSettings) => {
    try {
      const res = await settingsApi.saveSettings(newSettings)
      const saved = res.data as AppSettings
      setSettings(saved)
      setModelProfiles(saved.llm_profiles || [])
      if (saved.active_model) setSelectedModel(saved.active_model)
      if (saved.basic?.default_theme) setSelectedTheme(saved.basic.default_theme)
      setActiveView('chat')
    } catch (err) {
      console.error('Save settings error:', err)
      alert('设置保存失败')
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
        alert('目前支持上传 md、txt 和图片文件。')
      }
      if (nextItems.length === 0) return

      setAttachments((current) => {
        const merged = [...current, ...nextItems]
        const deduped = merged.filter((item, index, arr) => arr.findIndex((target) => target.name === item.name && target.size === item.size) === index)
        return deduped.slice(0, 6)
      })
    } catch (err) {
      console.error('Read attachment error:', err)
      alert('读取附件失败，请检查文件编码或重新选择文件。')
    }
  }

  const handleRemoveAttachment = (id: string) => {
    setAttachments((current) => current.filter((item) => item.id !== id))
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
    const hasImageGenerationIntent = /生成.*图|做.*图|画.*图|出图|海报|封面|logo|配图|主视觉|插画|banner|视觉稿|图像创作/.test(lower)

    if (/draw\.io|drawio|流程图|架构图|泳道图|拓扑图|er图/.test(lower)) hits.push('drawio')
    if (/excel|xlsx|表格|数据分析|公式|在线表/.test(lower)) hits.push('excel')
    if (/文档|报告|prd|方案|纪要|文章|docx|markdown|readme|知识库|说明文档|操作手册|md\b/.test(lower)) hits.push('doc')
    if (/ppt|演示文稿|幻灯片|presentation|做个.*汇报|生成.*汇报|制作.*汇报|汇报材料/.test(lower)) hits.push('ppt')
    if (hasImageGenerationIntent || (/图片|图像/.test(lower) && !hasImageRecognitionIntent && !hasImageAttachment)) hits.push('image')
    const wantsMultiple = /同时|一起|并且|再来|外加|附上|配一张|再补一个|多个|一套/.test(lower)
    const uniqueHits = Array.from(new Set(hits))
    if (hasImageAttachment && !hasImageGenerationIntent) return 'general'
    if (uniqueHits.length > 1 || (wantsMultiple && uniqueHits.length > 0)) return 'general'
    if (uniqueHits.length === 1) return uniqueHits[0]
    return activeTool
  }

  const handleSend = async () => {
    if ((!input.trim() && attachments.length === 0) || isStreaming) return

    const hasImageAttachment = attachments.some((item) => item.kind === 'image')
    const message = input.trim() || (hasImageAttachment ? '请识别并说明我上传图片的主要内容。' : '请结合我上传的文件内容继续处理。')
    const pendingAttachments = attachments
    const inferredTool = inferToolFromMessage(message, pendingAttachments)
    if (inferredTool !== activeTool) setActiveTool(inferredTool)
    setInput('')
    setAttachments([])
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
    const abortController = new AbortController()
    abortRef.current = abortController

    addMessage({
      role: 'user',
      content: message,
      timestamp: new Date().toISOString(),
      attachments: pendingAttachments,
    })

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
                  return { messages: msgs }
                })
              }
              if (data.session_id) {
                setSessionId(data.session_id)
                if (data.start) {
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
              setProcessLogs((logs) => [...logs.slice(-8), `${data.step || '进度'}：${data.detail || ''}`])
              break

            case 'done':
              setStreamPhase('done')
              setStreamStatus(Array.isArray(data.artifacts) && data.artifacts.length > 0 ? '生成完成' : '回复完成')
              if (data.session_id) setSessionId(data.session_id)
              if (Array.isArray(data.artifacts)) {
                data.artifacts.forEach((artifact: Artifact) => upsertArtifact(artifact))
                if (data.artifacts.length > 0) {
                  setShowArtifactPanel(true)
                }
              }
              const pptArtifact = (data.artifacts || []).find((item: Artifact) => item.kind === 'ppt')
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

            case 'error':
              console.error('SSE error:', data)
              setStreamPhase('error')
              setStreamStatus(data.message || data.detail || '生成失败')
              setPptProgress(null)
              usePPTStore.setState((state) => {
                const msgs = [...state.messages]
                msgs[msgs.length - 1] = {
                  ...msgs[msgs.length - 1],
                  content: `抱歉，生成失败：${data.message || data.detail || '请稍后重试'}`,
                }
                return { messages: msgs }
              })
              break
          }
        },
        token,
        abortController.signal
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
        return { messages: msgs }
      })
    } finally {
      setStreaming(false)
      abortRef.current = null
      refreshConversations()
      refreshProjects()
    }
  }

  const handleStop = () => {
    abortRef.current?.abort()
    setStreaming(false)
    setStreamPhase('idle')
    setStreamStatus('已停止生成')
    setPptProgress(null)
  }


  const handleExport = async (projectId = project?.id, projectTitle = project?.title) => {
    if (!projectId) return
    try {
      const res = await pptApi.exportPptx(projectId)
      const blob = res.data as Blob
      const url = window.URL.createObjectURL(blob)
      const link = document.createElement('a')
      const safeTitle = (projectTitle || 'presentation').replace(/[\\/:*?"<>|]/g, '_')
      link.href = url
      link.download = `${safeTitle}.pptx`
      document.body.appendChild(link)
      link.click()
      link.remove()
      window.URL.revokeObjectURL(url)
    } catch (err) {
      console.error('Export error:', err)
      alert('导出失败，请重试')
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

  const handleExportExcel = async (artifact: Artifact) => {
    try {
      const res = await excelApi.exportXlsx(artifact)
      const blob = res.data as Blob
      const url = window.URL.createObjectURL(blob)
      const link = document.createElement('a')
      const safeTitle = (artifact.title || 'spreadsheet').replace(/[\\/:*?"<>|]/g, '_')
      link.href = url
      link.download = `${safeTitle}.xlsx`
      document.body.appendChild(link)
      link.click()
      link.remove()
      window.URL.revokeObjectURL(url)
    } catch (err) {
      console.error('Excel export error:', err)
      alert('Excel 导出失败，请重试')
    }
  }

  const handleExportDocx = async (artifact: Artifact) => {
    try {
      const res = await docApi.exportDocx(artifact)
      const blob = res.data as Blob
      const url = window.URL.createObjectURL(blob)
      const link = document.createElement('a')
      const safeTitle = (artifact.title || 'document').replace(/[\\/:*?"<>|]/g, '_')
      link.href = url
      link.download = `${safeTitle}.docx`
      document.body.appendChild(link)
      link.click()
      link.remove()
      window.URL.revokeObjectURL(url)
    } catch (err) {
      console.error('DOCX export error:', err)
      alert('Word 导出失败，请重试')
    }
  }

  const handleExportMarkdown = (artifact: Artifact) => {
    try {
      const markdown = artifact.content?.markdown || ''
      if (!markdown.trim()) {
        alert('当前 Markdown 文档暂无可下载内容')
        return
      }
      const safeTitle = (artifact.title || 'document').replace(/[\\/:*?"<>|]/g, '_')
      const blob = new Blob([markdown], { type: 'text/markdown;charset=utf-8' })
      downloadBlob(blob, `${safeTitle}.md`)
    } catch (err) {
      console.error('Markdown export error:', err)
      alert('Markdown 下载失败，请重试')
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
        alert('当前 draw.io 文件暂无可下载内容')
        return
      }
      const safeTitle = (artifact.title || 'diagram').replace(/[\\/:*?"<>|]/g, '_')
      const blob = new Blob([xml], { type: 'application/xml;charset=utf-8' })
      downloadBlob(blob, `${safeTitle}.drawio`)
    } catch (err) {
      console.error('Draw.io export error:', err)
      alert('draw.io 下载失败，请重试')
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

  return (
    <div className="h-screen overflow-hidden bg-[#f6f4ef] text-surface-950">
      <div className="pointer-events-none fixed inset-0 bg-[radial-gradient(circle_at_15%_12%,rgba(255,255,255,0.92),transparent_32%),radial-gradient(circle_at_78%_8%,rgba(226,232,240,0.72),transparent_30%),linear-gradient(135deg,#f7f2e8_0%,#f3f1eb_45%,#ece7dc_100%)]" />

      <div className="relative z-10 flex h-full overflow-hidden">
        <ConversationSidebar
          project={project}
          messages={messages}
          conversations={conversations}
          projects={projects}
          activeProjectId={activeProjectId}
          userName={useAuthStore.getState().user?.username}
          activeTool={activeTool}
          activeConversationId={sessionId}
          activeView={activeView}
          onToolChange={handleToolChange}
          onNewProject={handleNewProject}
          onNewConversation={handleNewConversation}
          onSelectConversation={handleSelectConversation}
          onSelectProject={handleSelectProject}
          onClearConversation={handleClearConversation}
          onDeleteConversation={handleDeleteConversation}
          onDeleteProject={handleDeleteProject}
          onOpenSettings={() => setActiveView('settings')}
          searchQuery={conversationQuery}
          onSearchQueryChange={setConversationQuery}
        />

        <main className="relative flex min-w-0 flex-1 flex-col overflow-hidden">
          <header className="relative z-20 flex h-14 shrink-0 items-center justify-between border-b border-black/[0.05] bg-[#f6f4ef]/78 px-5 backdrop-blur-2xl">
            <div className="flex min-w-0 items-center gap-3">
              <div className="flex h-8 w-8 items-center justify-center rounded-2xl bg-white/70 shadow-sm ring-1 ring-black/[0.04]">
                <Sparkles className="h-4 w-4 text-surface-900" />
              </div>
              <div className="min-w-0">
                <div className="truncate text-sm font-semibold tracking-tight text-surface-950">
                  {settings?.basic?.workspace_title || '智能办公助手'}
                </div>
                <div className="truncate text-[11px] text-surface-500">
                  {settings?.basic?.brand_tagline || '分析 · 决策 · 绘制 · 流式反馈'}
                  {project ? ` · ${project.title}` : activeProjectId ? ` · ${projects.find(p => p.id === activeProjectId)?.title || ''}` : ''}
                </div>
              </div>
            </div>

            <div className="mx-4 min-w-0 flex-1" />

            <div className="flex items-center gap-2">
              <button
                onClick={() => setShowArtifactPanel(!showArtifactPanel)}
                className="btn-ghost rounded-full bg-white/45 hover:bg-white/75 disabled:cursor-not-allowed disabled:opacity-40"
                title={hasRenderableArtifact ? '切换右侧成果展示' : '暂无成果可展示'}
                disabled={!hasRenderableArtifact}
              >
                {showArtifactPanel ? <PanelRightClose className="w-4 h-4" /> : <PanelRight className="w-4 h-4" />}
              </button>
              <button
                onClick={() => setShowPresent(true)}
                className="btn-secondary rounded-full bg-white/55"
                disabled={slides.length === 0}
              >
                <Play className="w-4 h-4" />
                演示
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
                onProjectChange={(pid) => pid ? handleSelectProject(pid) : setActiveProjectId(null)}
                onModelChange={handleModelChange}
                onToolChange={handleToolChange}
                onThemeChange={setSelectedTheme}
                onInputChange={setInput}
                onSend={handleSend}
                onStop={handleStop}
                attachments={attachments}
                onPickAttachments={handlePickAttachments}
                onRemoveAttachment={handleRemoveAttachment}
                onOpenArtifact={handleOpenArtifact}
                onExportArtifact={handleExportArtifact}
                messagesEndRef={messagesEndRef}
              />
            )}
          </div>
        </main>

        {hasRenderableArtifact && (
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
          />
        )}
      </div>

      {showPresent && slides.length > 0 && (
        <PresentMode
          slides={slides}
          startIndex={currentSlideIndex}
          onClose={() => setShowPresent(false)}
        />
      )}
      <input
        ref={attachmentInputRef}
        type="file"
        accept=".md,.txt,text/markdown,text/plain,image/*"
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
    <div className="fixed inset-0 bg-black z-50 flex items-center justify-center">
      <button
        onClick={onClose}
        className="absolute top-4 right-4 text-white/60 hover:text-white z-10"
      >
        <X className="w-6 h-6" />
      </button>
      <div className="w-full max-w-5xl aspect-video">
        <SlidePreview slide={slides[index]} layout="16x9" fullScreen />
      </div>
      <div className="absolute bottom-4 left-1/2 -translate-x-1/2 text-white/50 text-sm">
        {index + 1} / {slides.length}
      </div>
    </div>
  )
}
