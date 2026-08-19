import { create } from 'zustand';
import type { Artifact, PPTProject, Slide, ChatMessage, ToolKind } from '@/types';

interface ConversationState {
  project: PPTProject | null;
  slides: Slide[];
  currentSlideIndex: number;
  messages: ChatMessage[];
  isGenerating: boolean;
  isStreaming: boolean;
  sessionId: string | null;
  artifacts: Artifact[];
  activeArtifactId: string | null;
  activeTool: ToolKind;
  input: string;
  streamStatus: string;
  streamPhase: 'idle' | 'thinking' | 'generating' | 'finishing' | 'done' | 'error';
  processLogs: string[];
  attachments: any[];
  selectedTheme: string;
  toolConfig: Record<string, any>;
  selectedModel: string;
  activeProjectId: string | null;
  tabTitle: string;
}

export interface TabState extends ConversationState {
  tabId: string;
  createdAt: number;
}

// tabTitle 在 TabState 中，不需要重复声明

interface PPTState {
  // 当前活跃 tab
  activeTabId: string | null;
  // 所有 tab 状态（tabId -> ConversationState）
  tabs: Record<string, TabState>;

  // 便捷访问（从活跃 tab 读取）
  project: PPTProject | null;
  slides: Slide[];
  currentSlideIndex: number;
  messages: ChatMessage[];
  isGenerating: boolean;
  isStreaming: boolean;
  sessionId: string | null;
  artifacts: Artifact[];
  activeArtifactId: string | null;

  // Tab 管理
  openTab: (tabId: string, initialState?: Partial<ConversationState>) => void;
  closeTab: (tabId: string) => void;
  switchTab: (tabId: string) => void;
  updateTab: (tabId: string, updates: Partial<ConversationState>) => void;
  getTabState: (tabId: string) => TabState | null;

  // 状态操作（操作活跃 tab）
  setProject: (project: PPTProject) => void;
  setSlides: (slides: Slide[]) => void;
  updateSlide: (slideId: string, updates: Partial<Slide>) => void;
  addSlide: (slide: Slide, index?: number) => void;
  deleteSlide: (slideId: string) => void;
  setCurrentSlide: (index: number) => void;
  addMessage: (message: ChatMessage) => void;
  setMessages: (messages: ChatMessage[]) => void;
  setGenerating: (v: boolean) => void;
  setStreaming: (v: boolean) => void;
  setSessionId: (id: string) => void;
  upsertArtifact: (artifact: Artifact) => void;
  updateArtifact: (artifactId: string, updates: Partial<Artifact>) => void;
  setActiveArtifact: (artifactId: string | null) => void;
  clearArtifacts: () => void;
  reset: () => void;

  // 批量设置（切换 tab 时恢复状态）
  restoreState: (state: Partial<ConversationState>) => void;
}

function createInitialTabState(tabId: string, overrides?: Partial<ConversationState>): TabState {
  return {
    tabId,
    tabTitle: '新对话',
    createdAt: Date.now(),
    project: null,
    slides: [],
    currentSlideIndex: 0,
    messages: [],
    isGenerating: false,
    isStreaming: false,
    sessionId: null,
    artifacts: [],
    activeArtifactId: null,
    activeTool: 'general',
    input: '',
    streamStatus: '空闲',
    streamPhase: 'idle',
    processLogs: [],
    attachments: [],
    selectedTheme: 'default',
    toolConfig: {},
    selectedModel: '',
    activeProjectId: null,
    ...overrides,
  };
}

function syncFromTab(state: PPTState, tabId: string): Partial<PPTState> {
  const tab = state.tabs[tabId];
  if (!tab) return {};
  return {
    project: tab.project,
    slides: tab.slides,
    currentSlideIndex: tab.currentSlideIndex,
    messages: tab.messages,
    isGenerating: tab.isGenerating,
    isStreaming: tab.isStreaming,
    sessionId: tab.sessionId,
    artifacts: tab.artifacts,
    activeArtifactId: tab.activeArtifactId,
  };
}

