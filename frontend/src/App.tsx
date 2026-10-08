import { Routes, Route, Navigate } from 'react-router-dom'
import { useEffect } from 'react'
import { useAuthStore } from '@/stores/auth-store'
import { api } from '@/api'
import Login from '@/pages/Login'
import { AppLayout } from '@/components/layout/AppLayout'
import FilesPage from '@/pages/files/FilesPage'
import Studio from '@/pages/Studio'

function App() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)

  // 服务端关闭登录（AIPPT_AUTH_ENABLED=false）时，/auth/me 无需 token 即返回 guest 用户，
  // 前端探测成功后自动完成"登录"，跳过登录页
  useEffect(() => {
    if (isAuthenticated) return
    api
      .get('/auth/me')
      .then(({ data }) => {
        useAuthStore.getState().login(data.access_token ?? 'guest', data)
      })
      .catch(() => {
        // 服务端仍需登录，保持跳转 /login
      })
  }, [isAuthenticated])

  if (!isAuthenticated) {
    return (
      <Routes>
        <Route path="/login" element={<Login />} />
        <Route path="/*" element={<Navigate to="/login" />} />
      </Routes>
    )
  }

  return (
    <Routes>
      <Route path="/login" element={<Navigate to="/" />} />
      <Route path="/" element={<Studio />} />
      <Route element={<AppLayout />}>
        <Route path="/files" element={<FilesPage />} />
        <Route path="/studio" element={<Navigate to="/" replace />} />
        <Route path="/*" element={<Navigate to="/" />} />
      </Route>
    </Routes>
  )
}

export default App
