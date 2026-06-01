import { Navigate, Route, Routes } from 'react-router-dom'
import { Layout } from './components/Layout'
import { HomePage } from './pages/HomePage'
import { TodayPage } from './pages/TodayPage'
import { SchedulePage } from './pages/SchedulePage'
import { MyTipsPage } from './pages/MyTipsPage'
import { AllTipsPage } from './pages/AllTipsPage'
import { ScoreboardPage } from './pages/ScoreboardPage'
import { PerfectPage } from './pages/PerfectPage'
import { PoolsPage } from './pages/PoolsPage'
import { ProfilePage } from './pages/ProfilePage'
import { InviteClaimPage } from './pages/InviteClaimPage'
import { InvitePage } from './pages/InvitePage'
import { RulesPage } from './pages/RulesPage'
import { AdminPage } from './pages/AdminPage'

export function App() {
  return (
    <Routes>
      <Route element={<Layout />}>
        <Route index element={<HomePage />} />
        <Route path="today" element={<TodayPage />} />
        <Route path="games" element={<SchedulePage />} />
        <Route path="mytips" element={<MyTipsPage />} />
        <Route path="alltips" element={<AllTipsPage />} />
        <Route path="scoreboard" element={<ScoreboardPage />} />
        <Route path="perfect" element={<PerfectPage />} />
        <Route path="pools" element={<PoolsPage />} />
        <Route path="profile" element={<ProfilePage />} />
        <Route path="invite" element={<InvitePage />} />
        <Route path="invite/:code" element={<InviteClaimPage />} />
        <Route path="rules" element={<RulesPage />} />
        <Route path="admin/*" element={<AdminPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  )
}

export default App
