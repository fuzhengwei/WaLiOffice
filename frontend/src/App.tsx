import { Routes, Route, Navigate } from 'react-router-dom'
import { useAuthStore } from '@/stores/auth-store'
import Login from '@/pages/Login'
import { AppLayout } from '@/components/layout/AppLayout'
import FilesPage from '@/pages/files/FilesPage'
import Studio from '@/pages/Studio'

function App() {
  const isAuthenticated = useAuthStore((s) => s.isAuthenticated)

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
