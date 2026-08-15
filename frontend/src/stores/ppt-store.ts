import { create } from 'zustand';
import type { Artifact, PPTProject, Slide, ChatMessage } from '@/types';

interface PPTState {
  project: PPTProject | null;
  slides: Slide[];
  currentSlideIndex: number;
  messages: ChatMessage[];
  isGenerating: boolean;
  isStreaming: boolean;
  sessionId: string | null;
  artifacts: Artifact[];
  activeArtifactId: string | null;

  setProject: (project: PPTProject) => void;
  setSlides: (slides: Slide[]) => void;
  updateSlide: (slideId: string, updates: Partial<Slide>) => void;
  addSlide: (slide: Slide, index?: number) => void;
  deleteSlide: (slideId: string) => void;
  setCurrentSlide: (index: number) => void;
  addMessage: (message: ChatMessage) => void;
  setGenerating: (v: boolean) => void;
  setStreaming: (v: boolean) => void;
  setSessionId: (id: string) => void;
  upsertArtifact: (artifact: Artifact) => void;
  updateArtifact: (artifactId: string, updates: Partial<Artifact>) => void;
  setActiveArtifact: (artifactId: string | null) => void;
  clearArtifacts: () => void;
  reset: () => void;
}

export const usePPTStore = create<PPTState>((set) => ({
  project: null,
  slides: [],
  currentSlideIndex: 0,
  messages: [],
  isGenerating: false,
  isStreaming: false,
  sessionId: null,
  artifacts: [],
  activeArtifactId: null,

  setProject: (project) =>
    set({ project, slides: project.slides || [] }),

  setSlides: (slides) => set({ slides }),

  updateSlide: (slideId, updates) =>
    set((state) => ({
      slides: state.slides.map((s) =>
        s.id === slideId ? { ...s, ...updates } : s
      ),
    })),

  addSlide: (slide, index) =>
    set((state) => {
      const slides = [...state.slides];
      if (index !== undefined && index >= 0 && index <= slides.length) {
        slides.splice(index, 0, slide);
      } else {
        slides.push(slide);
      }
      return { slides };
    }),

  deleteSlide: (slideId) =>
    set((state) => ({
      slides: state.slides.filter((s) => s.id !== slideId),
      currentSlideIndex: Math.min(state.currentSlideIndex, state.slides.length - 2),
    })),

  setCurrentSlide: (index) => set({ currentSlideIndex: index }),

  addMessage: (message) =>
    set((state) => ({ messages: [...state.messages, message] })),

  setGenerating: (v) => set({ isGenerating: v }),
  setStreaming: (v) => set({ isStreaming: v }),
  setSessionId: (id) => set({ sessionId: id }),

  upsertArtifact: (artifact) =>
    set((state) => {
      const exists = state.artifacts.some((item) => item.id === artifact.id);
      return {
        artifacts: exists
          ? state.artifacts.map((item) => item.id === artifact.id ? artifact : item)
          : [artifact, ...state.artifacts],
        activeArtifactId: artifact.id,
      };
    }),

  updateArtifact: (artifactId, updates) =>
    set((state) => ({
      artifacts: state.artifacts.map((item) =>
        item.id === artifactId
          ? { ...item, ...updates, updated_at: new Date().toISOString(), version: (updates.version ?? item.version + 1) }
          : item
      ),
    })),

  setActiveArtifact: (artifactId) => set({ activeArtifactId: artifactId }),
  clearArtifacts: () => set({ artifacts: [], activeArtifactId: null }),

  reset: () =>
    set({
      project: null,
      slides: [],
      currentSlideIndex: 0,
      messages: [],
      isGenerating: false,
      isStreaming: false,
      sessionId: null,
      artifacts: [],
      activeArtifactId: null,
    }),
}));
