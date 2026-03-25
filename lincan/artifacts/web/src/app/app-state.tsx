import {
  type PropsWithChildren,
  useEffect,
  useMemo,
  useReducer,
} from 'react'
import type {
  AppAction,
  AppState,
  CrateFilterState,
  KindFilterState,
  ThemeMode,
} from './types'
import { applyTheme, persistTheme, resolveInitialTheme } from '../lib/theme'
import { AppStateContext } from './app-state-context'

const CALL_SUBGRAPH_DEPTH_MIN = 1
const CALL_SUBGRAPH_DEPTH_MAX = 6
const CALL_SUBGRAPH_DEPTH_DEFAULT = 3

function buildKindFilter(kinds: string[]): KindFilterState {
  return kinds.reduce<KindFilterState>((acc, kind) => {
    acc[kind] = true
    return acc
  }, {})
}

function buildCrateFilter(crates: string[]): CrateFilterState {
  return crates.reduce<CrateFilterState>((acc, crateName) => {
    acc[crateName] = true
    return acc
  }, {})
}

function crateNameFromContainerPath(containerPath: string): string | null {
  const [crateName] = containerPath.split('::')
  return crateName && crateName.length > 0 ? crateName : null
}

function crateNameFromNodeId(nodeId: string): string | null {
  return crateNameFromContainerPath(nodeId)
}

function createInitialState(): AppState {
  return {
    artifact: {
      status: 'idle',
      artifact: null,
      fileName: null,
      errorMessage: null,
    },
    graphView: {
      viewMode: 'main',
      selection: {
        nodeId: null,
      },
      crateFilter: {},
      kindFilter: {},
      structMethodView: {
        structNodeId: null,
      },
      callSubgraph: {
        rootNodeId: null,
        depth: CALL_SUBGRAPH_DEPTH_DEFAULT,
        returnMode: 'main',
      },
    },
    ui: {
      theme: resolveInitialTheme(),
    },
  }
}

function toggleTheme(theme: ThemeMode): ThemeMode {
  return theme === 'light' ? 'dark' : 'light'
}

function clampCallSubgraphDepth(depth: number): number {
  if (!Number.isFinite(depth)) {
    return CALL_SUBGRAPH_DEPTH_DEFAULT
  }

  return Math.max(
    CALL_SUBGRAPH_DEPTH_MIN,
    Math.min(CALL_SUBGRAPH_DEPTH_MAX, Math.floor(depth)),
  )
}