export const usePPTStore = create<PPTState>((set, get) => ({
  activeTabId: null,
  tabs: {},

  project: null,
  slides: [],
  currentSlideIndex: 0,
  messages: [],
  isGenerating: false,
  isStreaming: false,
  sessionId: null,
  artifacts: [],
  activeArtifactId: null,

  openTab: (tabId, initialState) => {
    set((state) => {
      if (state.tabs[tabId]) {
        // tab 已存在，切换过去
        return { activeTabId: tabId, ...syncFromTab(state, tabId) };
      }
      const tab = createInitialTabState(tabId, initialState);
      return {
        activeTabId: tabId,
        tabs: { ...state.tabs, [tabId]: tab },
        ...syncFromTab({ ...state, tabs: { ...state.tabs, [tabId]: tab } }, tabId),
      };
    });
  },

  closeTab: (tabId) => {
    set((state) => {
      const newTabs = { ...state.tabs };
      delete newTabs[tabId];
      const remainingIds = Object.keys(newTabs);
      if (remainingIds.length === 0) {
        return {
          activeTabId: null,
          tabs: {},
          project: null,
          slides: [],
          currentSlideIndex: 0,
          messages: [],
          isGenerating: false,
          isStreaming: false,
          sessionId: null,
          artifacts: [],
          activeArtifactId: null,
        };
      }
      // 切换到最后一个 tab
      const nextActive = remainingIds[remainingIds.length - 1];
      return { activeTabId: nextActive, tabs: newTabs, ...syncFromTab({ ...state, tabs: newTabs }, nextActive) };
    });
  },

  switchTab: (tabId) => {
    set((state) => {
      if (!state.tabs[tabId]) return state;
      return { activeTabId: tabId, ...syncFromTab(state, tabId) };
    });
  },

  updateTab: (tabId, updates) => {
    set((state) => {
      const tab = state.tabs[tabId];
      if (!tab) return state;
      const newTab = { ...tab, ...updates };
      const newTabs = { ...state.tabs, [tabId]: newTab };
      const result: any = { tabs: newTabs };
      // 如果更新的是活跃 tab，同步到顶层
      if (state.activeTabId === tabId) {
        Object.assign(result, syncFromTab({ ...state, tabs: newTabs }, tabId));
      }
      return result;
    });
  },

  getTabState: (tabId) => {
    return get().tabs[tabId] || null;
  },

  setProject: (project) => {
    set((state) => {
      if (!state.activeTabId) return { project, slides: project.slides || [] };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { project, slides: project.slides || [] };
      const newTabs = { ...state.tabs, [tabId]: { ...tab, project, slides: project.slides || [] } };
      return { tabs: newTabs, project, slides: project.slides || [] };
    });
  },

  setSlides: (slides) => {
    set((state) => {
      if (!state.activeTabId) return { slides };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { slides };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, slides } }, slides };
    });
  },

  updateSlide: (slideId, updates) => {
    set((state) => {
      const slides = state.slides.map((s) => s.id === slideId ? { ...s, ...updates } : s);
      if (!state.activeTabId) return { slides };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { slides };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, slides } }, slides };
    });
  },

  addSlide: (slide, index) => {
    set((state) => {
      const slides = [...state.slides];
      if (index !== undefined && index >= 0 && index <= slides.length) {
        slides.splice(index, 0, slide);
      } else {
        slides.push(slide);
      }
      if (!state.activeTabId) return { slides };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { slides };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, slides } }, slides };
    });
  },

  deleteSlide: (slideId) => {
    set((state) => {
      const slides = state.slides.filter((s) => s.id !== slideId);
      const currentSlideIndex = Math.min(state.currentSlideIndex, slides.length - 1);
      if (!state.activeTabId) return { slides, currentSlideIndex };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { slides, currentSlideIndex };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, slides, currentSlideIndex } }, slides, currentSlideIndex };
    });
  },

  setCurrentSlide: (index) => {
    set((state) => {
      if (!state.activeTabId) return { currentSlideIndex: index };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { currentSlideIndex: index };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, currentSlideIndex: index } }, currentSlideIndex: index };
    });
  },

  addMessage: (message) => {
    set((state) => {
      const messages = [...state.messages, message];
      if (!state.activeTabId) return { messages };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { messages };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, messages } }, messages };
    });
  },

  setMessages: (messages) => {
    set((state) => {
      if (!state.activeTabId) return { messages };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { messages };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, messages } }, messages };
    });
  },

  setGenerating: (v) => {
    set((state) => {
      if (!state.activeTabId) return { isGenerating: v };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { isGenerating: v };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, isGenerating: v } }, isGenerating: v };
    });
  },

  setStreaming: (v) => {
    set((state) => {
      if (!state.activeTabId) return { isStreaming: v };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { isStreaming: v };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, isStreaming: v } }, isStreaming: v };
    });
  },

  setSessionId: (id) => {
    set((state) => {
      if (!state.activeTabId) return { sessionId: id };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { sessionId: id };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, sessionId: id } }, sessionId: id };
    });
  },

  upsertArtifact: (artifact) => {
    set((state) => {
      const exists = state.artifacts.some((item) => item.id === artifact.id);
      const artifacts = exists
        ? state.artifacts.map((item) => item.id === artifact.id ? artifact : item)
        : [artifact, ...state.artifacts];
      const activeArtifactId = artifact.id;
      if (!state.activeTabId) return { artifacts, activeArtifactId };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { artifacts, activeArtifactId };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, artifacts, activeArtifactId } }, artifacts, activeArtifactId };
    });
  },

  updateArtifact: (artifactId, updates) => {
    set((state) => {
      const artifacts = state.artifacts.map((item) =>
        item.id === artifactId
          ? { ...item, ...updates, updated_at: new Date().toISOString(), version: (updates.version ?? item.version + 1) }
          : item
      );
      if (!state.activeTabId) return { artifacts };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { artifacts };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, artifacts } }, artifacts };
    });
  },

  setActiveArtifact: (artifactId) => {
    set((state) => {
      if (!state.activeTabId) return { activeArtifactId: artifactId };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { activeArtifactId: artifactId };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, activeArtifactId: artifactId } }, activeArtifactId: artifactId };
    });
  },

  clearArtifacts: () => {
    set((state) => {
      if (!state.activeTabId) return { artifacts: [], activeArtifactId: null };
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return { artifacts: [], activeArtifactId: null };
      return { tabs: { ...state.tabs, [tabId]: { ...tab, artifacts: [], activeArtifactId: null } }, artifacts: [], activeArtifactId: null };
    });
  },

  restoreState: (partial) => {
    set((state) => {
      if (!state.activeTabId) return partial;
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) return partial;
      return { tabs: { ...state.tabs, [tabId]: { ...tab, ...partial } }, ...partial };
    });
  },

  reset: () => {
    set((state) => {
      if (!state.activeTabId) {
        return {
          project: null, slides: [], currentSlideIndex: 0, messages: [],
          isGenerating: false, isStreaming: false, sessionId: null,
          artifacts: [], activeArtifactId: null,
        };
      }
      const tabId = state.activeTabId;
      const tab = state.tabs[tabId];
      if (!tab) {
        return {
          project: null, slides: [], currentSlideIndex: 0, messages: [],
          isGenerating: false, isStreaming: false, sessionId: null,
          artifacts: [], activeArtifactId: null,
        };
      }
      const resetTab = { ...tab, project: null, slides: [], currentSlideIndex: 0, messages: [], isGenerating: false, isStreaming: false, sessionId: null, artifacts: [], activeArtifactId: null };
      return {
        tabs: { ...state.tabs, [tabId]: resetTab },
        project: null, slides: [], currentSlideIndex: 0, messages: [],
        isGenerating: false, isStreaming: false, sessionId: null,
        artifacts: [], activeArtifactId: null,
      };
    });
  },
}));
