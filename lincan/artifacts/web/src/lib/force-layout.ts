import {
  forceCenter,
  forceCollide,
  forceLink,
  forceManyBody,
  forceSimulation,
  type Force,
  type Simulation,
  type SimulationNodeDatum,
} from 'd3-force'
import type { GraphEdgeWithId } from './graph'
import type { GraphNode } from './artifact-contract'

export interface Position {
  x: number
  y: number
}

interface ForceNode extends SimulationNodeDatum {
  id: string
  x: number
  y: number
  fx?: number | null
  fy?: number | null
}

interface ForceLink {
  source: string | ForceNode
  target: string | ForceNode
}

interface ForceRuntimeInput {
  nodes: GraphNode[]
  edges: GraphEdgeWithId[]
  existingPositions: Map<string, Position>
  width: number
  height: number
  onTick: (positions: Map<string, Position>) => void
}

interface ForceLayoutTuning {
  chargeStrength: number
  chargeDistanceMin: number
  chargeDistanceMax: number
  chargeTheta: number
  linkDistance: number
  linkStrength: number
  collisionRadius: number
  alphaMin: number
  alphaDecay: number
  velocityDecay: number
  boundsStrength: number
  maxBoundsPush: number
}

export const FORCE_LAYOUT_TUNING: ForceLayoutTuning = {
  // More negative => stronger node-to-node repulsion.
  chargeStrength: -640,
  chargeDistanceMin: 10,
  chargeDistanceMax: 560,
  chargeTheta: 0.9,
  // Higher distance/strength => tighter edge-constrained grouping.
  linkDistance: 250,
  linkStrength: 0.34,
  // Collision radius for overlap prevention.
  collisionRadius: 58,
  // Global simulation damping.
  alphaMin: 0.018,
  alphaDecay: 0.035,
  velocityDecay: 0.35,
  // Soft boundary force (velocity-based), keeps rebound continuous.
  boundsStrength: 0.22,
  maxBoundsPush: 56,
}

const FORCE_LAYOUT_BOUNDS = {
  horizontalPadding: 124,
  verticalPadding: 84,
} as const

interface LayoutBounds {
  minX: number
  maxX: number
  minY: number
  maxY: number
}

function resolveForceLayoutTuning(nodeCount: number): ForceLayoutTuning {
  if (nodeCount >= 220) {
    return {
      chargeStrength: -280,
      chargeDistanceMin: 10,
      chargeDistanceMax: 420,
      chargeTheta: 1,
      linkDistance: 150,
      linkStrength: 0.22,
      collisionRadius: 42,
      alphaMin: 0.03,
      alphaDecay: 0.06,
      velocityDecay: 0.46,
      boundsStrength: 0.2,
      maxBoundsPush: 34,
    }
  }

  if (nodeCount >= 120) {
    return {
      chargeStrength: -340,
      chargeDistanceMin: 10,
      chargeDistanceMax: 500,
      chargeTheta: 0.95,
      linkDistance: 180,
      linkStrength: 0.26,
      collisionRadius: 48,
      alphaMin: 0.024,
      alphaDecay: 0.05,
      velocityDecay: 0.42,
      boundsStrength: 0.2,
      maxBoundsPush: 40,
    }
  }

  return FORCE_LAYOUT_TUNING
}

function isFiniteNumber(value: unknown): value is number {
  return typeof value === 'number' && Number.isFinite(value)
}

function clamp(value: number, min: number, max: number): number {
  if (value < min) {
    return min
  }
  if (value > max) {
    return max
  }
  return value
}

function clampToBounds(position: Position, bounds: LayoutBounds): Position {
  return {
    x: clamp(position.x, bounds.minX, bounds.maxX),
    y: clamp(position.y, bounds.minY, bounds.maxY),
  }
}

function createLayoutBounds(width: number, height: number): LayoutBounds {
  const minX = FORCE_LAYOUT_BOUNDS.horizontalPadding
  const minY = FORCE_LAYOUT_BOUNDS.verticalPadding
  const maxX = Math.max(minX, width - FORCE_LAYOUT_BOUNDS.horizontalPadding)
  const maxY = Math.max(minY, height - FORCE_LAYOUT_BOUNDS.verticalPadding)
  return {
    minX,
    maxX,
    minY,
    maxY,
  }
}

function sanitizePosition(
  raw: Position,
  fallback: Position,
): Position {
  const safeX = isFiniteNumber(raw.x) ? raw.x : fallback.x
  const safeY = isFiniteNumber(raw.y) ? raw.y : fallback.y
  return {
    x: safeX,
    y: safeY,
  }
}

function normalizeNode(node: ForceNode, fallback: Position) {
  const safe = sanitizePosition({ x: node.x, y: node.y }, fallback)
  node.x = safe.x
  node.y = safe.y

  node.vx = isFiniteNumber(node.vx) ? node.vx : 0
  node.vy = isFiniteNumber(node.vy) ? node.vy : 0
}

