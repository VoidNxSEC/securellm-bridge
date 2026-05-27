import { BrowserRouter, Routes, Route } from 'react-router-dom'
import { Toaster } from '@/components/ui/toaster'
import { Layout } from '@/components/dashboard/Layout'
import { Dashboard } from '@/pages/Dashboard'
import { Gateway } from '@/pages/Gateway'
import { OpenCore } from '@/pages/OpenCore'
import { Security } from '@/pages/Security'
import { Projects } from '@/pages/Projects'
import { Intelligence } from '@/pages/Intelligence'
import { Briefing } from '@/pages/Briefing'
import { Settings } from '@/pages/Settings'

function App() {
  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<OpenCore />} />
          <Route path="/gateway" element={<Gateway />} />
          <Route path="/security" element={<Security />} />
          <Route path="/ecosystem" element={<Dashboard />} />
          <Route path="/projects" element={<Projects />} />
          <Route path="/projects/:projectName" element={<Projects />} />
          <Route path="/intelligence" element={<Intelligence />} />
          <Route path="/briefing" element={<Briefing />} />
          <Route path="/settings" element={<Settings />} />
        </Routes>
      </Layout>
      <Toaster />
    </BrowserRouter>
  )
}

export default App
