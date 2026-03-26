import {
  Background,
  BackgroundVariant,
  ConnectionMode,
  MarkerType,
  ReactFlow,
  type Edge,
  type EdgeChange,
  type NodeChange,
  type OnNodeDrag,
  type ReactFlowInstance,
  type NodeTypes,
  useEdgesState,
  useNodesState,
} from "@xyflow/react";
import { Search, X } from "lucide-react";
import {
  forwardRef,
  useCallback,
  useEffect,
  useImperativeHandle,
  useMemo,
  useRef,
  useState,
  type MouseEvent,
} from "react";
import type { GraphNode } from "../lib/artifact-contract";
import type { GraphEdgeWithId } from "../lib/graph";
import type { HighlightSet, NodeSelection } from "../app/types";
import {
  createForceRuntime,
  type ForceRuntime,
  type Position,
} from "../lib/force-layout";
import { graphCanvasTw } from "../styles/graph-canvas.styles";
import { GraphNodeCard, type RustFlowNode } from "./graph/GraphNodeCard";

export interface GraphCanvasHandle {
  fitView: () => void;
  resetLayout: () => void;
  focusNode: (nodeId: string) => void;
}

interface GraphCanvasProps {
  hasArtifact: boolean;
  nodes: GraphNode[];
  edges: GraphEdgeWithId[];
  selection: NodeSelection;
  highlightSet: HighlightSet;
  onSelectNode: (nodeId: string | null) => void;
}

interface RustFlowEdgeData extends Record<string, unknown> {
  kind: GraphEdgeWithId["kind"];
  kindLabel: string;
  sourceContext: string;
}

type RustFlowEdge = Edge<RustFlowEdgeData, "straight">;

interface EdgeTooltipState {
  x: number;
  y: number;
  kind: GraphEdgeWithId["kind"];
  sourceContext: string;
}

interface SearchNodeCandidate {
  id: string;
  label: string;
  kind: string;
  score: number;
}

const nodeTypes: NodeTypes = {
  rustNode: GraphNodeCard,
};

// Raise this to make click selection easier (less accidental drag on slight cursor movement).
const NODE_DRAG_THRESHOLD = 9;

function isFinitePosition(position: Position): boolean {
  return Number.isFinite(position.x) && Number.isFinite(position.y);
}

function edgeColor(kind: GraphEdgeWithId["kind"]): string {
  if (kind === "impl") {
    return "hsl(var(--rm-kind-fn))";
  }

  if (kind === "inherit") {
    return "hsl(var(--rm-kind-struct))";
  }

  if (kind === "call") {
    return "hsl(var(--rm-accent))";
  }

  return "hsl(var(--rm-text-tertiary))";
}

function edgeKindLabel(kind: GraphEdgeWithId["kind"]): string {
  if (kind === "impl") {
    return "impl trait";
  }

  if (kind === "inherit") {
    return "inherit trait";
  }

  if (kind === "call") {
    return "call";
  }

  return "use";
}

function resolveNodeZIndex(
  nodeId: string,
  selection: NodeSelection,
  highlightSet: HighlightSet,
): number {
  if (selection.nodeId === nodeId) {
    return 30;
  }

  if (selection.nodeId !== null && highlightSet.nodeIds.has(nodeId)) {
    return 20;
  }

  if (selection.nodeId !== null) {
    return 2;
  }

  return 10;
}

function mapNodes(
  nodes: GraphNode[],
  positions: Map<string, Position>,
  selection: NodeSelection,
  highlightSet: HighlightSet,
  onNodePointerDown: (nodeId: string) => void,
): RustFlowNode[] {
  return nodes.map((node, index) => {
    const position = positions.get(node.id) ?? {
      x: 120 + (index % 8) * 140,
      y: 120 + Math.floor(index / 8) * 120,
    };

    const selected = selection.nodeId === node.id;
    const dimmed =
      selection.nodeId !== null && !highlightSet.nodeIds.has(node.id);

    return {
      id: node.id,
      type: "rustNode",
      draggable: true,
      zIndex: resolveNodeZIndex(node.id, selection, highlightSet),
      position,
      data: {
        kind: node.kind,
        label: node.label,
        nodeId: node.id,
        selected,
        dimmed,
        onPointerDown: onNodePointerDown,
      },
    };
  });
}

