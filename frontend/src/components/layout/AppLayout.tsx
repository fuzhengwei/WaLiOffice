import { NavLink, Outlet, useNavigate } from 'react-router-dom'
import { Bell, Files, LogOut, Sparkles, SquareCheckBig } from 'lucide-react'
import { useAuthStore } from '@/stores/auth-store'

const LOGO_URL = '/logo.png'

const NAV_ITEMS = [
  { to: '/', label: '智能助手', icon: Sparkles, end: true },
  { to: '/tasks', label: '任务清单', icon: SquareCheckBig },
  { to: '/files', label: '我的文件', icon: Files },
]

export function AppLayout() {
  const navigate = useNavigate()
  const { user, logout } = useAuthStore()

  const handleLogout = () => {
    logout()
    navigate('/login')
  }

  return (
    <div className="min-h-screen bg-[#f6f4ef] text-surface-950">
      <div className="pointer-events-none fixed inset-0 bg-[radial-gradient(circle_at_15%_12%,rgba(255,255,255,0.92),transparent_32%),radial-gradient(circle_at_78%_8%,rgba(226,232,240,0.72),transparent_30%),linear-gradient(135deg,#f7f2e8_0%,#f3f1eb_45%,#ece7dc_100%)]" />

      <div className="relative z-10 flex min-h-screen flex-col">
        <header className="sticky top-0 z-30 border-b border-black/[0.06] bg-[#f6f4ef]/78 backdrop-blur-2xl">
          <div className="mx-auto flex h-16 w-full max-w-[1440px] items-center justify-between gap-4 px-4 lg:px-6">
            <button
              type="button"
              onClick={() => navigate('/')}
              className="flex items-center gap-3 rounded-full px-1 py-1 text-left transition hover:bg-white/40"
            >
              <div className="flex h-10 w-10 items-center justify-center overflow-hidden rounded-2xl bg-white shadow-sm ring-1 ring-black/[0.06]">
                <img src={LOGO_URL} alt="WaLiOffice logo" className="h-full w-full object-cover" />
              </div>
              <div className="min-w-0">
                <div className="text-sm font-black tracking-tight text-surface-950">WaLiOffice</div>
                <div className="text-[11px] text-surface-500">打开即用，专注办公创作</div>
              </div>
            </button>

            <nav className="hidden items-center gap-2 md:flex">
              {NAV_ITEMS.map((item) => {
                const Icon = item.icon
                return (
                  <NavLink
                    key={item.to}
                    to={item.to}
                    end={item.end}
                    className={({ isActive }) =>
                      `inline-flex items-center gap-2 rounded-full px-4 py-2 text-sm font-medium transition ${
                        isActive
                          ? 'bg-surface-950 text-white shadow-[0_10px_30px_rgba(24,24,27,0.18)]'
                          : 'bg-white/55 text-surface-600 hover:bg-white/80 hover:text-surface-950'
                      }`
                    }
                  >
                    <Icon className="h-4 w-4" />
                    {item.label}
                  </NavLink>
                )
              })}
            </nav>

            <div className="flex items-center gap-2">
              <button
                type="button"
                className="inline-flex h-10 w-10 items-center justify-center rounded-full bg-white/55 text-surface-500 transition hover:bg-white/80 hover:text-surface-950"
                title="通知"
              >
                <Bell className="h-4 w-4" />
              </button>
              <div className="hidden items-center gap-3 rounded-full bg-white/55 px-3 py-1.5 md:flex">
                <div className="flex h-8 w-8 items-center justify-center rounded-full bg-surface-950 text-xs font-bold text-white">
                  {user?.username?.[0]?.toUpperCase() || 'U'}
                </div>
                <div className="min-w-0">
                  <div className="max-w-[140px] truncate text-sm font-semibold text-surface-900">{user?.username}</div>
                  <div className="max-w-[140px] truncate text-[11px] text-surface-500">{user?.email || 'WaLiOffice 用户'}</div>
                </div>
              </div>
              <button
                type="button"
                onClick={handleLogout}
                className="inline-flex h-10 items-center gap-2 rounded-full border border-red-200 bg-red-50 px-4 text-sm font-semibold text-red-600 transition hover:border-red-300 hover:bg-red-100 hover:text-red-700"
                title="退出登录"
              >
                <LogOut className="h-4 w-4" />
                <span>退出登录</span>
              </button>
            </div>
          </div>

          <div className="mx-auto flex max-w-[1440px] gap-2 px-4 pb-3 md:hidden">
            {NAV_ITEMS.map((item) => {
              const Icon = item.icon
              return (
                <NavLink
                  key={item.to}
                  to={item.to}
                  end={item.end}
                  className={({ isActive }) =>
                    `flex min-w-0 flex-1 items-center justify-center gap-2 rounded-full px-3 py-2 text-sm font-medium transition ${
                      isActive
                        ? 'bg-surface-950 text-white'
                        : 'bg-white/55 text-surface-600 hover:bg-white/80 hover:text-surface-950'
                    }`
                  }
                >
                  <Icon className="h-4 w-4" />
                  <span className="truncate">{item.label}</span>
                </NavLink>
              )
            })}
          </div>
        </header>

        <main className="relative z-10 flex-1">
          <div className="mx-auto flex min-h-[calc(100vh-64px)] w-full max-w-[1440px] flex-col px-4 py-4 lg:px-6 lg:py-6">
            <Outlet />
          </div>
        </main>
      </div>
    </div>
  )
}
