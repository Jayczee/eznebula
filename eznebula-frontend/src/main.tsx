import { StrictMode, Component, type ReactNode } from "react"
import { createRoot } from "react-dom/client"
import "./index.css"
import App from "./App.tsx"
import { ThemeProvider } from "@/components/theme-provider.tsx"
import { Toaster } from "@/components/ui/sonner"
import LogsWindow from "./components/logs-window.tsx"
import PeersWindow from "./components/peers-window.tsx"
import ServersWindow from "./components/servers-window.tsx"
import SettingsWindow from "./components/settings-window.tsx"

document.addEventListener("contextmenu", (e) => e.preventDefault())
document.addEventListener("keydown", (e) => {
  if (e.key === "F5" || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "r"))
    e.preventDefault()
})

// Error boundary to catch crashes in sub-windows
class ErrorBoundary extends Component<{ children: ReactNode }, { err: string | null }> {
  state = { err: null as string | null }
  static getDerivedStateFromError(e: Error) { return { err: e.message } }
  render() {
    if (this.state.err) return <main className="h-screen bg-background flex items-center justify-center"><p className="text-xs text-red-500">{this.state.err}</p></main>
    return this.props.children
  }
}

function WindowRouter() {
  const params = new URLSearchParams(window.location.search)
  const view = params.get("view")
  return (
    <ErrorBoundary>
      {view === "peers" ? <PeersWindow /> :
       view === "logs" ? <LogsWindow /> :
       view === "servers" ? <ServersWindow /> :
       view === "settings" ? <SettingsWindow /> :
       <App />}
    </ErrorBoundary>
  )
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ThemeProvider>
      <WindowRouter />
      <Toaster position="top-center" richColors />
    </ThemeProvider>
  </StrictMode>,
)
