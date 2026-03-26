import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import { kindThemeMap } from "../../styles/ui.styles";
import { nodeCardTw, nodeCardVariants } from "../../styles/node-card.styles";
import { cn } from "../../lib/cn";

export interface GraphNodeData extends Record<string, unknown> {
  kind: string;
  label: string;
  nodeId: string;
  selected: boolean;
  dimmed: boolean;
  onPointerDown?: (nodeId: string) => void;
}

export type RustFlowNode = Node<GraphNodeData, "rustNode">;

function resolveKindTheme(kind: string) {
  return (
    kindThemeMap[kind] ?? {
      text: "hsl(var(--rm-kind-crate))",
      softBg:
        "color-mix(in oklab, hsl(var(--rm-kind-crate)) 16%, hsl(var(--rm-surface)))",
    }
  );
}

export function GraphNodeCard({ data }: NodeProps<RustFlowNode>) {
  const theme = resolveKindTheme(data.kind);

  return (
    <article
      className={cn(
        nodeCardVariants({
          state: data.selected
            ? "selected"
            : data.dimmed
              ? "dimmed"
              : "default",
        }),
      )}
      onMouseDown={(event) => {
        if (event.button !== 0) {
          return;
        }
        data.onPointerDown?.(data.nodeId);
      }}
    >
      <span
        className={nodeCardTw.kindBadge}
        title={data.kind}
        style={{
          color: theme.text,
          backgroundColor: theme.softBg,
        }}
      >
        {data.kind}
      </span>
      <p className={nodeCardTw.label} title={data.label}>
        {data.label}
      </p>
      <p className={nodeCardTw.id} title={data.nodeId}>
        {data.nodeId}
      </p>

      <Handle
        id="target"
        type="target"
        position={Position.Left}
        className="pointer-events-none h-1 w-1 border-0 bg-transparent opacity-0"
      />
      <Handle
        id="source"
        type="source"
        position={Position.Right}
        className="pointer-events-none h-1 w-1 border-0 bg-transparent opacity-0"
      />
    </article>
  );
}