function mapEdges(
  edges: GraphEdgeWithId[],
  selection: NodeSelection,
  highlightSet: HighlightSet,
): RustFlowEdge[] {
  return edges.map((edge) => {
    const highlighted = highlightSet.edgeIds.has(edge.id);
    const dimmed = selection.nodeId !== null && !highlighted;
    const color = edgeColor(edge.kind);
    const kindLabel = edgeKindLabel(edge.kind);

    return {
      id: edge.id,
      source: edge.from,
      target: edge.to,
      type: "straight",
      markerEnd: {
        type: MarkerType.ArrowClosed,
        color,
        width: 18,
        height: 18,
      },
      data: {
        kind: edge.kind,
        kindLabel,
        sourceContext: edge.source_context,
      },
      label: kindLabel,
      labelStyle: {
        fill: "hsl(var(--rm-text-secondary))",
        fontSize: 10,
        fontWeight: 600,
        textTransform: "uppercase",
        letterSpacing: "0.06em",
      },
      labelBgStyle: {
        fill: "hsl(var(--rm-surface))",
        fillOpacity: 0.88,
      },
      labelBgPadding: [3, 6],
      labelBgBorderRadius: 8,
      style: {
        stroke: color,
        strokeWidth: highlighted ? 1.8 : 1.1,
        opacity: dimmed ? 0.24 : 0.9,
      },
      animated: selection.nodeId !== null && highlighted,
    };
  });
}

