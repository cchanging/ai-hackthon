import type {
  ArtifactContract,
  GraphEdge,
  GraphNode,
  RustItem,
  WarningItem,
} from './artifact-contract'
import type {
  CrateFilterState,
  HighlightSet,
  InspectorViewModel,
  KindFilterState,
  RelationSummaryItem,
} from '../app/types'

export interface GraphEdgeWithId extends GraphEdge {
  id: string
}

export type CallAdjacency = Map<string, string[]>

export interface CallTraceLevel {
  depth: number
  nodeIds: string[]
}

export interface CallTraceResult {
  levels: CallTraceLevel[]
  totalUniqueNodes: number
}

export interface CallSubgraphResult {
  nodes: GraphNode[]
  edges: GraphEdgeWithId[]
  nodeIds: Set<string>
  levels: CallTraceLevel[]
  totalUniqueNodes: number
}

export interface StructMethodSubgraphResult {
  nodes: GraphNode[]
  edges: GraphEdgeWithId[]
  nodeIds: Set<string>
  totalMethods: number
}

export function createGraphEdgeId(edge: GraphEdge, index: number): string {
  return `${edge.kind}:${edge.from}->${edge.to}:${index}`
}

export function withEdgeIds(edges: GraphEdge[]): GraphEdgeWithId[] {
  return edges.map((edge, index) => ({
    ...edge,
    id: createGraphEdgeId(edge, index),
  }))
}

export function buildCallAdjacency(
  edges: Array<Pick<GraphEdge, 'kind' | 'from' | 'to'>>,
): CallAdjacency {
  const adjacencySet = new Map<string, Set<string>>()

  for (const edge of edges) {
    if (edge.kind !== 'call') {
      continue
    }

    let peers = adjacencySet.get(edge.from)
    if (!peers) {
      peers = new Set<string>()
      adjacencySet.set(edge.from, peers)
    }
    peers.add(edge.to)
  }

  return new Map(
    Array.from(adjacencySet.entries()).map(([from, peers]) => [
      from,
      Array.from(peers),
    ]),
  )
}

export function traceDownstreamCalls(
  adjacency: CallAdjacency,
  startId: string,
  maxDepth: number,
): CallTraceResult {
  const depthCap = Number.isFinite(maxDepth)
    ? Math.max(1, Math.min(6, Math.floor(maxDepth)))
    : 1

  const visited = new Set<string>([startId])
  const levels: CallTraceLevel[] = []

  let frontier = [startId]

  for (let depth = 1; depth <= depthCap; depth += 1) {
    const nextLevel: string[] = []

    for (const nodeId of frontier) {
      const peers = adjacency.get(nodeId)
      if (!peers) {
        continue
      }

      for (const peerId of peers) {
        if (visited.has(peerId)) {
          continue
        }
        visited.add(peerId)
        nextLevel.push(peerId)
      }
    }

    if (nextLevel.length === 0) {
      break
    }

    levels.push({
      depth,
      nodeIds: nextLevel,
    })
    frontier = nextLevel
  }

  return {
    levels,
    totalUniqueNodes: visited.size - 1,
  }
}

export function buildCallSubgraph(
  graphNodes: GraphNode[],
  graphEdges: GraphEdgeWithId[],
  adjacency: CallAdjacency,
  rootNodeId: string | null,
  depth: number,
): CallSubgraphResult {
  if (!rootNodeId) {
    return {
      nodes: [],
      edges: [],
      nodeIds: new Set<string>(),
      levels: [],
      totalUniqueNodes: 0,
    }
  }

  const rootExists = graphNodes.some((node) => node.id === rootNodeId)
  if (!rootExists) {
    return {
      nodes: [],
      edges: [],
      nodeIds: new Set<string>(),
      levels: [],
      totalUniqueNodes: 0,
    }
  }

  const trace = traceDownstreamCalls(adjacency, rootNodeId, depth)
  const nodeIds = new Set<string>([rootNodeId])

  for (const level of trace.levels) {
    for (const nodeId of level.nodeIds) {
      nodeIds.add(nodeId)
    }
  }

  return {
    nodes: graphNodes.filter((node) => nodeIds.has(node.id)),
    edges: graphEdges.filter(
      (edge) =>
        edge.kind === 'call' && nodeIds.has(edge.from) && nodeIds.has(edge.to),
    ),
    nodeIds,
    levels: trace.levels,
    totalUniqueNodes: trace.totalUniqueNodes,
  }
}

