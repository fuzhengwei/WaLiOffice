import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuthStore } from '@/stores/auth-store'
import { authApi } from '@/api'
import { Loader2 } from 'lucide-react'

const QR_CODE_URL = 'https://bugstack.cn/images/personal/qrcode.png'
const LOGO_URL = '/logo.png'

export default function Login() {
  const navigate = useNavigate()
  const login = useAuthStore((s) => s.login)
  const [verificationCode, setVerificationCode] = useState('')
  const [agreed, setAgreed] = useState(false)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState('')

  const handleSubmit = async (e?: React.FormEvent) => {
    e?.preventDefault()
    setError('')

    if (!verificationCode.trim()) {
      setError('请填写访问验证码')
      return
    }

    if (!agreed) {
      setError('请先同意用户协议')
      return
    }

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

  return (
    <div className="min-h-screen bg-white px-4 py-8 text-surface-900">
      <main className="mx-auto flex min-h-[calc(100vh-4rem)] w-full max-w-xl flex-col items-center justify-center text-center">
        <div className="mb-5 flex h-16 w-16 items-center justify-center overflow-hidden rounded-2xl bg-white shadow-sm ring-1 ring-black/[0.06]">
          <img src={LOGO_URL} alt="WaLiOffice logo" className="h-full w-full object-cover" />
        </div>

        <h1 className="text-4xl font-extrabold tracking-tight text-surface-900">WaLiOffice</h1>
        <p className="mt-4 text-lg font-semibold text-surface-700">学习AI办公、掌握AI部署、运用AI提效</p>

        <div className="mt-6 rounded-3xl bg-white p-3 shadow-[0_18px_60px_rgba(15,23,42,0.08)]">
          <img
            src={QR_CODE_URL}
            alt="bugstack 虫洞栈公众号二维码"
            className="h-72 w-72 rounded-2xl object-cover"
          />
        </div>

        <p className="mt-5 text-base font-semibold text-surface-700">
          扫码公众号【bugstack虫洞栈】，回复 <span className="rounded bg-red-50 px-1.5 py-0.5 font-extrabold text-red-600">405</span> 获取访问验证码
        </p>

        <form onSubmit={handleSubmit} className="mt-8 w-full max-w-sm space-y-5">
          <input
            className="h-12 w-full rounded-xl border border-surface-200 bg-white px-4 text-center text-base outline-none transition placeholder:text-surface-300 focus:border-primary-500 focus:ring-4 focus:ring-primary-100"
            placeholder="在此处填写访问验证码"
            value={verificationCode}
            onChange={(e) => setVerificationCode(e.target.value)}
            autoComplete="one-time-code"
          />

          {error && (
            <div className="rounded-lg bg-red-50 px-3 py-2 text-sm text-red-500">
              {error}
            </div>
          )}

          <button type="submit" disabled={loading} className="mx-auto flex h-11 min-w-32 items-center justify-center rounded-xl bg-primary-600 px-6 text-base font-semibold text-white shadow-sm transition hover:bg-primary-700 disabled:cursor-not-allowed disabled:opacity-60">
            {loading ? (
              <>
                <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                登录中...
              </>
            ) : (
              '确认登录'
            )}
          </button>

          <label className="flex cursor-pointer items-center justify-center gap-2 text-sm text-surface-600">
            <input
              type="checkbox"
              checked={agreed}
              onChange={(e) => setAgreed(e.target.checked)}
              className="h-4 w-4 rounded border-surface-300 text-primary-600 focus:ring-primary-500"
            />
            <span>同意用户协议</span>
          </label>

          <button type="button" onClick={() => navigate('/')} className="text-sm font-medium text-surface-600 hover:text-primary-600">
            回到首页
          </button>
        </form>

        <p className="mt-12 text-sm font-medium leading-7 text-surface-600">
          说明：此平台主要以学习 OpenAI 为主，请合理、合法、合规的使用相关资料！
          <a href="https://bugstack.cn/" target="_blank" rel="noreferrer" className="ml-1 text-primary-600 underline underline-offset-2">
            查看用户协议
          </a>
        </p>
      </main>
    </div>
  )
}
