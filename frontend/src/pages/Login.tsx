import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuthStore } from '@/stores/auth-store'
import { authApi } from '@/api'
import { Sparkles, Loader2, User, Lock, Eye, EyeOff, Mail, UserPlus } from 'lucide-react'

interface DemoAccount {
  username: string
  password: string
  role: string
  description?: string
}

export default function Login() {
  const navigate = useNavigate()
  const login = useAuthStore((s) => s.login)
  const [mode, setMode] = useState<'login' | 'register'>('login')
  const [username, setUsername] = useState('')
  const [email, setEmail] = useState('')
  const [password, setPassword] = useState('')
  const [showPassword, setShowPassword] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')
  const [demoAccounts, setDemoAccounts] = useState<DemoAccount[]>([])

  useEffect(() => {
    let mounted = true

    authApi.getDemoAccounts()
      .then(({ data }) => {
        if (!mounted) return
        const accounts = Array.isArray(data?.accounts) ? data.accounts : []
        setDemoAccounts(accounts)
      })
      .catch(() => {
        if (!mounted) return
        setDemoAccounts([])
      })

    return () => {
      mounted = false
    }
  }, [])

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault()
    setError('')

    if (mode === 'register') {
      if (!username || !email || !password) {
        setError('请填写所有字段')
        return
      }
    } else {
      if (!username || !password) {
        setError('请输入用户名和密码')
        return
      }
    }

    setLoading(true)

    try {
      if (mode === 'register') {
        const { data } = await authApi.register(username, email, password)
        login(data.access_token, data.user)
      } else {
        const { data } = await authApi.login(username, password)
        login(data.access_token, data.user)
      }
      navigate('/')
    } catch (err: any) {
      setError(err.response?.data?.detail || (mode === 'register' ? '注册失败，请重试' : '登录失败，请重试'))
    } finally {
      setLoading(false)
    }
  }

  const fillDemo = (user: string, pass: string) => {
    setUsername(user)
    setPassword(pass)
    setError('')
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-primary-50 via-white to-accent-500/5">
      <div className="absolute inset-0 overflow-hidden pointer-events-none">
        <div className="absolute -top-40 -right-40 w-96 h-96 bg-primary-200/30 rounded-full blur-3xl" />
        <div className="absolute -bottom-40 -left-40 w-96 h-96 bg-accent-500/10 rounded-full blur-3xl" />
      </div>

      <div className="relative w-full max-w-md mx-4">
        <div className="text-center mb-8">
          <div className="inline-flex items-center justify-center w-16 h-16 bg-gradient-to-br from-primary-500 to-primary-700 rounded-2xl shadow-lg mb-4">
            <Sparkles className="w-8 h-8 text-white" />
          </div>
          <h1 className="text-3xl font-bold text-surface-900">WaLiOffice</h1>
          <p className="text-surface-500 mt-2">智能办公平台</p>
        </div>

        <div className="card p-8 animate-fade-in">
          {/* 模式切换 */}
          <div className="flex gap-1 mb-6 rounded-lg bg-surface-100 p-1">
            <button
              onClick={() => { setMode('login'); setError('') }}
              className={`flex-1 rounded-md py-2 text-sm font-medium transition-all ${
                mode === 'login' ? 'bg-white text-surface-900 shadow-sm' : 'text-surface-500'
              }`}
            >
              登录
            </button>
            <button
              onClick={() => { setMode('register'); setError('') }}
              className={`flex-1 rounded-md py-2 text-sm font-medium transition-all ${
                mode === 'register' ? 'bg-white text-surface-900 shadow-sm' : 'text-surface-500'
              }`}
            >
              注册
            </button>
          </div>

          <form onSubmit={handleSubmit} className="space-y-5">
            <div>
              <label className="block text-sm font-medium text-surface-700 mb-1.5">用户名</label>
              <div className="relative">
                <User className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
                <input
                  className="input pl-10"
                  placeholder="请输入用户名"
                  value={username}
                  onChange={(e) => setUsername(e.target.value)}
                  autoComplete="username"
                />
              </div>
            </div>

            {mode === 'register' && (
              <div>
                <label className="block text-sm font-medium text-surface-700 mb-1.5">邮箱</label>
                <div className="relative">
                  <Mail className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
                  <input
                    className="input pl-10"
                    type="email"
                    placeholder="请输入邮箱"
                    value={email}
                    onChange={(e) => setEmail(e.target.value)}
                    autoComplete="email"
                  />
                </div>
              </div>
            )}

            <div>
              <label className="block text-sm font-medium text-surface-700 mb-1.5">密码</label>
              <div className="relative">
                <Lock className="absolute left-3 top-1/2 -translate-y-1/2 w-4 h-4 text-surface-400" />
                <input
                  className="input pl-10 pr-10"
                  type={showPassword ? 'text' : 'password'}
                  placeholder={mode === 'register' ? '至少 6 个字符' : '请输入密码'}
                  value={password}
                  onChange={(e) => setPassword(e.target.value)}
                  autoComplete={mode === 'register' ? 'new-password' : 'current-password'}
                />
                <button
                  type="button"
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-surface-400 hover:text-surface-600"
                  onClick={() => setShowPassword(!showPassword)}
                >
                  {showPassword ? <EyeOff className="w-4 h-4" /> : <Eye className="w-4 h-4" />}
                </button>
              </div>
            </div>

            {error && (
              <div className="text-sm text-red-500 bg-red-50 rounded-lg px-3 py-2 animate-fade-in">
                {error}
              </div>
            )}

            <button type="submit" disabled={loading} className="btn-primary w-full h-11 text-base">
              {loading ? (
                <>
                  <Loader2 className="w-5 h-5 animate-spin" />
                  {mode === 'register' ? '注册中...' : '登录中...'}
                </>
              ) : mode === 'register' ? (
                <>
                  <UserPlus className="w-5 h-5" />
                  注 册
                </>
              ) : (
                '登 录'
              )}
            </button>
          </form>

          {mode === 'login' && demoAccounts.length > 0 && (
            <div className="mt-6 pt-5 border-t border-surface-100">
              <p className="text-xs text-surface-400 text-center mb-3">演示账号（点击快速填入）</p>
              <div className="grid grid-cols-2 gap-2">
                {demoAccounts.map((account) => (
                  <button
                    key={account.username}
                    type="button"
                    onClick={() => fillDemo(account.username, account.password)}
                    className="btn-secondary text-xs"
                  >
                    {account.description || account.username}
                  </button>
                ))}
              </div>
            </div>
          )}
        </div>

        <p className="text-center text-xs text-surface-400 mt-6">
          WaLiOffice © 2026
        </p>
      </div>
    </div>
  )
}