function appReducer(state: AppState, action: AppAction): AppState {
  switch (action.type) {
    case 'artifact/loaded': {
      const allKinds = Array.from(
        new Set(action.payload.artifact.graph_index.nodes.map((node) => node.kind)),
      ).sort((a, b) => a.localeCompare(b))
      const crateNames = action.payload.artifact.crates
        .map((crateItem) => crateItem.name)
        .filter((name): name is string => typeof name === 'string' && name.length > 0)
      const containerCrates = Object.keys(action.payload.artifact.graph_index.by_container)
        .map(crateNameFromContainerPath)
        .filter((name): name is string => name !== null)
      const nodeIdCrates = action.payload.artifact.graph_index.nodes
        .map((node) => crateNameFromNodeId(node.id))
        .filter((name): name is string => name !== null)
      const allCrates = Array.from(
        new Set([...crateNames, ...containerCrates, ...nodeIdCrates]),
      ).sort((a, b) => a.localeCompare(b))

      return {
        ...state,
        artifact: {
          status: 'ready',
          artifact: action.payload.artifact,
          fileName: action.payload.fileName,
          errorMessage: null,
        },
        graphView: {
          viewMode: 'main',
          selection: {
            nodeId: null,
          },
          crateFilter: buildCrateFilter(allCrates),
          kindFilter: buildKindFilter(allKinds),
          structMethodView: {
            structNodeId: null,
          },
          callSubgraph: {
            rootNodeId: null,
            depth: CALL_SUBGRAPH_DEPTH_DEFAULT,
            returnMode: 'main',
          },
        },
      }
    }

    case 'artifact/failed': {
      return {
        ...state,
        artifact: {
          status: 'error',
          artifact: null,
          fileName: null,
          errorMessage: action.payload.message,
        },
        graphView: {
          viewMode: 'main',
          selection: {
            nodeId: null,
          },
          crateFilter: {},
          kindFilter: {},
          structMethodView: {
            structNodeId: null,
          },
          callSubgraph: {
            rootNodeId: null,
            depth: CALL_SUBGRAPH_DEPTH_DEFAULT,
            returnMode: 'main',
          },
        },
      }
    }

    case 'selection/set': {
      return {
        ...state,
        graphView: {
          ...state.graphView,
          selection: {
            nodeId: action.payload.nodeId,
          },
        },
      }
    }

    case 'selection/clear': {
      return {
        ...state,
        graphView: {
          ...state.graphView,
          selection: {
            nodeId: null,
          },
        },
      }
    }

    case 'filter/toggle-kind': {
      const previous = state.graphView.kindFilter[action.payload.kind]
      return {
        ...state,
        graphView: {
          ...state.graphView,
          kindFilter: {
            ...state.graphView.kindFilter,
            [action.payload.kind]: !previous,
          },
        },
      }
    }

    case 'filter/toggle-crate': {
      const previous = state.graphView.crateFilter[action.payload.crate]
      return {
        ...state,
        graphView: {
          ...state.graphView,
          crateFilter: {
            ...state.graphView.crateFilter,
            [action.payload.crate]: !previous,
          },
        },
      }
    }

    case 'filter/set-all': {
      const nextFilter = Object.keys(state.graphView.kindFilter).reduce<KindFilterState>(
        (acc, kind) => {
          acc[kind] = action.payload.enabled
          return acc
        },
        {},
      )

      return {
        ...state,
        graphView: {
          ...state.graphView,
          kindFilter: nextFilter,
        },
      }
    }

    case 'filter/set-all-crates': {
      const nextFilter = Object.keys(state.graphView.crateFilter).reduce<CrateFilterState>(
        (acc, crateName) => {
          acc[crateName] = action.payload.enabled
          return acc
        },
        {},
      )

      return {
        ...state,
        graphView: {
          ...state.graphView,
          crateFilter: nextFilter,
        },
      }
    }

    case 'filter/set-kind': {
      if (!(action.payload.kind in state.graphView.kindFilter)) {
        return state
      }

      return {
        ...state,
        graphView: {
          ...state.graphView,
          kindFilter: {
            ...state.graphView.kindFilter,
            [action.payload.kind]: action.payload.enabled,
          },
        },
      }
    }

    case 'filter/set-crate': {
      if (!(action.payload.crate in state.graphView.crateFilter)) {
        return state
      }

      return {
        ...state,
        graphView: {
          ...state.graphView,
          crateFilter: {
            ...state.graphView.crateFilter,
            [action.payload.crate]: action.payload.enabled,
          },
        },
      }
    }

    case 'filter/invert': {
      const nextFilter = Object.keys(state.graphView.kindFilter).reduce<KindFilterState>(
        (acc, kind) => {
          acc[kind] = !state.graphView.kindFilter[kind]
          return acc
        },
        {},
      )

      return {
        ...state,
        graphView: {
          ...state.graphView,
          kindFilter: nextFilter,
        },
      }
    }

    case 'filter/invert-crates': {
      const nextFilter = Object.keys(state.graphView.crateFilter).reduce<CrateFilterState>(
        (acc, crateName) => {
          acc[crateName] = !state.graphView.crateFilter[crateName]
          return acc
        },
        {},
      )

      return {
        ...state,
        graphView: {
          ...state.graphView,
          crateFilter: nextFilter,
        },
      }
    }

    case 'filter/isolate-kind': {
      if (!(action.payload.kind in state.graphView.kindFilter)) {
        return state
      }

      const nextFilter = Object.keys(state.graphView.kindFilter).reduce<KindFilterState>(
        (acc, kind) => {
          acc[kind] = kind === action.payload.kind
          return acc
        },
        {},
      )

      return {
        ...state,
        graphView: {
          ...state.graphView,
          kindFilter: nextFilter,
        },
      }
    }

    case 'filter/isolate-crate': {
      if (!(action.payload.crate in state.graphView.crateFilter)) {
        return state
      }

      const nextFilter = Object.keys(state.graphView.crateFilter).reduce<CrateFilterState>(
        (acc, crateName) => {
          acc[crateName] = crateName === action.payload.crate
          return acc
        },
        {},
      )

      return {
        ...state,
        graphView: {
          ...state.graphView,
          crateFilter: nextFilter,
        },
      }
    }

    case 'method-view/open-for-struct': {
      const nextSelectionNodeId =
        action.payload.focusNodeId ?? action.payload.structNodeId

      return {
        ...state,
        graphView: {
          ...state.graphView,
          viewMode: 'struct_methods',
          selection: {
            nodeId: nextSelectionNodeId,
          },
          structMethodView: {
            structNodeId: action.payload.structNodeId,
          },
        },
      }
    }

    case 'method-view/close': {
      return {
        ...state,
        graphView: {
          ...state.graphView,
          viewMode: 'main',
          structMethodView: {
            structNodeId: null,
          },
        },
      }
    }

    case 'call-view/open-from-method': {
      return {
        ...state,
        graphView: {
          ...state.graphView,
          viewMode: 'call_subgraph',
          selection: {
            nodeId: action.payload.rootNodeId,
          },
          callSubgraph: {
            rootNodeId: action.payload.rootNodeId,
            depth: CALL_SUBGRAPH_DEPTH_DEFAULT,
            returnMode: action.payload.returnMode,
          },
        },
      }
    }

    case 'call-view/set-depth': {
      return {
        ...state,
        graphView: {
          ...state.graphView,
          callSubgraph: {
            ...state.graphView.callSubgraph,
            depth: clampCallSubgraphDepth(action.payload.depth),
          },
        },
      }
    }

    case 'call-view/close': {
      return {
        ...state,
        graphView: {
          ...state.graphView,
          viewMode: state.graphView.callSubgraph.returnMode,
          callSubgraph: {
            ...state.graphView.callSubgraph,
            rootNodeId: null,
            returnMode: 'main',
          },
        },
      }
    }

    case 'ui/theme-set': {
      return {
        ...state,
        ui: {
          ...state.ui,
          theme: action.payload.theme,
        },
      }
    }

    case 'ui/theme-toggle': {
      return {
        ...state,
        ui: {
          ...state.ui,
          theme: toggleTheme(state.ui.theme),
        },
      }
    }

    default: {
      return state
    }
  }
}

export function AppStateProvider({ children }: PropsWithChildren) {
  const [state, dispatch] = useReducer(appReducer, undefined, createInitialState)

  useEffect(() => {
    applyTheme(state.ui.theme)
    persistTheme(state.ui.theme)
  }, [state.ui.theme])

  const value = useMemo(
    () => ({
      state,
      dispatch,
    }),
    [state],
  )

  return <AppStateContext.Provider value={value}>{children}</AppStateContext.Provider>
}