export function buildStructMethodSubgraph(
  graphNodes: GraphNode[],
  graphEdges: GraphEdgeWithId[],
  itemIndex: Map<string, RustItem>,
  structNodeId: string | null,
): StructMethodSubgraphResult {
  if (!structNodeId) {
    return {
      nodes: [],
      edges: [],
      nodeIds: new Set<string>(),
      totalMethods: 0,
    }
  }

  const rootNode = graphNodes.find((node) => node.id === structNodeId)
  if (!rootNode || rootNode.kind !== 'struct') {
    return {
      nodes: [],
      edges: [],
      nodeIds: new Set<string>(),
      totalMethods: 0,
    }
  }

  const methodNodeIds = new Set<string>()
  const structItem = itemIndex.get(structNodeId)

  if (structItem && Array.isArray(structItem.methods)) {
    for (const method of structItem.methods) {
      if (method.method_id) {
        methodNodeIds.add(method.method_id)
      }
    }
  }

  for (const edge of graphEdges) {
    if (edge.kind !== 'contain' || edge.from !== structNodeId) {
      continue
    }
    if (graphNodes.some((node) => node.id === edge.to && node.kind === 'method')) {
      methodNodeIds.add(edge.to)
    }
  }

  const nodeIds = new Set<string>([structNodeId])
  for (const nodeId of methodNodeIds) {
    nodeIds.add(nodeId)
  }

  const nodes = graphNodes.filter((node) => nodeIds.has(node.id))
  const edges = graphEdges.filter(
    (edge) =>
      edge.kind === 'contain' &&
      edge.from === structNodeId &&
      nodeIds.has(edge.from) &&
      nodeIds.has(edge.to),
  )
  const totalMethods = nodes.filter((node) => node.kind === 'method').length

  return {
    nodes,
    edges,
    nodeIds,
    totalMethods,
  }
}

export function buildItemIndex(artifact: ArtifactContract): Map<string, RustItem> {
  const index = new Map<string, RustItem>()

  for (const crateItem of artifact.crates) {
    for (const moduleItem of crateItem.modules) {
      for (const item of moduleItem.items) {
        index.set(item.id, item)
      }
    }
  }

  return index
}

export function buildWarningIndex(
  warnings: WarningItem[],
): Map<string, WarningItem[]> {
  const index = new Map<string, WarningItem[]>()

  for (const warning of warnings) {
    if (!warning.item_id) {
      continue
    }

    const current = index.get(warning.item_id) ?? []
    current.push(warning)
    index.set(warning.item_id, current)
  }

  return index
}

function crateFromPath(path: string): string | null {
  const [crateName] = path.split('::')
  if (!crateName) {
    return null
  }
  return crateName.length > 0 ? crateName : null
}

function crateFromNodeId(nodeId: string): string | null {
  return crateFromPath(nodeId)
}

export function buildNodeCrateIndex(
  graphNodes: GraphNode[],
  byContainer: Record<string, string[]>,
): Map<string, string> {
  const nodeCrateIndex = new Map<string, string>()

  for (const [containerPath, nodeIds] of Object.entries(byContainer)) {
    const crateName = crateFromPath(containerPath)
    if (!crateName) {
      continue
    }

    for (const nodeId of nodeIds) {
      nodeCrateIndex.set(nodeId, crateName)
    }
  }

  for (const node of graphNodes) {
    if (nodeCrateIndex.has(node.id)) {
      continue
    }
    nodeCrateIndex.set(node.id, crateFromNodeId(node.id) ?? 'unknown')
  }

  return nodeCrateIndex
}

