import { useCallback, useEffect, useMemo, useRef, useState } from 'react'
import { TopBar } from '../components/TopBar'
import { LeftRail } from '../components/LeftRail'
import { GraphCanvas, type GraphCanvasHandle } from '../components/GraphCanvas'
import { InspectorDrawer } from '../components/InspectorDrawer'
import { useAppState } from './app-state-context'
import { appShellTw, inspectorWidthVariants } from '../styles/app-shell.styles'
import { buttonVariants } from '../styles/ui.styles'
import { cn } from '../lib/cn'
import {
  artifactContractSchema,
  formatArtifactContractError,
} from '../lib/artifact-contract'
import {
  buildCallAdjacency,
  buildCallSubgraph,
  buildStructMethodSubgraph,
  buildNodeCrateIndex,
  buildInspectorViewModel,
  buildItemIndex,
  buildWarningIndex,
  computeHighlightSet,
  filterGraphByCrateThenKind,
  withEdgeIds,
} from '../lib/graph'

const EMPTY_VISIBLE_GRAPH = {
  crateFilteredNodes: [],
  crateFilteredNodeIds: new Set<string>(),
  visibleNodes: [],
  visibleEdges: [],
  visibleNodeIds: new Set<string>(),
}

const EMPTY_CALL_SUBGRAPH = {
  nodes: [],
  edges: [],
  nodeIds: new Set<string>(),
  levels: [],
  totalUniqueNodes: 0,
}

const EMPTY_STRUCT_METHOD_SUBGRAPH = {
  nodes: [],
  edges: [],
  nodeIds: new Set<string>(),
  totalMethods: 0,
}

