import { Check, KeyRound, Plus, Server, ShieldCheck, Trash2, X } from 'lucide-react'
import { useEffect, useMemo, useState } from 'react'
import { settingsApi } from '@/api'
import type { AppSettings, LLMProfile, MCPServiceConfig } from '@/types'

interface SettingsDialogProps {
  open: boolean
  settings: AppSettings | null
  onClose: () => void
  onSave: (settings: AppSettings) => Promise<void> | void
}

const emptyProfile = (): LLMProfile => ({
  id: `profile-${Date.now()}`,
  name: '新的模型服务',
  base_url: 'http://127.0.0.1:8777/v1',
  api_key: '',
  models: ['glm_for_coding'],
  default_model: 'glm_for_coding',
})

const emptyMcpServer = (): MCPServiceConfig => ({
  id: `mcp-${Date.now()}`,
  name: '新的 MCP 服务',
  transport: 'http',
  endpoint: 'http://127.0.0.1:3001',
  enabled: true,
  description: '',
})

export function SettingsDialog({ open, settings, onClose, onSave }: SettingsDialogProps) {
  const [section, setSection] = useState<'llm' | 'base' | 'mcp'>('llm')
  const [draft, setDraft] = useState<AppSettings | null>(settings)
  const [saving, setSaving] = useState(false)
  const [testingMcpId, setTestingMcpId] = useState<string | null>(null)
  const [mcpTestResults, setMcpTestResults] = useState<Record<string, { ok: boolean; message: string; tools: string[] }>>({})

  useEffect(() => {
    setDraft(settings)
  }, [settings])

  const hasChanges = useMemo(() => JSON.stringify(draft) !== JSON.stringify(settings), [draft, settings])

  if (!open || !draft) return null

  const updateDraft = (patch: Partial<AppSettings>) => {
    setDraft((prev) => (prev ? { ...prev, ...patch } : prev))
  }

  const updateProfile = (id: string, patch: Partial<LLMProfile>) => {
    const nextProfiles = draft.llm_profiles.map((profile) => {
      if (profile.id !== id) return profile
      const next = { ...profile, ...patch }
      if (!next.models.includes(next.default_model)) {
        next.default_model = next.models[0] || ''
      }
      return next
    })
    updateDraft({ llm_profiles: nextProfiles })
  }

  const removeProfile = (id: string) => {
    if (draft.llm_profiles.length <= 1) return
    const nextProfiles = draft.llm_profiles.filter((profile) => profile.id !== id)
    const activeProfile = nextProfiles.find((profile) => profile.id === draft.active_profile_id) || nextProfiles[0]
    updateDraft({
      llm_profiles: nextProfiles,
      active_profile_id: activeProfile.id,
      default_model: activeProfile.default_model,
      active_model: activeProfile.default_model,
    })
  }

  const addProfile = () => {
    const profile = emptyProfile()
    updateDraft({
      llm_profiles: [...draft.llm_profiles, profile],
      active_profile_id: profile.id,
      default_model: profile.default_model,
      active_model: profile.default_model,
    })
  }

  const setActiveProfile = (profileId: string) => {
    const profile = draft.llm_profiles.find((item) => item.id === profileId)
    if (!profile) return
    updateDraft({
      active_profile_id: profileId,
      default_model: profile.default_model,
      active_model: profile.default_model,
    })
  }

  const updateMcp = (id: string, patch: Partial<MCPServiceConfig>) => {
    updateDraft({
      mcp_servers: draft.mcp_servers.map((server) => (server.id === id ? { ...server, ...patch } : server)),
    })
  }

  const addMcp = () => updateDraft({ mcp_servers: [...draft.mcp_servers, emptyMcpServer()] })

  const removeMcp = (id: string) => updateDraft({ mcp_servers: draft.mcp_servers.filter((server) => server.id !== id) })

  const testMcp = async (server: MCPServiceConfig) => {
    setTestingMcpId(server.id)
    try {
      const res = await settingsApi.testMcp(server)
      const tools = Array.isArray(res.data?.tools)
        ? res.data.tools.map((item: any) => item?.name).filter(Boolean)
        : []
      setMcpTestResults((prev) => ({
        ...prev,
        [server.id]: {
          ok: !!res.data?.ok,
          message: res.data?.message || '测试完成',
          tools,
        },
      }))
    } catch (err: any) {
      setMcpTestResults((prev) => ({
        ...prev,
        [server.id]: {
          ok: false,
          message: err.response?.data?.detail || err.message || '测试失败',
          tools: [],
        },
      }))
    } finally {
      setTestingMcpId(null)
    }
  }

  const handleSave = async () => {
    if (!draft) return
    setSaving(true)
    try {
      await onSave({
        ...draft,
        updated_at: new Date().toISOString(),
      })
    } finally {
      setSaving(false)
    }
  }

  const activeProfile = draft.llm_profiles.find((profile) => profile.id === draft.active_profile_id) || draft.llm_profiles[0]

  return (
    <div className="h-full overflow-hidden bg-transparent p-5">
      <div className="mx-auto flex h-full max-w-6xl overflow-hidden rounded-[2rem] border border-black/[0.06] bg-white/72 shadow-[0_24px_80px_rgba(24,24,27,0.10)] backdrop-blur-2xl">
        <aside className="w-56 shrink-0 border-r border-black/[0.06] bg-[#eee9df]/70 p-4">
          <div className="mb-5 flex items-center justify-between">
            <div>
              <div className="text-lg font-bold tracking-tight text-surface-950">设置</div>
              <div className="mt-0.5 text-[11px] text-surface-500">模型 · 基础信息 · MCP</div>
            </div>
            <button onClick={onClose} className="rounded-full bg-white/75 p-2 text-surface-500 hover:bg-white hover:text-surface-950" title="关闭">
              <X className="h-4 w-4" />
            </button>
          </div>

          <div className="space-y-1.5">
            {[
              ['llm', '模型服务', Server],
              ['base', '基础信息', ShieldCheck],
              ['mcp', 'MCP 服务', KeyRound],
            ].map(([key, label, Icon]: any) => (
              <button
                key={key}
                type="button"
                onClick={() => setSection(key)}
                className={`flex w-full items-center gap-2 rounded-2xl px-3 py-2.5 text-left text-sm font-semibold transition-all ${section === key ? 'bg-surface-950 text-white shadow-sm' : 'text-surface-600 hover:bg-white/75 hover:text-surface-950'}`}
              >
                <Icon className="h-4 w-4" />
                {label}
              </button>
            ))}
          </div>

          <div className="mt-6 rounded-3xl border border-black/[0.05] bg-white/60 p-3 text-[11px] leading-relaxed text-surface-500">
            <div className="mb-1 font-bold text-surface-700">{draft.basic.app_name}</div>
            <div className="truncate">{activeProfile?.name || '未配置模型服务'}</div>
            <div className="mt-1 truncate font-mono text-[10px] text-surface-400">{draft.active_model}</div>
          </div>
        </aside>

        <main className="min-w-0 flex-1 overflow-y-auto p-6">
          {section === 'llm' && (
            <div>
              <div className="mb-5 flex items-start justify-between gap-4">
                <div>
                  <h2 className="text-xl font-bold tracking-tight text-surface-950">模型服务</h2>
                  <p className="mt-1 text-sm text-surface-500">配置多个模型服务，选择默认启用的模型。</p>
                </div>
                <button onClick={addProfile} className="inline-flex items-center gap-1.5 rounded-full bg-surface-950 px-4 py-2 text-sm font-semibold text-white hover:bg-surface-800">
                  <Plus className="h-4 w-4" />
                  添加配置
                </button>
              </div>

              <div className="mb-5 rounded-[1.5rem] border border-black/[0.06] bg-[#f8f5ee]/80 p-4">
                <label className="mb-2 block text-xs font-bold uppercase tracking-[0.14em] text-surface-500">当前默认模型</label>
                <div className="grid gap-3 md:grid-cols-2">
                  <select
                    value={draft.active_profile_id}
                    onChange={(event) => setActiveProfile(event.target.value)}
                    className="rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm outline-none focus:border-surface-500"
                  >
                    {draft.llm_profiles.map((profile) => (
                      <option key={profile.id} value={profile.id}>{profile.name}</option>
                    ))}
                  </select>
                  <select
                    value={draft.active_model}
                    onChange={(event) => updateDraft({ active_model: event.target.value, default_model: event.target.value })}
                    className="rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm outline-none focus:border-surface-500"
                  >
                    {(activeProfile?.models || []).map((model) => (
                      <option key={model} value={model}>{model}</option>
                    ))}
                  </select>
                </div>
              </div>

              <div className="grid gap-4 xl:grid-cols-2">
                {draft.llm_profiles.map((profile) => {
                  const isActive = draft.active_profile_id === profile.id
                  return (
                    <div key={profile.id} className={`rounded-[1.6rem] border p-4 shadow-sm transition-all ${isActive ? 'border-surface-900 bg-white ring-2 ring-surface-950/5' : 'border-black/[0.06] bg-white/75'}`}>
                      <div className="mb-4 flex items-start justify-between gap-3">
                        <div className="min-w-0 flex-1">
                          <input
                            value={profile.name}
                            onChange={(event) => updateProfile(profile.id, { name: event.target.value })}
                            className="w-full bg-transparent text-base font-bold text-surface-950 outline-none"
                            placeholder="配置名称"
                          />
                          <div className="mt-1 flex items-center gap-2 text-[11px] text-surface-400">
                            <KeyRound className="h-3 w-3" />
                            {profile.has_api_key || profile.api_key ? 'API Key 已配置' : 'API Key 未配置'}
                          </div>
                        </div>
                        <div className="flex items-center gap-1.5">
                          {isActive ? (
                            <span className="inline-flex items-center gap-1 rounded-full bg-surface-950 px-2.5 py-1 text-[10px] font-bold text-white">
                              <Check className="h-3 w-3" />启用中
                            </span>
                          ) : (
                            <button onClick={() => setActiveProfile(profile.id)} className="rounded-full border border-black/10 bg-white px-3 py-1 text-xs font-semibold text-surface-600 hover:bg-surface-50">启用</button>
                          )}
                          <button
                            onClick={() => removeProfile(profile.id)}
                            disabled={draft.llm_profiles.length <= 1}
                            className="rounded-full p-2 text-surface-400 hover:bg-red-50 hover:text-red-600 disabled:opacity-30"
                            title="删除配置"
                          >
                            <Trash2 className="h-4 w-4" />
                          </button>
                        </div>
                      </div>

                      <div className="space-y-3">
                        <label className="block text-xs font-semibold text-surface-500">
                          Base URL
                          <input
                            value={profile.base_url}
                            onChange={(event) => updateProfile(profile.id, { base_url: event.target.value })}
                            className="mt-1.5 w-full rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm text-surface-900 outline-none focus:border-surface-500"
                          />
                        </label>
                        <label className="block text-xs font-semibold text-surface-500">
                          API Key
                          <input
                            value={profile.api_key || ''}
                            onChange={(event) => updateProfile(profile.id, { api_key: event.target.value, has_api_key: !!event.target.value })}
                            type="password"
                            placeholder={profile.has_api_key ? '已保存，重新输入可覆盖' : 'sk-...'}
                            className="mt-1.5 w-full rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm text-surface-900 outline-none focus:border-surface-500"
                          />
                        </label>
                        <label className="block text-xs font-semibold text-surface-500">
                          模型列表（逗号分隔）
                          <input
                            value={(profile.models || []).join(', ')}
                            onChange={(event) => {
                              const models = event.target.value.split(',').map((item) => item.trim()).filter(Boolean)
                              updateProfile(profile.id, { models, default_model: models[0] || profile.default_model })
                            }}
                            className="mt-1.5 w-full rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm text-surface-900 outline-none focus:border-surface-500"
                          />
                        </label>
                      </div>
                    </div>
                  )
                })}
              </div>
            </div>
          )}

          {section === 'base' && (
            <div>
              <h2 className="text-xl font-bold tracking-tight text-surface-950">基础信息</h2>
              <p className="mt-1 text-sm text-surface-500">配置产品名称、工作区标题和默认主题。</p>
              <div className="mt-5 grid gap-4 md:grid-cols-2">
                <label className="block text-sm font-semibold text-surface-600">
                  产品名称
                  <input
                    value={draft.basic.app_name}
                    onChange={(event) => updateDraft({ basic: { ...draft.basic, app_name: event.target.value } })}
                    className="mt-2 w-full rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm outline-none focus:border-surface-500"
                  />
                </label>
                <label className="block text-sm font-semibold text-surface-600">
                  工作区标题
                  <input
                    value={draft.basic.workspace_title}
                    onChange={(event) => updateDraft({ basic: { ...draft.basic, workspace_title: event.target.value } })}
                    className="mt-2 w-full rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm outline-none focus:border-surface-500"
                  />
                </label>
                <label className="block text-sm font-semibold text-surface-600 md:col-span-2">
                  品牌副标题
                  <input
                    value={draft.basic.brand_tagline}
                    onChange={(event) => updateDraft({ basic: { ...draft.basic, brand_tagline: event.target.value } })}
                    className="mt-2 w-full rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm outline-none focus:border-surface-500"
                  />
                </label>
                <label className="block text-sm font-semibold text-surface-600">
                  默认主题
                  <select
                    value={draft.basic.default_theme}
                    onChange={(event) => updateDraft({ basic: { ...draft.basic, default_theme: event.target.value } })}
                    className="mt-2 w-full rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm outline-none focus:border-surface-500"
                  >
                    {['default', 'business', 'tech', 'warm', 'minimal'].map((theme) => (
                      <option key={theme} value={theme}>{theme}</option>
                    ))}
                  </select>
                </label>
              </div>
            </div>
          )}

          {section === 'mcp' && (
            <div>
              <div className="mb-5 flex items-start justify-between gap-4">
                <div>
                  <h2 className="text-xl font-bold tracking-tight text-surface-950">MCP 服务</h2>
                  <p className="mt-1 text-sm text-surface-500">保存常用 MCP 服务的地址和启用状态，方便后续接入。</p>
                </div>
                <button onClick={addMcp} className="inline-flex items-center gap-1.5 rounded-full bg-surface-950 px-4 py-2 text-sm font-semibold text-white hover:bg-surface-800">
                  <Plus className="h-4 w-4" />
                  添加服务
                </button>
              </div>

              <div className="space-y-4">
                {draft.mcp_servers.length === 0 && (
                  <div className="rounded-[1.6rem] border border-dashed border-black/10 bg-white/65 px-4 py-8 text-center text-sm text-surface-500">
                    还没有 MCP 服务配置，添加后会保存在当前账号下。
                  </div>
                )}
                {draft.mcp_servers.map((server) => (
                  <div key={server.id} className="rounded-[1.6rem] border border-black/[0.06] bg-white/75 p-4 shadow-sm">
                    <div className="mb-4 flex items-start justify-between gap-3">
                      <div className="min-w-0 flex-1">
                        <input
                          value={server.name}
                          onChange={(event) => updateMcp(server.id, { name: event.target.value })}
                          className="w-full bg-transparent text-base font-bold text-surface-950 outline-none"
                          placeholder="服务名称"
                        />
                        <div className="mt-1 text-[11px] text-surface-400">{server.enabled ? '已启用' : '已停用'}</div>
                      </div>
                      <div className="flex items-center gap-2">
                        <button
                          type="button"
                          onClick={() => testMcp(server)}
                          disabled={testingMcpId === server.id}
                          className="rounded-full border border-black/10 bg-white px-3 py-1 text-xs font-semibold text-surface-600 hover:bg-surface-50 disabled:cursor-not-allowed disabled:opacity-50"
                        >
                          {testingMcpId === server.id ? '测试中...' : '测试连接'}
                        </button>
                        <label className="flex items-center gap-2 text-xs font-semibold text-surface-500">
                          <input
                            type="checkbox"
                            checked={server.enabled}
                            onChange={(event) => updateMcp(server.id, { enabled: event.target.checked })}
                          />
                          启用
                        </label>
                        <button onClick={() => removeMcp(server.id)} className="rounded-full p-2 text-surface-400 hover:bg-red-50 hover:text-red-600" title="删除服务">
                          <Trash2 className="h-4 w-4" />
                        </button>
                      </div>
                    </div>

                    <div className="grid gap-3 md:grid-cols-2">
                      <label className="block text-xs font-semibold text-surface-500">
                        传输方式
                        <select
                          value={server.transport}
                          onChange={(event) => updateMcp(server.id, { transport: event.target.value })}
                          className="mt-1.5 w-full rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm outline-none focus:border-surface-500"
                        >
                          <option value="http">HTTP</option>
                          <option value="sse">SSE</option>
                          <option value="stdio">STDIO</option>
                        </select>
                      </label>
                      <label className="block text-xs font-semibold text-surface-500">
                        服务地址
                        <input
                          value={server.endpoint}
                          onChange={(event) => updateMcp(server.id, { endpoint: event.target.value })}
                          className="mt-1.5 w-full rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm outline-none focus:border-surface-500"
                        />
                      </label>
                      <label className="block text-xs font-semibold text-surface-500 md:col-span-2">
                        描述
                        <input
                          value={server.description || ''}
                          onChange={(event) => updateMcp(server.id, { description: event.target.value })}
                          className="mt-1.5 w-full rounded-2xl border border-black/10 bg-white px-3 py-2.5 text-sm outline-none focus:border-surface-500"
                        />
                      </label>
                    </div>

                    {mcpTestResults[server.id] && (
                      <div className={`mt-3 rounded-2xl border px-3 py-2 text-xs ${mcpTestResults[server.id].ok ? 'border-emerald-200 bg-emerald-50 text-emerald-700' : 'border-red-200 bg-red-50 text-red-600'}`}>
                        <div className="font-semibold">{mcpTestResults[server.id].message}</div>
                        {mcpTestResults[server.id].tools.length > 0 && (
                          <div className="mt-1 text-[11px]">
                            可用工具：{mcpTestResults[server.id].tools.join(' / ')}
                          </div>
                        )}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          )}

          <div className="mt-6 flex items-center justify-end gap-3 border-t border-black/[0.06] pt-5">
            <button onClick={onClose} className="rounded-full border border-black/10 bg-white px-4 py-2 text-sm font-semibold text-surface-600 hover:bg-surface-50">
              取消
            </button>
            <button
              onClick={handleSave}
              disabled={!hasChanges || saving}
              className="rounded-full bg-surface-950 px-5 py-2 text-sm font-semibold text-white disabled:cursor-not-allowed disabled:opacity-40"
            >
              {saving ? '保存中...' : '保存设置'}
            </button>
          </div>
        </main>
      </div>
    </div>
  )
}
