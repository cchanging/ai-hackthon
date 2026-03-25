import { createContext, useContext, type Dispatch } from 'react'
import type { AppAction, AppState } from './types'

export interface AppStateContextValue {
  state: AppState
  dispatch: Dispatch<AppAction>
}

export const AppStateContext = createContext<AppStateContextValue | null>(null)

export function useAppState(): AppStateContextValue {
  const context = useContext(AppStateContext)
  if (!context) {
    throw new Error('useAppState must be used inside AppStateProvider')
  }
  return context
}
