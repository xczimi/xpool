import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import './index.css'
import App from './App.tsx'
import { GraphqlProvider } from './graphql/GraphqlProvider'
import { AuthProvider } from './auth/AuthContext'
import { I18nProvider } from './i18n/I18nContext'
import { DisplayModeProvider } from './display/DisplayModeProvider'
import { Auth0Gate } from './auth/auth0Provider'
import { ThemeProvider } from './theme/ThemeProvider'

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <Auth0Gate>
      <ThemeProvider>
        <I18nProvider>
          <DisplayModeProvider>
            <AuthProvider>
              <GraphqlProvider>
                <BrowserRouter>
                  <App />
                </BrowserRouter>
              </GraphqlProvider>
            </AuthProvider>
          </DisplayModeProvider>
        </I18nProvider>
      </ThemeProvider>
    </Auth0Gate>
  </StrictMode>,
)
