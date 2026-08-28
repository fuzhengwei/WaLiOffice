import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuthStore } from '@/stores/auth-store'
import { authApi } from '@/api'
import { Loader2 } from 'lucide-react'

const LOGO_URL = '/logo.png'

type Mode = 'verify' | 'password' | 'register' | 'invite'

export default function Login() {
  const navigate = useNavigate()
  const login = useAuthStore((s) => s.login)

  const [mode, setMode] = useState<Mode>('verify')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  // 验证码登录
  const [verificationCode, setVerificationCode] = useState('')
  const [agreed, setAgreed] = useState(false)

  // 用户名密码登录
  const [username, setUsername] = useState('')
  const [password, setPassword] = useState('')

  // 注册
  const [regUsername, setRegUsername] = useState('')
  const [regPassword, setRegPassword] = useState('')
  const [regEmail, setRegEmail] = useState('')

  // 邀请码注册
  const [inviteCode, setInviteCode] = useState('')
  const [invUsername, setInvUsername] = useState('')
  const [invPassword, setInvPassword] = useState('')

  // 飞书登录
  const [feishuEnabled, setFeishuEnabled] = useState(false)
  const [feishuAppId, setFeishuAppId] = useState('')
  const [feishuRedirect, setFeishuRedirect] = useState('')

  useEffect(() => {
    authApi.feishuConfig().then(({ data }) => {
      if (data.enabled) {
        setFeishuEnabled(true)
        setFeishuAppId(data.app_id)
        setFeishuRedirect(data.redirect_uri)
      }
    }).catch(() => {})

    // 处理飞书 OAuth 回调：URL 里带 code
    const params = new URLSearchParams(window.location.search)
    const code = params.get('code')
    if (code) {
      handleFeishuCallback(code)
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [])

  const handleFeishuCallback = async (code: string) => {
    setLoading(true)
    setError('')
    try {
      const { data } = await authApi.feishuLogin(code)
      login(data.access_token, data.user)
      navigate('/')
    } catch (err: any) {
      setError(err.response?.data?.detail || '飞书登录失败')
    } finally {
      setLoading(false)
    }
  }

  const handleFeishuLogin = () => {
    const redirect = feishuRedirect || `${window.location.origin}/login`
    const url = `https://open.feishu.cn/open-apis/authen/v1/authorize?app_id=${feishuAppId}&redirect_uri=${encodeURIComponent(redirect)}&state=${Date.now()}`
    window.location.href = url
  }

  const handleVerifySubmit = async (e?: React.FormEvent) => {
    e?.preventDefault()
    setError('')
    if (!verificationCode.trim()) return setError('请填写访问验证码')
    if (!agreed) return setError('请先同意用户协议')
    setLoading(true)
    try {
      const { data } = await authApi.verificationLogin(verificationCode.trim())
      login(data.access_token, data.user)
      navigate('/')
    } catch (err: any) {
      setError(err.response?.data?.detail || '登录失败，请重新获取验证码')
    } finally {
      setLoading(false)
    }
  }

  const handlePasswordSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault()
    setError('')
    if (!username.trim()) return setError('请输入用户名')
    if (!password) return setError('请输入密码')
    setLoading(true)
    try {
      const { data } = await authApi.login(username.trim(), password)
      login(data.access_token, data.user)
      navigate('/')
    } catch (err: any) {
      setError(err.response?.data?.detail || '用户名或密码错误')
    } finally {
      setLoading(false)
    }
  }

  const handleRegisterSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault()
    setError('')
    if (regUsername.trim().length < 3) return setError('用户名至少 3 个字符')
    if (regPassword.length < 6) return setError('密码至少 6 个字符')
    setLoading(true)
    try {
      const { data } = await authApi.register(regUsername.trim(), regEmail.trim(), regPassword)
      login(data.access_token, data.user)
      navigate('/')
    } catch (err: any) {
      setError(err.response?.data?.detail || '注册失败')
    } finally {
      setLoading(false)
    }
  }

  const handleInviteSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault()
    setError('')
    if (!inviteCode.trim()) return setError('请输入邀请码')
    if (invUsername.trim().length < 3) return setError('用户名至少 3 个字符')
    if (invPassword.length < 6) return setError('密码至少 6 个字符')
    setLoading(true)
    try {
      const { data } = await authApi.registerByInvite(inviteCode.trim(), invUsername.trim(), invPassword)
      login(data.access_token, data.user)
      navigate('/')
    } catch (err: any) {
      setError(err.response?.data?.detail || '邀请注册失败')
    } finally {
      setLoading(false)
    }
  }

  const TABS: { key: Mode; label: string }[] = [
    { key: 'verify', label: '验证码登录' },
    { key: 'password', label: '账号登录' },
    { key: 'register', label: '注册' },
    { key: 'invite', label: '邀请注册' },
  ]

  return (
    <div className="min-h-screen bg-white px-4 py-8 text-surface-900">
      <main className="mx-auto flex min-h-[calc(100vh-4rem)] w-full max-w-xl flex-col items-center justify-center text-center">
        <div className="mb-5 flex h-16 w-16 items-center justify-center overflow-hidden rounded-2xl bg-white shadow-sm ring-1 ring-black/[0.06]">
          <img src={LOGO_URL} alt="WaLiOffice logo" className="h-full w-full object-cover" />
        </div>

        <h1 className="text-4xl font-extrabold tracking-tight text-surface-900">WaLiOffice</h1>
        <p className="mt-4 text-lg font-semibold text-surface-700">学习AI办公、掌握AI部署、运用AI提效</p>

        {/* Tab 切换 */}
        <div className="mt-8 flex w-full max-w-md items-center justify-center gap-1 rounded-full bg-surface-100 p-1">
          {TABS.map((t) => (
            <button
              key={t.key}
              type="button"
              onClick={() => { setMode(t.key); setError('') }}
              className={`flex-1 rounded-full px-3 py-2 text-sm font-medium transition ${
                mode === t.key ? 'bg-white text-surface-950 shadow-sm' : 'text-surface-500 hover:text-surface-800'
              }`}
            >
              {t.label}
            </button>
          ))}
        </div>

        <div className="mt-6 w-full max-w-sm space-y-4">
          {mode === 'verify' && (
            <form onSubmit={handleVerifySubmit} className="space-y-4">
              <input
                className="h-12 w-full rounded-xl border border-surface-200 bg-white px-4 text-center text-base outline-none transition placeholder:text-surface-300 focus:border-primary-500 focus:ring-4 focus:ring-primary-100"
                placeholder="请输入访问验证码"
                value={verificationCode}
                onChange={(e) => setVerificationCode(e.target.value)}
                autoComplete="one-time-code"
              />
              <label className="flex cursor-pointer items-center justify-center gap-2 text-sm text-surface-600">
                <input type="checkbox" checked={agreed} onChange={(e) => setAgreed(e.target.checked)} className="h-4 w-4 rounded border-surface-300 text-primary-600" />
                <span>同意用户协议</span>
              </label>
              <button type="submit" disabled={loading} className="mx-auto flex h-11 min-w-32 items-center justify-center rounded-xl bg-primary-600 px-6 text-base font-semibold text-white shadow-sm transition hover:bg-primary-700 disabled:opacity-60">
                {loading ? <Loader2 className="mr-2 h-5 w-5 animate-spin" /> : null}确认登录
              </button>
            </form>
          )}

          {mode === 'password' && (
            <form onSubmit={handlePasswordSubmit} className="space-y-4">
              <input className="h-12 w-full rounded-xl border border-surface-200 px-4 text-base outline-none focus:border-primary-500" placeholder="用户名" value={username} onChange={(e) => setUsername(e.target.value)} autoComplete="username" />
              <input type="password" className="h-12 w-full rounded-xl border border-surface-200 px-4 text-base outline-none focus:border-primary-500" placeholder="密码" value={password} onChange={(e) => setPassword(e.target.value)} autoComplete="current-password" />
              <button type="submit" disabled={loading} className="flex h-11 w-full items-center justify-center rounded-xl bg-primary-600 px-6 text-base font-semibold text-white transition hover:bg-primary-700 disabled:opacity-60">
                {loading ? <Loader2 className="mr-2 h-5 w-5 animate-spin" /> : null}登录
              </button>
            </form>
          )}

          {mode === 'register' && (
            <form onSubmit={handleRegisterSubmit} className="space-y-4">
              <input className="h-12 w-full rounded-xl border border-surface-200 px-4 text-base outline-none focus:border-primary-500" placeholder="用户名（至少 3 字符）" value={regUsername} onChange={(e) => setRegUsername(e.target.value)} />
              <input className="h-12 w-full rounded-xl border border-surface-200 px-4 text-base outline-none focus:border-primary-500" placeholder="邮箱（可选）" value={regEmail} onChange={(e) => setRegEmail(e.target.value)} />
              <input type="password" className="h-12 w-full rounded-xl border border-surface-200 px-4 text-base outline-none focus:border-primary-500" placeholder="密码（至少 6 字符）" value={regPassword} onChange={(e) => setRegPassword(e.target.value)} />
              <button type="submit" disabled={loading} className="flex h-11 w-full items-center justify-center rounded-xl bg-primary-600 px-6 text-base font-semibold text-white transition hover:bg-primary-700 disabled:opacity-60">
                {loading ? <Loader2 className="mr-2 h-5 w-5 animate-spin" /> : null}注册
              </button>
            </form>
          )}

          {mode === 'invite' && (
            <form onSubmit={handleInviteSubmit} className="space-y-4">
              <input className="h-12 w-full rounded-xl border border-surface-200 px-4 text-base outline-none focus:border-primary-500" placeholder="邀请码" value={inviteCode} onChange={(e) => setInviteCode(e.target.value)} />
              <input className="h-12 w-full rounded-xl border border-surface-200 px-4 text-base outline-none focus:border-primary-500" placeholder="用户名（至少 3 字符）" value={invUsername} onChange={(e) => setInvUsername(e.target.value)} />
              <input type="password" className="h-12 w-full rounded-xl border border-surface-200 px-4 text-base outline-none focus:border-primary-500" placeholder="密码（至少 6 字符）" value={invPassword} onChange={(e) => setInvPassword(e.target.value)} />
              <button type="submit" disabled={loading} className="flex h-11 w-full items-center justify-center rounded-xl bg-primary-600 px-6 text-base font-semibold text-white transition hover:bg-primary-700 disabled:opacity-60">
                {loading ? <Loader2 className="mr-2 h-5 w-5 animate-spin" /> : null}用邀请码注册
              </button>
            </form>
          )}

          {error && <div className="rounded-lg bg-red-50 px-4 py-2 text-sm text-red-600">{error}</div>}

          {/* 飞书登录 */}
          {feishuEnabled && (
            <button
              type="button"
              onClick={handleFeishuLogin}
              disabled={loading}
              className="flex h-11 w-full items-center justify-center gap-2 rounded-xl bg-[#3370ff] px-6 text-base font-semibold text-white transition hover:bg-[#2b5fd6] disabled:opacity-60"
            >
              飞书账号登录
            </button>
          )}
        </div>

        <p className="mt-12 text-sm font-medium leading-7 text-surface-600">
          说明：此平台主要以学习 OpenAI 为主，请合理、合法、合规的使用相关资料！
          <a href="https://bugstack.cn/" target="_blank" rel="noreferrer" className="ml-1 text-primary-600 underline underline-offset-2">查看用户协议</a>
        </p>
      </main>
    </div>
  )
}
