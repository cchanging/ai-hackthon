import { AppStateProvider } from './app/app-state'
import { AppShell } from './app/AppShell'

function App() {
  return (
    <AppStateProvider>
      <AppShell />
    </AppStateProvider>
  )
}

export default App
