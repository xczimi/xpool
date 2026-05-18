import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import './index.css'
import App from './App.tsx'
import { GraphqlProvider } from './graphql/GraphqlProvider'
import { AuthProvider } from './auth/AuthContext'
import { I18nProvider } from './i18n/I18nContext'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <I18nProvider>
      <AuthProvider>
        <GraphqlProvider>
          <BrowserRouter>
            <App />
          </BrowserRouter>
        </GraphqlProvider>
      </AuthProvider>
    </I18nProvider>
  </StrictMode>,
)
