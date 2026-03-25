import type {
  ArtifactContract,
  GraphEdge,
  GraphNode,
  RustItem,
  WarningItem,
} from '../lib/artifact-contract'

export type ThemeMode = 'light' | 'dark'

export interface NodeSelection {
  nodeId: string | null
}

export type KindFilterState = Record<string, boolean>
export type CrateFilterState = Record<string, boolean>
export type GraphViewMode = 'main' | 'struct_methods' | 'call_subgraph'

export type CallSubgraphReturnMode = 'main' | 'struct_methods'

export interface CallSubgraphState {
  rootNodeId: string | null
  depth: number
  returnMode: CallSubgraphReturnMode
}

export interface StructMethodViewState {
  structNodeId: string | null
}

export interface HighlightSet {
  nodeIds: Set<string>
  edgeIds: Set<string>
}

export interface ArtifactState {
  status: 'idle' | 'ready' | 'error'
  artifact: ArtifactContract | null
  fileName: string | null
  errorMessage: string | null
}

export interface GraphViewState {
  viewMode: GraphViewMode
  selection: NodeSelection
  crateFilter: CrateFilterState
  kindFilter: KindFilterState
  structMethodView: StructMethodViewState
  callSubgraph: CallSubgraphState
}

export interface UiState {
  theme: ThemeMode
}

export interface AppState {
  artifact: ArtifactState
  graphView: GraphViewState
  ui: UiState
}

export type AppAction =
  | {
      type: 'artifact/loaded'
      payload: {
        artifact: ArtifactContract
        fileName: string
      }
    }
  | {
      type: 'artifact/failed'
      payload: {
        message: string
      }
    }
  | {
      type: 'selection/set'
      payload: {
        nodeId: string | null
      }
    }
  | {
      type: 'selection/clear'
    }
  | {
      type: 'filter/toggle-kind'
      payload: {
        kind: string
      }
    }
  | {
      type: 'filter/toggle-crate'
      payload: {
        crate: string
      }
    }
  | {
      type: 'filter/set-all'
      payload: {
        enabled: boolean
      }
    }
  | {
      type: 'filter/set-all-crates'
      payload: {
        enabled: boolean
      }
    }
  | {
      type: 'filter/set-kind'
      payload: {
        kind: string
        enabled: boolean
      }
    }
  | {
      type: 'filter/set-crate'
      payload: {
        crate: string
        enabled: boolean
      }
    }
  | {
      type: 'filter/invert'
    }
  | {
      type: 'filter/invert-crates'
    }
  | {
      type: 'filter/isolate-kind'
      payload: {
        kind: string
      }
    }
  | {
      type: 'filter/isolate-crate'
      payload: {
        crate: string
      }
    }
  | {
      type: 'method-view/open-for-struct'
      payload: {
        structNodeId: string
        focusNodeId?: string
      }
    }
  | {
      type: 'method-view/close'
    }
  | {
      type: 'call-view/open-from-method'
      payload: {
        rootNodeId: string
        returnMode: CallSubgraphReturnMode
      }
    }
  | {
      type: 'call-view/set-depth'
      payload: {
        depth: number
      }
    }
  | {
      type: 'call-view/close'
    }
  | {
      type: 'ui/theme-set'
      payload: {
        theme: ThemeMode
      }
    }
  | {
      type: 'ui/theme-toggle'
    }

export interface RelationSummaryItem {
  edgeKind: GraphEdge['kind']
  relationLabel: string
  sourceContext: string
  peerId: string
  peerKind: string
}

export interface InspectorViewModel {
  node: GraphNode
  item: RustItem | null
  location: {
    file: string
    line: number
    col: number
  } | null
  relations: RelationSummaryItem[]
  relationCountByLabel: Record<string, number>
  warnings: WarningItem[]
}
