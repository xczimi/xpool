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
import { NeedsInvite } from './components/NeedsInvite'
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
        {/* Public recipient-side entry: paste an invite code/link → routes to
            the claim page below. Sharing lives on Pools; see
            .scratch/merge-pools-invite-pages/PRD.md. */}
        <Route path="invite" element={<NeedsInvite />} />
        <Route path="invite/:code" element={<InviteClaimPage />} />
        <Route path="rules" element={<RulesPage />} />
        <Route path="admin/*" element={<AdminPage />} />
        <Route path="*" element={<Navigate to="/" replace />} />
      </Route>
    </Routes>
  )
}

export default App