export function AppShell() {
  const { state, dispatch } = useAppState()
  const graphCanvasRef = useRef<GraphCanvasHandle | null>(null)
  const [relationNavHistory, setRelationNavHistory] = useState<string[]>([])
  const pendingFocusNodeIdRef = useRef<string | null>(null)

  const artifact = state.artifact.artifact
  const graphEdges = useMemo(() => (artifact ? withEdgeIds(artifact.graph_index.edges) : []), [artifact])
  const callAdjacency = useMemo(() => buildCallAdjacency(graphEdges), [graphEdges])
  const itemIndex = useMemo(() => (artifact ? buildItemIndex(artifact) : new Map()), [artifact])
  const allNodeIds = useMemo(
    () => new Set((artifact?.graph_index.nodes ?? []).map((node) => node.id)),
    [artifact],
  )
  const nodeById = useMemo(
    () =>
      new Map(
        (artifact?.graph_index.nodes ?? []).map((node) => [node.id, node]),
      ),
    [artifact],
  )
  const nodeKindById = useMemo(
    () =>
      new Map(
        (artifact?.graph_index.nodes ?? []).map((node) => [node.id, node.kind]),
      ),
    [artifact],
  )
  const nodeCrateById = useMemo(
    () =>
      artifact
        ? buildNodeCrateIndex(
            artifact.graph_index.nodes,
            artifact.graph_index.by_container,
          )
        : new Map<string, string>(),
    [artifact],
  )

  const visibleGraph = useMemo(() => {
    if (!artifact) {
      return EMPTY_VISIBLE_GRAPH
    }

    const mainGraphNodes = artifact.graph_index.nodes.filter((node) => node.kind !== 'method')
    const mainGraphNodeIds = new Set(mainGraphNodes.map((node) => node.id))
    const mainGraphEdges = graphEdges.filter(
      (edge) => mainGraphNodeIds.has(edge.from) && mainGraphNodeIds.has(edge.to),
    )

    return filterGraphByCrateThenKind(
      mainGraphNodes,
      mainGraphEdges,
      nodeCrateById,
      state.graphView.crateFilter,
      state.graphView.kindFilter,
    )
  }, [
    artifact,
    graphEdges,
    nodeCrateById,
    state.graphView.crateFilter,
    state.graphView.kindFilter,
  ])
  const structMethodSubgraph = useMemo(() => {
    if (!artifact) {
      return EMPTY_STRUCT_METHOD_SUBGRAPH
    }

    return buildStructMethodSubgraph(
      artifact.graph_index.nodes,
      graphEdges,
      itemIndex,
      state.graphView.structMethodView.structNodeId,
    )
  }, [
    artifact,
    graphEdges,
    itemIndex,
    state.graphView.structMethodView.structNodeId,
  ])
  const callSubgraph = useMemo(() => {
    if (!artifact) {
      return EMPTY_CALL_SUBGRAPH
    }

    return buildCallSubgraph(
      artifact.graph_index.nodes,
      graphEdges,
      callAdjacency,
      state.graphView.callSubgraph.rootNodeId,
      state.graphView.callSubgraph.depth,
    )
  }, [
    artifact,
    callAdjacency,
    graphEdges,
    state.graphView.callSubgraph.depth,
    state.graphView.callSubgraph.rootNodeId,
  ])
  const isStructMethodMode = state.graphView.viewMode === 'struct_methods'
  const isCallSubgraphMode = state.graphView.viewMode === 'call_subgraph'
  const activeNodes = isCallSubgraphMode
    ? callSubgraph.nodes
    : isStructMethodMode
      ? structMethodSubgraph.nodes
      : visibleGraph.visibleNodes
  const activeEdges = isCallSubgraphMode
    ? callSubgraph.edges
    : isStructMethodMode
      ? structMethodSubgraph.edges
      : visibleGraph.visibleEdges
  const activeNodeIds = isCallSubgraphMode
    ? callSubgraph.nodeIds
    : isStructMethodMode
      ? structMethodSubgraph.nodeIds
      : visibleGraph.visibleNodeIds

  const crateItems = useMemo(() => {
    if (!artifact) {
      return []
    }

    const countByCrate = artifact.graph_index.nodes.reduce<Map<string, number>>(
      (acc, node) => {
        const crateName = nodeCrateById.get(node.id) ?? 'unknown'
        acc.set(crateName, (acc.get(crateName) ?? 0) + 1)
        return acc
      },
      new Map(),
    )

    return Object.keys(state.graphView.crateFilter)
      .map((crateName) => ({
        crateName,
        total: countByCrate.get(crateName) ?? 0,
        active: state.graphView.crateFilter[crateName] !== false,
      }))
      .sort((a, b) => {
        if (a.active !== b.active) {
          return a.active ? -1 : 1
        }

        if (a.total !== b.total) {
          return b.total - a.total
        }

        return a.crateName.localeCompare(b.crateName)
      })
  }, [artifact, nodeCrateById, state.graphView.crateFilter])

  const kindItems = useMemo(() => {
    if (!artifact) {
      return []
    }

    const crateScopedKindCount = visibleGraph.crateFilteredNodes.reduce<Map<string, number>>(
      (acc, node) => {
        acc.set(node.kind, (acc.get(node.kind) ?? 0) + 1)
        return acc
      },
      new Map(),
    )

    return Object.keys(state.graphView.kindFilter)
      .filter((kind) => kind !== 'method')
      .map((kind) => ({
        kind,
        total: crateScopedKindCount.get(kind) ?? 0,
        active: state.graphView.kindFilter[kind] !== false,
      }))
      .sort((a, b) => {
        if (a.active !== b.active) {
          return a.active ? -1 : 1
        }

        if (a.total !== b.total) {
          return b.total - a.total
        }

        return a.kind.localeCompare(b.kind)
      })
  }, [artifact, state.graphView.kindFilter, visibleGraph.crateFilteredNodes])

  useEffect(() => {
    const selectedNodeId = state.graphView.selection.nodeId
    if (!selectedNodeId) {
      return
    }

    if (!activeNodeIds.has(selectedNodeId)) {
      dispatch({ type: 'selection/clear' })
    }
  }, [activeNodeIds, dispatch, state.graphView.selection.nodeId])

  useEffect(() => {
    if (!isCallSubgraphMode) {
      return
    }

    const rootNodeId = state.graphView.callSubgraph.rootNodeId
    if (!rootNodeId || !callSubgraph.nodeIds.has(rootNodeId)) {
      dispatch({ type: 'call-view/close' })
    }
  }, [
    callSubgraph.nodeIds,
    dispatch,
    isCallSubgraphMode,
    state.graphView.callSubgraph.rootNodeId,
  ])

  useEffect(() => {
    if (!isStructMethodMode) {
      return
    }

    const structRootNodeId = state.graphView.structMethodView.structNodeId
    if (!structRootNodeId || !structMethodSubgraph.nodeIds.has(structRootNodeId)) {
      dispatch({ type: 'method-view/close' })
    }
  }, [
    dispatch,
    isStructMethodMode,
    state.graphView.structMethodView.structNodeId,
    structMethodSubgraph.nodeIds,
  ])

  useEffect(() => {
    const pendingFocusNodeId = pendingFocusNodeIdRef.current
    if (!pendingFocusNodeId) {
      return
    }

    if (!activeNodeIds.has(pendingFocusNodeId)) {
      return
    }

    graphCanvasRef.current?.focusNode(pendingFocusNodeId)
    pendingFocusNodeIdRef.current = null
  }, [activeNodeIds, state.graphView.selection.nodeId])

  const highlightSet = useMemo(
    () =>
      computeHighlightSet(
        state.graphView.selection.nodeId,
        activeEdges,
        activeNodeIds,
      ),
    [activeEdges, activeNodeIds, state.graphView.selection.nodeId],
  )

  const warningIndex = useMemo(
    () => (artifact ? buildWarningIndex(artifact.warnings) : new Map()),
    [artifact],
  )

  const inspectorViewModel = useMemo(() => {
    if (!artifact) {
      return null
    }

    return buildInspectorViewModel(
      artifact,
      itemIndex,
      warningIndex,
      state.graphView.selection.nodeId,
    )
  }, [artifact, itemIndex, state.graphView.selection.nodeId, warningIndex])

  const inspectorOpen = inspectorViewModel !== null

  const navigateToNode = useCallback(
    (targetNodeId: string, pushCurrentToHistory: boolean) => {
      if (!artifact || !allNodeIds.has(targetNodeId)) {
        return
      }

      if (pushCurrentToHistory) {
        const currentNodeId = state.graphView.selection.nodeId
        if (currentNodeId && currentNodeId !== targetNodeId) {
          setRelationNavHistory((prev) => [...prev, currentNodeId])
        }
      }

      if (isCallSubgraphMode) {
        if (callSubgraph.nodeIds.has(targetNodeId)) {
          dispatch({
            type: 'selection/set',
            payload: {
              nodeId: targetNodeId,
            },
          })
          pendingFocusNodeIdRef.current = targetNodeId
          return
        }

        dispatch({ type: 'call-view/close' })
      }

      if (isStructMethodMode) {
        if (structMethodSubgraph.nodeIds.has(targetNodeId)) {
          dispatch({
            type: 'selection/set',
            payload: {
              nodeId: targetNodeId,
            },
          })
          pendingFocusNodeIdRef.current = targetNodeId
          return
        }

        dispatch({ type: 'method-view/close' })
      }

      const targetKind = nodeKindById.get(targetNodeId)
      const targetCrate = nodeCrateById.get(targetNodeId)
      if (!targetKind || !targetCrate) {
        return
      }

      if (state.graphView.crateFilter[targetCrate] === false) {
        dispatch({
          type: 'filter/set-crate',
          payload: {
            crate: targetCrate,
            enabled: true,
          },
        })
      }

      if (state.graphView.kindFilter[targetKind] === false) {
        dispatch({
          type: 'filter/set-kind',
          payload: {
            kind: targetKind,
            enabled: true,
          },
        })
      }

      dispatch({
        type: 'selection/set',
        payload: {
          nodeId: targetNodeId,
        },
      })
      pendingFocusNodeIdRef.current = targetNodeId
    },
    [
      allNodeIds,
      artifact,
      callSubgraph.nodeIds,
      dispatch,
      isCallSubgraphMode,
      isStructMethodMode,
      nodeCrateById,
      nodeKindById,
      state.graphView.crateFilter,
      state.graphView.kindFilter,
      state.graphView.selection.nodeId,
      structMethodSubgraph.nodeIds,
    ],
  )

  const handleNavigateRelation = useCallback(
    (targetNodeId: string) => {
      navigateToNode(targetNodeId, true)
    },
    [navigateToNode],
  )

  const handleNavigateBack = useCallback(() => {
    if (relationNavHistory.length === 0) {
      return
    }

    const previousNodeId = relationNavHistory[relationNavHistory.length - 1]
    setRelationNavHistory((prev) => prev.slice(0, -1))
    navigateToNode(previousNodeId, false)
  }, [navigateToNode, relationNavHistory])

  const handleCloseInspector = useCallback(() => {
    setRelationNavHistory([])
    pendingFocusNodeIdRef.current = null
    dispatch({ type: 'selection/clear' })
  }, [dispatch])

  const handleGraphSelection = useCallback(
    (nodeId: string | null) => {
      setRelationNavHistory([])
      pendingFocusNodeIdRef.current = null
      dispatch({
        type: 'selection/set',
        payload: {
          nodeId,
        },
      })
    },
    [dispatch],
  )

  const handleOpenCallGraph = useCallback(
    (rootNodeId: string) => {
      if (!allNodeIds.has(rootNodeId)) {
        return
      }
      const rootKind = nodeKindById.get(rootNodeId)
      if (rootKind !== 'fn' && rootKind !== 'method') {
        return
      }

      setRelationNavHistory([])
      dispatch({
        type: 'call-view/open-from-method',
        payload: {
          rootNodeId,
          returnMode: state.graphView.viewMode === 'struct_methods' ? 'struct_methods' : 'main',
        },
      })
      pendingFocusNodeIdRef.current = rootNodeId
    },
    [allNodeIds, dispatch, nodeKindById, state.graphView.viewMode],
  )

  const handleCloseCallGraph = useCallback(() => {
    dispatch({ type: 'call-view/close' })
  }, [dispatch])

  const handleOpenStructMethodView = useCallback(
    (structNodeId: string, focusNodeId?: string) => {
      const structKind = nodeKindById.get(structNodeId)
      if (structKind !== 'struct') {
        return
      }

      const resolvedFocusNodeId =
        focusNodeId && allNodeIds.has(focusNodeId) ? focusNodeId : structNodeId

      setRelationNavHistory([])
      pendingFocusNodeIdRef.current = resolvedFocusNodeId
      dispatch({
        type: 'method-view/open-for-struct',
        payload: {
          structNodeId,
          focusNodeId: resolvedFocusNodeId,
        },
      })
    },
    [allNodeIds, dispatch, nodeKindById],
  )

  const handleCloseStructMethodView = useCallback(() => {
    dispatch({ type: 'method-view/close' })
  }, [dispatch])

  const handleCallGraphDepthChange = useCallback(
    (value: string) => {
      const parsed = Number.parseInt(value, 10)
      if (Number.isNaN(parsed)) {
        return
      }

      dispatch({
        type: 'call-view/set-depth',
        payload: {
          depth: parsed,
        },
      })
    },
    [dispatch],
  )

  const handleUploadFile = useCallback(
    async (file: File) => {
      try {
        const text = await file.text()
        const unknownJson = JSON.parse(text) as unknown
        const parsed = artifactContractSchema.safeParse(unknownJson)

        if (!parsed.success) {
          setRelationNavHistory([])
          pendingFocusNodeIdRef.current = null
          dispatch({
            type: 'artifact/failed',
            payload: {
              message: formatArtifactContractError(parsed.error),
            },
          })
          return
        }

        dispatch({
          type: 'artifact/loaded',
          payload: {
            artifact: parsed.data,
            fileName: file.name,
          },
        })
        setRelationNavHistory([])
        pendingFocusNodeIdRef.current = null
      } catch (error) {
        const message =
          error instanceof Error
            ? `Invalid JSON: ${error.message}`
            : 'Invalid JSON: unable to parse file content.'

        setRelationNavHistory([])
        pendingFocusNodeIdRef.current = null
        dispatch({
          type: 'artifact/failed',
          payload: {
            message,
          },
        })
      }
    },
    [dispatch],
  )

  return (
    <div className={appShellTw.root}>
      <TopBar
        theme={state.ui.theme}
        hasArtifact={artifact !== null}
        hasSelection={state.graphView.selection.nodeId !== null}
        onToggleTheme={() => {
          dispatch({ type: 'ui/theme-toggle' })
        }}
        onUploadFile={handleUploadFile}
        onFitView={() => {
          graphCanvasRef.current?.fitView()
        }}
        onResetLayout={() => {
          graphCanvasRef.current?.resetLayout()
        }}
        onClearSelection={handleCloseInspector}
      />

      <main className={appShellTw.main}>
        <div className={appShellTw.layoutBase}>
          <LeftRail
            fileName={state.artifact.fileName}
            nodeCount={visibleGraph.visibleNodes.length}
            crateVisibleNodeCount={visibleGraph.crateFilteredNodes.length}
            edgeCount={visibleGraph.visibleEdges.length}
            warningCount={artifact?.warnings.length ?? 0}
            crateItems={crateItems}
            onToggleCrate={(crateName) => {
              dispatch({
                type: 'filter/toggle-crate',
                payload: {
                  crate: crateName,
                },
              })
            }}
            onSetAllCrates={(enabled) => {
              dispatch({
                type: 'filter/set-all-crates',
                payload: {
                  enabled,
                },
              })
            }}
            onInvertCrates={() => {
              dispatch({
                type: 'filter/invert-crates',
              })
            }}
            onIsolateCrate={(crateName) => {
              dispatch({
                type: 'filter/isolate-crate',
                payload: {
                  crate: crateName,
                },
              })
            }}
            kindItems={kindItems}
            onToggleKind={(kind) => {
              dispatch({
                type: 'filter/toggle-kind',
                payload: {
                  kind,
                },
              })
            }}
            onSetAllKinds={(enabled) => {
              dispatch({
                type: 'filter/set-all',
                payload: {
                  enabled,
                },
              })
            }}
            onInvertKinds={() => {
              dispatch({
                type: 'filter/invert',
              })
            }}
            onIsolateKind={(kind) => {
              dispatch({
                type: 'filter/isolate-kind',
                payload: {
                  kind,
                },
              })
            }}
            contractError={state.artifact.errorMessage}
            warnings={(artifact?.warnings ?? []).map(
              (warning) => `[${warning.severity}] ${warning.message}`,
            )}
          />

          <div className="min-h-0 min-w-0 flex flex-1 flex-col">
            {isCallSubgraphMode ? (
              <section className="mb-3 shrink-0 flex items-center justify-between gap-3 rounded-[var(--rm-radius-md)] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))] px-3 py-2.5">
                <div className="min-w-0">
                  <p className="truncate text-[0.78rem] font-semibold text-[hsl(var(--rm-text))]">
                    Call Subgraph
                  </p>
                  <p className="truncate text-[0.7rem] text-[hsl(var(--rm-text-secondary))]">
                    Root: {state.graphView.callSubgraph.rootNodeId ?? 'unknown'} · Reachable:{' '}
                    {callSubgraph.totalUniqueNodes}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <label
                    htmlFor="call-subgraph-depth"
                    className="text-[0.68rem] uppercase tracking-[0.08em] text-[hsl(var(--rm-text-tertiary))]"
                  >
                    Depth
                  </label>
                  <input
                    id="call-subgraph-depth"
                    type="number"
                    min={1}
                    max={6}
                    value={state.graphView.callSubgraph.depth}
                    onChange={(event) => {
                      handleCallGraphDepthChange(event.target.value)
                    }}
                    className="h-8 w-16 rounded-[8px] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface-muted))] px-2 text-[0.72rem] font-semibold text-[hsl(var(--rm-text))] outline-none focus:border-[hsl(var(--rm-accent))]"
                  />
                  <button
                    type="button"
                    className={cn(buttonVariants({ variant: 'surface' }), 'h-8 px-2.5 py-1')}
                    onClick={handleCloseCallGraph}
                  >
                    Back to Main
                  </button>
                </div>
              </section>
            ) : isStructMethodMode ? (
              <section className="mb-3 shrink-0 flex items-center justify-between gap-3 rounded-[var(--rm-radius-md)] border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface))] px-3 py-2.5">
                <div className="min-w-0">
                  <p className="truncate text-[0.78rem] font-semibold text-[hsl(var(--rm-text))]">
                    Struct Methods
                  </p>
                  <p className="truncate text-[0.7rem] text-[hsl(var(--rm-text-secondary))]">
                    Struct: {state.graphView.structMethodView.structNodeId ?? 'unknown'} · Methods:{' '}
                    {structMethodSubgraph.totalMethods}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    className={cn(buttonVariants({ variant: 'surface' }), 'h-8 px-2.5 py-1')}
                    onClick={handleCloseStructMethodView}
                  >
                    Back to Main
                  </button>
                </div>
              </section>
            ) : null}

            <GraphCanvas
              ref={graphCanvasRef}
              hasArtifact={artifact !== null}
              nodes={activeNodes}
              edges={activeEdges}
              selection={state.graphView.selection}
              highlightSet={highlightSet}
              onSelectNode={handleGraphSelection}
            />
          </div>

          <div className={inspectorWidthVariants({ open: inspectorOpen })}>
            <InspectorDrawer
              open={inspectorOpen}
              viewModel={inspectorViewModel}
              canGoBack={relationNavHistory.length > 0}
              onGoBack={handleNavigateBack}
              onNavigateRelation={handleNavigateRelation}
              nodeById={nodeById}
              callViewMode={state.graphView.viewMode}
              callViewRootNodeId={state.graphView.callSubgraph.rootNodeId}
              onOpenCallGraph={handleOpenCallGraph}
              onOpenStructMethodView={handleOpenStructMethodView}
              onClose={handleCloseInspector}
            />
          </div>
        </div>
      </main>
    </div>
  )
}
