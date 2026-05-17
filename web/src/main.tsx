import { StrictMode } from 'react'
import { createRoot } from 'react-dom/client'
import { BrowserRouter } from 'react-router-dom'
import { Provider as UrqlProvider } from 'urql'
import './index.css'
import App from './App.tsx'
import { createGraphqlClient } from './graphql/client'
import { AuthProvider } from './auth/AuthContext'
import { I18nProvider } from './i18n/I18nContext'

const client = createGraphqlClient()

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <I18nProvider>
      <AuthProvider>
        <UrqlProvider value={client}>
          <BrowserRouter>
            <App />
          </BrowserRouter>
        </UrqlProvider>
      </AuthProvider>
    </I18nProvider>
  </StrictMode>,
)