export function filterGraphByCrateThenKind(
  graphNodes: GraphNode[],
  graphEdges: GraphEdgeWithId[],
  nodeCrateIndex: Map<string, string>,
  crateFilter: CrateFilterState,
  kindFilter: KindFilterState,
): {
  crateFilteredNodes: GraphNode[]
  crateFilteredNodeIds: Set<string>
  visibleNodes: GraphNode[]
  visibleEdges: GraphEdgeWithId[]
  visibleNodeIds: Set<string>
} {
  const crateFilteredNodes = graphNodes.filter((node) => {
    const crateName = nodeCrateIndex.get(node.id)
    if (!crateName) {
      return true
    }
    return crateFilter[crateName] !== false
  })
  const crateFilteredNodeIds = new Set(crateFilteredNodes.map((node) => node.id))
  const visibleNodes = crateFilteredNodes.filter(
    (node) => kindFilter[node.kind] !== false,
  )
  const visibleNodeIds = new Set(visibleNodes.map((node) => node.id))
  const visibleEdges = graphEdges.filter(
    (edge) => visibleNodeIds.has(edge.from) && visibleNodeIds.has(edge.to),
  )

  return {
    crateFilteredNodes,
    crateFilteredNodeIds,
    visibleNodes,
    visibleEdges,
    visibleNodeIds,
  }
}

export function computeHighlightSet(
  selectedNodeId: string | null,
  visibleEdges: GraphEdgeWithId[],
  visibleNodeIds: Set<string>,
): HighlightSet {
  if (!selectedNodeId || !visibleNodeIds.has(selectedNodeId)) {
    return {
      nodeIds: new Set(visibleNodeIds),
      edgeIds: new Set(visibleEdges.map((edge) => edge.id)),
    }
  }

  const nodeIds = new Set<string>([selectedNodeId])
  const edgeIds = new Set<string>()

  for (const edge of visibleEdges) {
    // Downstream-only highlighting: only keep edges that originate from selection.
    if (edge.from !== selectedNodeId) {
      continue
    }

    edgeIds.add(edge.id)
    nodeIds.add(edge.to)
  }

  return {
    nodeIds,
    edgeIds,
  }
}

function buildRelationSummary(
  selectedNodeId: string,
  nodeKindIndex: Map<string, string>,
  edges: GraphEdgeWithId[],
): RelationSummaryItem[] {
  const relations: RelationSummaryItem[] = []

  for (const edge of edges) {
    if (edge.from !== selectedNodeId) {
      continue
    }

    const peerKind = nodeKindIndex.get(edge.to) ?? 'unknown'

    let relationLabel: string = edge.kind
    if (edge.kind === 'impl') {
      relationLabel = 'impl trait'
    } else if (edge.kind === 'inherit') {
      relationLabel = 'inherit trait'
    } else if (edge.kind === 'call') {
      relationLabel = 'call'
    } else if (edge.kind === 'contain') {
      relationLabel = 'contain'
    } else if (edge.kind === 'use') {
      relationLabel = 'use'
    }

    relations.push({
      edgeKind: edge.kind,
      relationLabel,
      sourceContext: edge.source_context,
      peerId: edge.to,
      peerKind,
    })
  }

  return relations
}

export function buildInspectorViewModel(
  artifact: ArtifactContract,
  itemIndex: Map<string, RustItem>,
  warningIndex: Map<string, WarningItem[]>,
  selectedNodeId: string | null,
): InspectorViewModel | null {
  if (!selectedNodeId) {
    return null
  }

  const node = artifact.graph_index.nodes.find((candidate) => candidate.id === selectedNodeId)
  if (!node) {
    return null
  }

  const item = itemIndex.get(selectedNodeId) ?? null
  const location =
    item?.file && item.span
      ? {
          file: item.file,
          line: item.span.start_line + 1,
          col: item.span.start_col + 1,
        }
      : null

  const edgeList = withEdgeIds(artifact.graph_index.edges)
  const nodeKindIndex = new Map(
    artifact.graph_index.nodes.map((graphNode) => [graphNode.id, graphNode.kind]),
  )
  const relations = buildRelationSummary(selectedNodeId, nodeKindIndex, edgeList)
  const relationCountByLabel = relations.reduce<Record<string, number>>((acc, relation) => {
    acc[relation.relationLabel] = (acc[relation.relationLabel] ?? 0) + 1
    return acc
  }, {})

  return {
    node,
    item,
    location,
    relations,
    relationCountByLabel,
    warnings: warningIndex.get(selectedNodeId) ?? [],
  }
}