function createSoftBoundsForce(
  bounds: LayoutBounds,
  strength: number,
  maxPush: number,
): Force<ForceNode, ForceLink> {
  let currentNodes: ForceNode[] = []

  const force = ((alpha: number) => {
    const base = Math.max(0, Math.min(1, strength)) * alpha
    if (base <= 0) {
      return
    }

    for (const node of currentNodes) {
      if (!isFiniteNumber(node.x) || !isFiniteNumber(node.y)) {
        continue
      }

      let deltaX = 0
      let deltaY = 0

      if (node.x < bounds.minX) {
        deltaX = bounds.minX - node.x
      } else if (node.x > bounds.maxX) {
        deltaX = bounds.maxX - node.x
      }

      if (node.y < bounds.minY) {
        deltaY = bounds.minY - node.y
      } else if (node.y > bounds.maxY) {
        deltaY = bounds.maxY - node.y
      }

      if (deltaX === 0 && deltaY === 0) {
        continue
      }

      node.vx = (node.vx ?? 0) + clamp(deltaX * base, -maxPush, maxPush)
      node.vy = (node.vy ?? 0) + clamp(deltaY * base, -maxPush, maxPush)
    }
  }) as Force<ForceNode, ForceLink>

  force.initialize = (nodes) => {
    currentNodes = nodes as ForceNode[]
  }

  return force
}

function seededPosition(index: number, width: number, height: number): Position {
  const col = index % 8
  const row = Math.floor(index / 8)
  return {
    x: width * 0.25 + col * 85,
    y: height * 0.22 + row * 70,
  }
}

export interface ForceRuntime {
  simulation: Simulation<ForceNode, ForceLink>
  nodesById: Map<string, ForceNode>
  getPositions: () => Map<string, Position>
  pinNode: (id: string, position: Position) => void
  movePinnedNode: (id: string, position: Position) => void
  releaseNode: (id: string) => void
  reheat: (alphaTarget?: number) => void
  stop: () => void
}

export function createForceRuntime(input: ForceRuntimeInput): ForceRuntime | null {
  const { nodes, edges, existingPositions, width, height, onTick } = input

  if (nodes.length === 0) {
    return null
  }

  const tuning = resolveForceLayoutTuning(nodes.length)

  const bounds = createLayoutBounds(width, height)
  const centerFallback: Position = {
    x: width / 2,
    y: height / 2,
  }
  const fallbackPositions = new Map<string, Position>()

  const forceNodes: ForceNode[] = nodes.map((node, index) => {
    const savedPosition = existingPositions.get(node.id)
    const seed = seededPosition(index, width, height)
    const start = clampToBounds(sanitizePosition(savedPosition ?? seed, seed), bounds)
    fallbackPositions.set(node.id, start)
    return {
      id: node.id,
      x: start.x,
      y: start.y,
      fx: null,
      fy: null,
    }
  })

  const nodeIds = new Set(forceNodes.map((node) => node.id))
  const forceLinks: ForceLink[] = edges
    .filter((edge) => nodeIds.has(edge.from) && nodeIds.has(edge.to))
    .map((edge) => ({
      source: edge.from,
      target: edge.to,
    }))

  const simulation = forceSimulation(forceNodes)
    .alpha(1)
    .alphaMin(tuning.alphaMin)
    .alphaDecay(tuning.alphaDecay)
    .velocityDecay(tuning.velocityDecay)
    .force(
      'charge',
      forceManyBody<ForceNode>()
        .strength(tuning.chargeStrength)
        .distanceMin(tuning.chargeDistanceMin)
        .distanceMax(tuning.chargeDistanceMax)
        .theta(tuning.chargeTheta),
    )
    .force(
      'link',
      forceLink<ForceNode, ForceLink>(forceLinks)
        .id((node) => node.id)
        .distance(tuning.linkDistance)
        .strength(tuning.linkStrength),
    )
    .force(
      'collide',
      forceCollide<ForceNode>(tuning.collisionRadius),
    )
    .force('center', forceCenter<ForceNode>(width / 2, height / 2))
    .force(
      'bounds',
      createSoftBoundsForce(
        bounds,
        tuning.boundsStrength,
        tuning.maxBoundsPush,
      ),
    )

  const nodesById = new Map(forceNodes.map((node) => [node.id, node]))

  const getPositions = (): Map<string, Position> => {
    const positions = new Map<string, Position>()
    for (const node of forceNodes) {
      normalizeNode(node, fallbackPositions.get(node.id) ?? centerFallback)
      positions.set(node.id, {
        x: node.x,
        y: node.y,
      })
    }
    return positions
  }

  simulation.on('tick', () => {
    onTick(getPositions())
  })

  const pinNode = (id: string, position: Position) => {
    const node = nodesById.get(id)
    if (!node) {
      return
    }
    const safe = sanitizePosition(position, fallbackPositions.get(id) ?? centerFallback)
    node.fx = safe.x
    node.fy = safe.y
    node.x = safe.x
    node.y = safe.y
    node.vx = 0
    node.vy = 0
  }

  const movePinnedNode = (id: string, position: Position) => {
    const node = nodesById.get(id)
    if (!node) {
      return
    }
    const safe = sanitizePosition(position, fallbackPositions.get(id) ?? centerFallback)
    node.fx = safe.x
    node.fy = safe.y
    node.x = safe.x
    node.y = safe.y
    node.vx = 0
    node.vy = 0
  }

  const releaseNode = (id: string) => {
    const node = nodesById.get(id)
    if (!node) {
      return
    }
    node.fx = null
    node.fy = null
  }

  const reheat = (alphaTarget = 0.18) => {
    const safeAlphaTarget =
      isFiniteNumber(alphaTarget) && alphaTarget >= 0 && alphaTarget <= 1
        ? alphaTarget
        : 0.18
    simulation.alphaTarget(safeAlphaTarget).restart()
  }

  const stop = () => {
    simulation.stop()
  }

  return {
    simulation,
    nodesById,
    getPositions,
    pinNode,
    movePinnedNode,
    releaseNode,
    reheat,
    stop,
  }
}