export const GraphCanvas = forwardRef<GraphCanvasHandle, GraphCanvasProps>(
  function GraphCanvas(
    { hasArtifact, nodes, edges, selection, highlightSet, onSelectNode },
    ref,
  ) {
    const reactFlowRef = useRef<ReactFlowInstance<
      RustFlowNode,
      RustFlowEdge
    > | null>(null);
    const canvasRef = useRef<HTMLElement | null>(null);
    const runtimeRef = useRef<ForceRuntime | null>(null);
    const positionsRef = useRef<Map<string, Position>>(new Map());
    const draggingNodeIdsRef = useRef<Set<string>>(new Set());
    const pendingTickRafRef = useRef<number | null>(null);
    const pendingTickPositionsRef = useRef<Map<string, Position> | null>(null);
    const selectionRef = useRef(selection);
    const highlightSetRef = useRef(highlightSet);

    const [layoutNonce, setLayoutNonce] = useState(0);
    const [flowNodes, setFlowNodes, setNodesChangeBase] =
      useNodesState<RustFlowNode>([]);
    const [flowEdges, setFlowEdges, setEdgesChangeBase] =
      useEdgesState<RustFlowEdge>([]);
    const [edgeTooltip, setEdgeTooltip] = useState<EdgeTooltipState | null>(
      null,
    );
    const [searchQuery, setSearchQuery] = useState("");

    const stableNodeTypes = useMemo(() => nodeTypes, []);
    const normalizedSearchQuery = searchQuery.trim().toLowerCase();
    const searchCandidates = useMemo<SearchNodeCandidate[]>(() => {
      if (normalizedSearchQuery.length === 0) {
        return [];
      }

      const candidates: SearchNodeCandidate[] = [];

      for (const node of nodes) {
        const idLower = node.id.toLowerCase();
        const labelLower = node.label.toLowerCase();
        const kindLower = node.kind.toLowerCase();

        let score = Number.POSITIVE_INFINITY;
        if (labelLower === normalizedSearchQuery || idLower === normalizedSearchQuery) {
          score = 0;
        } else if (labelLower.startsWith(normalizedSearchQuery)) {
          score = 1;
        } else if (idLower.startsWith(normalizedSearchQuery)) {
          score = 2;
        } else if (kindLower.startsWith(normalizedSearchQuery)) {
          score = 3;
        } else if (labelLower.includes(normalizedSearchQuery)) {
          score = 4;
        } else if (idLower.includes(normalizedSearchQuery)) {
          score = 5;
        } else if (kindLower.includes(normalizedSearchQuery)) {
          score = 6;
        }

        if (!Number.isFinite(score)) {
          continue;
        }

        candidates.push({
          id: node.id,
          label: node.label,
          kind: node.kind,
          score,
        });
      }

      candidates.sort((left, right) => {
        if (left.score !== right.score) {
          return left.score - right.score;
        }
        return left.label.localeCompare(right.label);
      });

      return candidates.slice(0, 12);
    }, [nodes, normalizedSearchQuery]);

    const fitView = useCallback(() => {
      reactFlowRef.current?.fitView({
        duration: 220,
        padding: 0.14,
      });
    }, []);

    const focusNodeById = useCallback((nodeId: string) => {
      const instance = reactFlowRef.current;
      if (!instance) {
        return;
      }

      const node = instance.getNode(nodeId);
      const fallbackPosition = positionsRef.current.get(nodeId);
      if (!node && !fallbackPosition) {
        return;
      }

      const width = node?.width ?? 170;
      const height = node?.height ?? 90;
      const centerX = (node?.position.x ?? fallbackPosition?.x ?? 0) + width / 2;
      const centerY = (node?.position.y ?? fallbackPosition?.y ?? 0) + height / 2;

      instance.setCenter(centerX, centerY, {
        zoom: Math.max(instance.getZoom(), 0.72),
        duration: 260,
      });
    }, []);

    const syncNodePresentation = useCallback(
      (nextPositions: Map<string, Position>) => {
        setFlowNodes(
          mapNodes(
            nodes,
            nextPositions,
            selectionRef.current,
            highlightSetRef.current,
            onSelectNode,
          ),
        );
      },
      [nodes, onSelectNode, setFlowNodes],
    );

    const stopRuntime = useCallback(() => {
      runtimeRef.current?.stop();
      runtimeRef.current = null;
      draggingNodeIdsRef.current.clear();
      if (pendingTickRafRef.current !== null) {
        cancelAnimationFrame(pendingTickRafRef.current);
        pendingTickRafRef.current = null;
      }
      pendingTickPositionsRef.current = null;
    }, []);

    const commitTickPositions = useCallback(
      (nextPositions: Map<string, Position>) => {
        const draggingNodeIds = draggingNodeIdsRef.current;
        if (draggingNodeIds.size === 0) {
          positionsRef.current = nextPositions;
          setFlowNodes((currentNodes) => {
            let changed = false;
            const nextNodes = currentNodes.map((node) => {
              const nextPosition = nextPositions.get(node.id);
              if (!nextPosition || !isFinitePosition(nextPosition)) {
                return node;
              }

              if (
                node.position.x === nextPosition.x &&
                node.position.y === nextPosition.y
              ) {
                return node;
              }

              changed = true;
              return {
                ...node,
                position: nextPosition,
              };
            });

            return changed ? nextNodes : currentNodes;
          });
          return;
        }

        const mergedPositions = new Map(positionsRef.current);
        for (const [nodeId, position] of nextPositions.entries()) {
          if (!draggingNodeIds.has(nodeId)) {
            mergedPositions.set(nodeId, position);
          }
        }

        for (const draggingNodeId of draggingNodeIds) {
          const pinnedPosition = positionsRef.current.get(draggingNodeId);
          if (pinnedPosition && isFinitePosition(pinnedPosition)) {
            mergedPositions.set(draggingNodeId, pinnedPosition);
          }
        }

        positionsRef.current = mergedPositions;
        setFlowNodes((currentNodes) =>
          currentNodes.map((node) => {
            if (draggingNodeIds.has(node.id)) {
              return node;
            }

            const nextPosition = mergedPositions.get(node.id);
            if (!nextPosition || !isFinitePosition(nextPosition)) {
              return node;
            }

            if (
              node.position.x === nextPosition.x &&
              node.position.y === nextPosition.y
            ) {
              return node;
            }

            return {
              ...node,
              position: nextPosition,
            };
          }),
        );
      },
      [setFlowNodes],
    );

    const scheduleTickCommit = useCallback(
      (nextPositions: Map<string, Position>) => {
        pendingTickPositionsRef.current = nextPositions;

        if (pendingTickRafRef.current !== null) {
          return;
        }

        pendingTickRafRef.current = requestAnimationFrame(() => {
          pendingTickRafRef.current = null;
          const latestPositions = pendingTickPositionsRef.current;
          pendingTickPositionsRef.current = null;
          if (!latestPositions) {
            return;
          }
          commitTickPositions(latestPositions);
        });
      },
      [commitTickPositions],
    );

    const resolveDragPosition = useCallback(
      (nodeId: string, position: Position): Position => {
        if (isFinitePosition(position)) {
          return position;
        }

        const cached = positionsRef.current.get(nodeId);
        if (cached && isFinitePosition(cached)) {
          return cached;
        }

        return { x: 120, y: 120 };
      },
      [],
    );

    useImperativeHandle(
      ref,
      () => ({
        fitView,
        focusNode: focusNodeById,
        resetLayout: () => {
          positionsRef.current.clear();
          stopRuntime();
          setLayoutNonce((prev) => prev + 1);
        },
      }),
      [fitView, focusNodeById, stopRuntime],
    );

    useEffect(() => {
      selectionRef.current = selection;
      highlightSetRef.current = highlightSet;
    }, [highlightSet, selection]);

    useEffect(() => {
      return () => {
        stopRuntime();
      };
    }, [stopRuntime]);

    useEffect(() => {
      const visibleNodeIds = new Set(nodes.map((node) => node.id));
      for (const savedNodeId of Array.from(positionsRef.current.keys())) {
        if (!visibleNodeIds.has(savedNodeId)) {
          positionsRef.current.delete(savedNodeId);
        }
      }

      stopRuntime();

      const canvasRect = canvasRef.current?.getBoundingClientRect();
      const layoutWidth = Math.max(640, Math.round(canvasRect?.width ?? 1280));
      const layoutHeight = Math.max(420, Math.round(canvasRect?.height ?? 760));

      const runtime = createForceRuntime({
        nodes,
        edges,
        existingPositions: positionsRef.current,
        width: layoutWidth,
        height: layoutHeight,
        onTick: scheduleTickCommit,
      });

      if (!runtime) {
        setFlowNodes([]);
        setFlowEdges([]);
        return;
      }

      runtimeRef.current = runtime;
      const initialPositions = runtime.getPositions();
      positionsRef.current = initialPositions;

      setFlowEdges(
        mapEdges(edges, selectionRef.current, highlightSetRef.current),
      );
      syncNodePresentation(initialPositions);

      requestAnimationFrame(() => {
        fitView();
      });
    }, [
      edges,
      fitView,
      layoutNonce,
      nodes,
      setFlowEdges,
      setFlowNodes,
      stopRuntime,
      syncNodePresentation,
      scheduleTickCommit,
    ]);

    useEffect(() => {
      setFlowNodes((currentNodes) =>
        currentNodes.map((node) => {
          const nextSelected = selection.nodeId === node.id;
          const nextDimmed =
            selection.nodeId !== null && !highlightSet.nodeIds.has(node.id);
          const nextZIndex = resolveNodeZIndex(node.id, selection, highlightSet);

          if (
            node.data.selected === nextSelected &&
            node.data.dimmed === nextDimmed &&
            node.zIndex === nextZIndex
          ) {
            return node;
          }

          return {
            ...node,
            zIndex: nextZIndex,
            data: {
              ...node.data,
              selected: nextSelected,
              dimmed: nextDimmed,
            },
          };
        }),
      );
      setFlowEdges(mapEdges(edges, selection, highlightSet));
    }, [edges, highlightSet, selection, setFlowEdges, setFlowNodes]);

    const onNodesChange = useCallback(
      (changes: NodeChange<RustFlowNode>[]) => {
        // Position updates are owned by d3 runtime + drag handlers to avoid state races.
        const safeChanges = changes.filter(
          (change) =>
            change.type === "dimensions" ||
            change.type === "select",
        );

        if (safeChanges.length > 0) {
          setNodesChangeBase(safeChanges);
        }
      },
      [setNodesChangeBase],
    );

    const onEdgesChange = useCallback(
      (changes: EdgeChange<RustFlowEdge>[]) => {
        const safeChanges = changes.filter(
          (change) =>
            change.type === "select" || change.type === "add" || change.type === "remove",
        );
        if (safeChanges.length > 0) {
          setEdgesChangeBase(safeChanges);
        }
      },
      [setEdgesChangeBase],
    );

    const onNodeDragStart = useCallback<OnNodeDrag<RustFlowNode>>(
      (_, node) => {
        draggingNodeIdsRef.current.add(node.id);
        const runtime = runtimeRef.current;
        if (!runtime) {
          return;
        }

        const safePosition = resolveDragPosition(node.id, node.position);
        positionsRef.current.set(node.id, safePosition);
        runtime.pinNode(node.id, safePosition);
        runtime.reheat(0.24);
      },
      [resolveDragPosition],
    );

    const onNodeDrag = useCallback<OnNodeDrag<RustFlowNode>>(
      (_, node) => {
        const runtime = runtimeRef.current;
        if (!runtime) {
          return;
        }

        const safePosition = resolveDragPosition(node.id, node.position);
        positionsRef.current.set(node.id, safePosition);
        runtime.movePinnedNode(node.id, safePosition);
        runtime.reheat(0.24);

        setFlowNodes((currentNodes) =>
          currentNodes.map((currentNode) =>
            currentNode.id === node.id
              ? {
                  ...currentNode,
                  position: safePosition,
                }
              : currentNode,
          ),
        );
      },
      [resolveDragPosition, setFlowNodes],
    );

    const onNodeDragStop = useCallback<OnNodeDrag<RustFlowNode>>(
      (_, node) => {
        draggingNodeIdsRef.current.delete(node.id);

        const runtime = runtimeRef.current;
        if (!runtime) {
          return;
        }

        const safePosition = resolveDragPosition(node.id, node.position);
        positionsRef.current.set(node.id, safePosition);
        runtime.movePinnedNode(node.id, safePosition);
        runtime.releaseNode(node.id);
        runtime.reheat(0.34);
        runtime.simulation.alphaTarget(0);
      },
      [resolveDragPosition],
    );

    const updateEdgeTooltip = useCallback(
      (event: MouseEvent<Element>, edge: RustFlowEdge) => {
        const rootRect = canvasRef.current?.getBoundingClientRect();
        if (!rootRect) {
          return;
        }

        setEdgeTooltip({
          x: event.clientX - rootRect.left + 12,
          y: event.clientY - rootRect.top + 12,
          kind: (edge.data?.kind ?? "use") as GraphEdgeWithId["kind"],
          sourceContext: String(
            edge.data?.sourceContext ?? edge.data?.kindLabel ?? "unknown",
          ),
        });
      },
      [],
    );

    const selectSearchCandidate = useCallback(
      (nodeId: string) => {
        onSelectNode(nodeId);
        focusNodeById(nodeId);
        setSearchQuery("");
      },
      [focusNodeById, onSelectNode],
    );

    return (
      <section className={graphCanvasTw.root} ref={canvasRef}>
        {hasArtifact ? (
          <div className={graphCanvasTw.searchWrap}>
            <div className={graphCanvasTw.searchInputWrap}>
              <Search className={graphCanvasTw.searchIcon} />
              <input
                value={searchQuery}
                onChange={(event) => {
                  setSearchQuery(event.target.value);
                }}
                onKeyDown={(event) => {
                  if (event.key === "Enter" && searchCandidates.length > 0) {
                    event.preventDefault();
                    selectSearchCandidate(searchCandidates[0].id);
                    return;
                  }

                  if (event.key === "Escape") {
                    event.preventDefault();
                    setSearchQuery("");
                  }
                }}
                className={graphCanvasTw.searchInput}
                placeholder="Search node by label, id, or kind"
                aria-label="Search node"
              />
              {searchQuery.length > 0 ? (
                <button
                  type="button"
                  className={graphCanvasTw.searchClear}
                  onClick={() => {
                    setSearchQuery("");
                  }}
                  aria-label="Clear search"
                >
                  <X className="h-3.5 w-3.5" />
                </button>
              ) : null}
            </div>

            {normalizedSearchQuery.length > 0 ? (
              <div className={graphCanvasTw.searchResultPanel}>
                {searchCandidates.length === 0 ? (
                  <p className={graphCanvasTw.searchEmpty}>No matching node in current canvas.</p>
                ) : (
                  <ul className="space-y-1">
                    {searchCandidates.map((candidate) => (
                      <li key={candidate.id}>
                        <button
                          type="button"
                          onClick={() => {
                            selectSearchCandidate(candidate.id);
                          }}
                          className={graphCanvasTw.searchResultItem}
                          title={`${candidate.label} (${candidate.kind})`}
                        >
                          <div className={graphCanvasTw.searchResultTop}>
                            <p className={graphCanvasTw.searchResultLabel}>{candidate.label}</p>
                            <span className={graphCanvasTw.searchResultKind}>{candidate.kind}</span>
                          </div>
                          <p className={graphCanvasTw.searchResultId}>{candidate.id}</p>
                        </button>
                      </li>
                    ))}
                  </ul>
                )}
              </div>
            ) : null}
          </div>
        ) : null}

        {!hasArtifact ? (
          <div className={graphCanvasTw.emptyState}>
            <div className={graphCanvasTw.emptyInner}>
              <h2 className={graphCanvasTw.emptyTitle}>
                Upload a RustMap JSON artifact
              </h2>
              <p className={graphCanvasTw.emptyBody}>
                The canvas renders `graph_index.nodes` and `graph_index.edges`
                after strict contract validation.
              </p>
            </div>
          </div>
        ) : null}

        {edgeTooltip ? (
          <div
            className={graphCanvasTw.edgeTooltip}
            style={{
              left: edgeTooltip.x,
              top: edgeTooltip.y,
            }}
          >
            <p className={graphCanvasTw.edgeTooltipTitle}>{edgeTooltip.kind}</p>
            <p
              className={graphCanvasTw.edgeTooltipBody}
              title={edgeTooltip.sourceContext}
            >
              {edgeTooltip.sourceContext}
            </p>
          </div>
        ) : null}

        <ReactFlow<RustFlowNode, RustFlowEdge>
          nodes={flowNodes}
          edges={flowEdges}
          nodeTypes={stableNodeTypes}
          onInit={(instance) => {
            reactFlowRef.current = instance;
          }}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onNodeDragStart={onNodeDragStart}
          onNodeDrag={onNodeDrag}
          onNodeDragStop={onNodeDragStop}
          onNodeClick={(_, node) => {
            onSelectNode(node.id);
          }}
          onPaneClick={() => {
            setEdgeTooltip(null);
            onSelectNode(null);
          }}
          onEdgeMouseEnter={updateEdgeTooltip}
          onEdgeMouseMove={updateEdgeTooltip}
          onEdgeMouseLeave={() => {
            setEdgeTooltip(null);
          }}
          fitView
          panOnDrag={false}
          zoomOnPinch
          panOnScroll
          panOnScrollSpeed={1.2}
          zoomOnDoubleClick={false}
          connectionMode={ConnectionMode.Loose}
          nodesDraggable
          nodeDragThreshold={NODE_DRAG_THRESHOLD}
          selectNodesOnDrag={false}
          elementsSelectable
          minZoom={0.2}
          maxZoom={1.8}
          proOptions={{ hideAttribution: true }}
        >
          <Background
            variant={BackgroundVariant.Dots}
            gap={20}
            size={1.2}
            color="hsl(var(--rm-border))"
          />
        </ReactFlow>
      </section>
    );
  },
);
