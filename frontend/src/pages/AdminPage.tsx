import { useEffect, useState } from 'react'
import { useAuthStore } from '@/stores/auth-store'
import { tenantApi } from '@/api'
import type { Tenant, User } from '@/types'
import { Loader2, Users, RefreshCw, Trash2, Plus, Copy, Check, Crown, Shield } from 'lucide-react'

const ROLE_LABEL: Record<string, string> = {
  super_admin: '超级管理员',
  tenant_admin: '租户管理员',
  member: '成员',
}

export default function AdminPage() {
  const user = useAuthStore((s) => s.user)
  const isSuperAdmin = user?.role === 'super_admin'
  const canManageTenants = isSuperAdmin
  const canManageMembers = user?.role === 'tenant_admin' || isSuperAdmin

  const [tenants, setTenants] = useState<Tenant[]>([])
  const [members, setMembers] = useState<User[]>([])
  const [activeTenant, setActiveTenant] = useState<Tenant | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [copied, setCopied] = useState('')

  // 新建租户表单
  const [newName, setNewName] = useState('')
  const [newSlug, setNewSlug] = useState('')
  const [newPlan, setNewPlan] = useState('free')
  const [addUserId, setAddUserId] = useState('')

  const loadTenants = async () => {
    if (!isSuperAdmin) return
    setLoading(true)
    setError('')
    try {
      const { data } = await tenantApi.list()
      setTenants(data.tenants || [])
    } catch (e: any) {
      setError(e.response?.data?.detail || '加载租户失败')
    } finally {
      setLoading(false)
    }
  }

  const loadMyTenant = async () => {
    if (isSuperAdmin) return
    setLoading(true)
    setError('')
    try {
      const { data } = await tenantApi.myTenant()
      const t = (data as any)?.tenant || (data as Tenant)
      if (t && t.id) {
        setActiveTenant(t as Tenant)
        loadMembers((t as Tenant).id)
      } else {
        setActiveTenant(null)
      }
    } catch (e: any) {
      setError(e.response?.data?.detail || '加载租户失败')
    } finally {
      setLoading(false)
    }
  }

  const loadMembers = async (tenantId: string) => {
    try {
      const { data } = await tenantApi.listMembers(tenantId)
      setMembers(data.members || [])
    } catch (e: any) {
      setError(e.response?.data?.detail || '加载成员失败')
    }
  }

  useEffect(() => {
    if (isSuperAdmin) {
      loadTenants()
    } else {
      loadMyTenant()
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [user?.role])

  const handleCreateTenant = async () => {
    if (!newName.trim() || !newSlug.trim()) {
      setError('租户名称和标识不能为空')
      return
    }
    setLoading(true)
    setError('')
    try {
      await tenantApi.create(newName.trim(), newSlug.trim(), newPlan)
      setNewName('')
      setNewSlug('')
      await loadTenants()
    } catch (e: any) {
      setError(e.response?.data?.detail || '创建租户失败')
    } finally {
      setLoading(false)
    }
  }

  const handleDeleteTenant = async (id: string) => {
    if (!confirm('确认删除该租户？')) return
    try {
      await tenantApi.delete(id)
      await loadTenants()
    } catch (e: any) {
      setError(e.response?.data?.detail || '删除租户失败')
    }
  }

  const handleResetInvite = async (tenantId: string) => {
    try {
      await tenantApi.resetInviteCode(tenantId)
      await loadTenants()
      if (activeTenant?.id === tenantId) {
        const { data } = await tenantApi.myTenant()
        const t = (data as any)?.tenant || (data as Tenant)
        if (t?.id) setActiveTenant(t as Tenant)
      }
    } catch (e: any) {
      setError(e.response?.data?.detail || '重置邀请码失败')
    }
  }

  const handleAddMember = async () => {
    if (!addUserId.trim()) {
      setError('请输入用户 ID')
      return
    }
    const targetId = activeTenant?.id || (tenants[0]?.id as string)
    if (!targetId) return
    try {
      await tenantApi.addMember(targetId, addUserId.trim())
      setAddUserId('')
      await loadMembers(targetId)
    } catch (e: any) {
      setError(e.response?.data?.detail || '添加成员失败')
    }
  }

  const handleRoleChange = async (targetId: string, memberId: string, role: string) => {
    try {
      if (isSuperAdmin) {
        await tenantApi.updateUserRole(memberId, role)
      } else {
        await tenantApi.updateMemberRole(targetId, memberId, role)
      }
      await loadMembers(targetId)
    } catch (e: any) {
      setError(e.response?.data?.detail || '更新角色失败')
    }
  }

  const handleRemoveMember = async (targetId: string, memberId: string) => {
    if (!confirm('确认移除该成员？')) return
    try {
      await tenantApi.removeMember(targetId, memberId)
      await loadMembers(targetId)
    } catch (e: any) {
      setError(e.response?.data?.detail || '移除成员失败')
    }
  }

  const copyInviteCode = (code: string) => {
    navigator.clipboard?.writeText(code)
    setCopied(code)
    setTimeout(() => setCopied(''), 1500)
  }

  const selectTenant = (t: Tenant) => {
    setActiveTenant(t)
    loadMembers(t.id)
  }

  return (
    <div className="mx-auto w-full max-w-4xl space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-extrabold tracking-tight text-surface-950">多租户管理</h1>
          <p className="mt-1 text-sm text-surface-500">
            {isSuperAdmin ? '管理平台所有租户与成员' : '管理你所在租户的成员与邀请码'}
          </p>
        </div>
        {isSuperAdmin && (
          <button
            onClick={loadTenants}
            disabled={loading}
            className="inline-flex items-center gap-2 rounded-full bg-surface-950 px-4 py-2 text-sm font-semibold text-white transition hover:opacity-90 disabled:opacity-50"
          >
            <RefreshCw className={`h-4 w-4 ${loading ? 'animate-spin' : ''}`} />
            刷新
          </button>
        )}
      </div>

      {error && (
        <div className="rounded-lg bg-red-50 px-4 py-3 text-sm text-red-600">{error}</div>
      )}

      {/* 超级管理员：创建租户 + 租户列表 */}
      {isSuperAdmin && (
        <section className="rounded-2xl border border-black/[0.06] bg-white p-5 shadow-sm">
          <h2 className="flex items-center gap-2 text-base font-bold text-surface-900">
            <Crown className="h-4 w-4 text-amber-500" /> 创建租户
          </h2>
          <div className="mt-3 flex flex-wrap gap-3">
            <input
              className="h-10 flex-1 rounded-lg border border-surface-200 px-3 text-sm outline-none focus:border-primary-500"
              placeholder="租户名称（如 碳基脉冲）"
              value={newName}
              onChange={(e) => setNewName(e.target.value)}
            />
            <input
              className="h-10 flex-1 rounded-lg border border-surface-200 px-3 text-sm outline-none focus:border-primary-500"
              placeholder="标识 slug（如 carbonpulse）"
              value={newSlug}
              onChange={(e) => setNewSlug(e.target.value)}
            />
            <select
              className="h-10 rounded-lg border border-surface-200 px-3 text-sm outline-none"
              value={newPlan}
              onChange={(e) => setNewPlan(e.target.value)}
            >
              <option value="free">free</option>
              <option value="pro">pro</option>
              <option value="enterprise">enterprise</option>
            </select>
            <button
              onClick={handleCreateTenant}
              disabled={loading}
              className="inline-flex h-10 items-center gap-2 rounded-lg bg-primary-600 px-4 text-sm font-semibold text-white transition hover:bg-primary-700 disabled:opacity-50"
            >
              <Plus className="h-4 w-4" /> 创建
            </button>
          </div>
        </section>
      )}

      {/* 租户列表（超管）或当前租户（租户管理员） */}
      {isSuperAdmin ? (
        <section className="rounded-2xl border border-black/[0.06] bg-white p-5 shadow-sm">
          <h2 className="text-base font-bold text-surface-900">租户列表</h2>
          {loading && tenants.length === 0 ? (
            <div className="flex items-center gap-2 py-8 text-sm text-surface-500">
              <Loader2 className="h-4 w-4 animate-spin" /> 加载中…
            </div>
          ) : (
            <div className="mt-3 divide-y divide-black/[0.04]">
              {tenants.map((t) => (
                <div key={t.id} className="flex items-center justify-between gap-3 py-3">
                  <button
                    onClick={() => selectTenant(t)}
                    className="flex min-w-0 flex-1 items-center gap-3 text-left"
                  >
                    <div className="flex h-9 w-9 items-center justify-center rounded-lg bg-surface-100 text-sm font-bold text-surface-700">
                      {t.name?.[0]?.toUpperCase() || 'T'}
                    </div>
                    <div className="min-w-0">
                      <div className="truncate text-sm font-semibold text-surface-900">{t.name}</div>
                      <div className="truncate text-xs text-surface-500">
                        {t.slug} · {t.plan} · {t.status}
                      </div>
                    </div>
                  </button>
                  <div className="flex items-center gap-2">
                    <button
                      onClick={() => t.invite_code && copyInviteCode(t.invite_code)}
                      className="inline-flex items-center gap-1 rounded-lg px-2 py-1 text-xs text-surface-500 transition hover:bg-surface-100"
                      title="复制邀请码"
                    >
                      {copied === t.invite_code ? <Check className="h-3.5 w-3.5 text-green-500" /> : <Copy className="h-3.5 w-3.5" />}
                      邀请
                    </button>
                    <button
                      onClick={() => handleResetInvite(t.id)}
                      className="inline-flex items-center gap-1 rounded-lg px-2 py-1 text-xs text-surface-500 transition hover:bg-surface-100"
                      title="重置邀请码"
                    >
                      <RefreshCw className="h-3.5 w-3.5" /> 重置
                    </button>
                    <button
                      onClick={() => handleDeleteTenant(t.id)}
                      className="inline-flex items-center gap-1 rounded-lg px-2 py-1 text-xs text-red-500 transition hover:bg-red-50"
                    >
                      <Trash2 className="h-3.5 w-3.5" />
                    </button>
                  </div>
                </div>
              ))}
              {tenants.length === 0 && !loading && (
                <div className="py-8 text-center text-sm text-surface-400">暂无租户</div>
              )}
            </div>
          )}
        </section>
      ) : (
        activeTenant && (
          <section className="rounded-2xl border border-black/[0.06] bg-white p-5 shadow-sm">
            <div className="flex items-center justify-between">
              <h2 className="flex items-center gap-2 text-base font-bold text-surface-900">
                <Shield className="h-4 w-4 text-primary-500" /> 我的租户：{activeTenant.name}
              </h2>
              <button
                onClick={() => handleResetInvite(activeTenant.id)}
                className="inline-flex items-center gap-1 rounded-lg px-2 py-1 text-xs text-surface-500 transition hover:bg-surface-100"
              >
                <RefreshCw className="h-3.5 w-3.5" /> 重置邀请码
              </button>
            </div>
            {activeTenant.invite_code && (
              <div className="mt-3 flex items-center gap-2 rounded-lg bg-surface-50 px-3 py-2">
                <span className="text-xs text-surface-500">邀请码：</span>
                <code className="flex-1 truncate text-sm font-mono text-primary-700">{activeTenant.invite_code}</code>
                <button
                  onClick={() => copyInviteCode(activeTenant.invite_code!)}
                  className="inline-flex items-center gap-1 rounded px-2 py-1 text-xs text-surface-500 hover:bg-surface-100"
                >
                  {copied === activeTenant.invite_code ? <Check className="h-3.5 w-3.5 text-green-500" /> : <Copy className="h-3.5 w-3.5" />}
                  复制
                </button>
              </div>
            )}
          </section>
        )
      )}

      {/* 成员管理 */}
      {canManageMembers && (
        <section className="rounded-2xl border border-black/[0.06] bg-white p-5 shadow-sm">
          <div className="flex items-center justify-between">
            <h2 className="flex items-center gap-2 text-base font-bold text-surface-900">
              <Users className="h-4 w-4 text-primary-500" /> 成员管理
            </h2>
          </div>

          {activeTenant && (
            <div className="mt-3 flex gap-2">
              <input
                className="h-10 flex-1 rounded-lg border border-surface-200 px-3 text-sm outline-none focus:border-primary-500"
                placeholder="输入用户 ID 添加成员"
                value={addUserId}
                onChange={(e) => setAddUserId(e.target.value)}
              />
              <button
                onClick={handleAddMember}
                className="inline-flex h-10 items-center gap-2 rounded-lg bg-primary-600 px-4 text-sm font-semibold text-white transition hover:bg-primary-700"
              >
                <Plus className="h-4 w-4" /> 添加
              </button>
            </div>
          )}

          <div className="mt-3 divide-y divide-black/[0.04]">
            {members.map((m) => (
              <div key={m.id} className="flex items-center justify-between gap-3 py-3">
                <div className="flex min-w-0 items-center gap-3">
                  <div className="flex h-9 w-9 items-center justify-center rounded-full bg-surface-950 text-xs font-bold text-white">
                    {m.username?.[0]?.toUpperCase() || 'U'}
                  </div>
                  <div className="min-w-0">
                    <div className="truncate text-sm font-semibold text-surface-900">{m.username}</div>
                    <div className="truncate text-xs text-surface-500">{m.email || m.id}</div>
                  </div>
                </div>
                <div className="flex items-center gap-2">
                  <select
                    className="h-9 rounded-lg border border-surface-200 px-2 text-xs outline-none"
                    value={m.role || 'member'}
                    onChange={(e) => activeTenant && handleRoleChange(activeTenant.id, m.id, e.target.value)}
                  >
                    <option value="member">{ROLE_LABEL.member}</option>
                    <option value="tenant_admin">{ROLE_LABEL.tenant_admin}</option>
                  </select>
                  <button
                    onClick={() => activeTenant && handleRemoveMember(activeTenant.id, m.id)}
                    className="inline-flex items-center rounded-lg px-2 py-1 text-xs text-red-500 transition hover:bg-red-50"
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </button>
                </div>
              </div>
            ))}
            {members.length === 0 && (
              <div className="py-6 text-center text-sm text-surface-400">暂无成员</div>
            )}
          </div>
        </section>
      )}
    </div>
  )
}
