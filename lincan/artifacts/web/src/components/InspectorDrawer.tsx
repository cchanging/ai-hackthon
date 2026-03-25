import { ArrowRight, ChevronLeft, Copy, ExternalLink, X } from 'lucide-react'
import { useMemo, useState } from 'react'
import type {
  GraphViewMode,
  InspectorViewModel,
  RelationSummaryItem,
} from '../app/types'
import type { GraphNode } from '../lib/artifact-contract'
import { buildVscodeFileUri, copyLocation, locationToClipboardText } from '../lib/code-jump'
import { buttonVariants, iconButtonVariants, panelVariants } from '../styles/ui.styles'
import { inspectorTw } from '../styles/inspector.styles'
import { cn } from '../lib/cn'

interface InspectorDrawerProps {
  open: boolean
  viewModel: InspectorViewModel | null
  canGoBack: boolean
  onGoBack: () => void
  onNavigateRelation: (nodeId: string) => void
  nodeById: Map<string, GraphNode>
  callViewMode: GraphViewMode
  callViewRootNodeId: string | null
  onOpenCallGraph: (rootNodeId: string) => void
  onOpenStructMethodView: (structNodeId: string, methodNodeId?: string) => void
  onClose: () => void
}

function isCallableNodeKind(kind: string): boolean {
  return kind === 'fn' || kind === 'method'
}

const RUST_BUILTIN_TYPE_NAMES = new Set([
  'bool',
  'char',
  'str',
  'String',
  'usize',
  'isize',
  'u8',
  'u16',
  'u32',
  'u64',
  'u128',
  'i8',
  'i16',
  'i32',
  'i64',
  'i128',
  'f32',
  'f64',
  'Option',
  'Result',
  'Vec',
  'VecDeque',
  'LinkedList',
  'HashMap',
  'HashSet',
  'BTreeMap',
  'BTreeSet',
  'BinaryHeap',
  'Box',
  'Rc',
  'Arc',
  'Cell',
  'RefCell',
  'Cow',
  'Path',
  'PathBuf',
  'OsStr',
  'OsString',
  'CString',
  'CStr',
  'Duration',
  'Instant',
  'IpAddr',
  'SocketAddr',
])

function normalizeTypeLeaf(typeText: string): string | null {
  const cleaned = typeText
    .trim()
    .replace(/^&\s*(?:mut\s+)?/, '')
    .replace(/^dyn\s+/, '')
    .replace(/^impl\s+/, '')
  if (!cleaned) {
    return null
  }

  const firstToken = cleaned.split(/[\s<>,()[\]]+/)[0]
  if (!firstToken) {
    return null
  }

  const segments = firstToken.split('::')
  return segments[segments.length - 1] ?? null
}

function isRustBuiltinType(typeExpr: string, typeId: string): boolean {
  const exprLeaf = normalizeTypeLeaf(typeExpr)
  if (exprLeaf && RUST_BUILTIN_TYPE_NAMES.has(exprLeaf)) {
    return true
  }

  const idLeaf = normalizeTypeLeaf(typeId)
  if (idLeaf && RUST_BUILTIN_TYPE_NAMES.has(idLeaf)) {
    return true
  }

  return false
}

function renderStructDetails(
  viewModel: InspectorViewModel,
  onOpenStructMethodView: (structNodeId: string, methodNodeId?: string) => void,
) {
  if (!viewModel.item || !Array.isArray(viewModel.item.methods)) {
    return null
  }

  if (viewModel.item.methods.length === 0) {
    return <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">No methods extracted.</p>
  }

  return (
    <ul className={inspectorTw.list}>
      {viewModel.item.methods.map((method) => {
        const methodId = method.method_id
        const canJumpToMethod = typeof methodId === 'string' && methodId.length > 0
        const jumpTitle = methodId ?? 'missing method_id'

        if (canJumpToMethod) {
          return (
            <li className="min-w-0" key={`${method.name}-${method.trait_id ?? 'inherent'}`}>
              <button
                type="button"
                onClick={() => {
                  onOpenStructMethodView(viewModel.node.id, methodId)
                }}
                className={inspectorTw.relationCard}
                title={jumpTitle}
              >
                <div className={inspectorTw.relationCardTop}>
                  <p className={inspectorTw.relationTitle} title={method.name}>
                    {method.name}
                  </p>
                  <span className={inspectorTw.relationJumpHint}>
                    Jump
                    <ArrowRight className="h-3 w-3" />
                  </span>
                </div>
                <p
                  className="overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.68rem] text-[hsl(var(--rm-text-secondary))]"
                  title={methodId}
                >
                  {methodId}
                </p>
                <span
                  className="mt-1 block overflow-hidden text-ellipsis whitespace-nowrap text-[0.68rem] uppercase tracking-[0.08em] text-[hsl(var(--rm-text-tertiary))]"
                  title={method.source ?? 'inherent'}
                >
                  {method.source ?? 'inherent'}
                </span>
                {method.trait_id ? (
                  <p
                    className="mt-1 overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.68rem] text-[hsl(var(--rm-text-tertiary))]"
                    title={method.trait_id}
                  >
                    {method.trait_id}
                  </p>
                ) : null}
              </button>
            </li>
          )
        }

        return (
          <li className={inspectorTw.listItem} key={`${method.name}-${method.trait_id ?? 'inherent'}`}>
            <span
              className="block overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.72rem] text-[hsl(var(--rm-text))]"
              title={method.name}
            >
              {method.name}
            </span>
            <span
              className="mt-1 block overflow-hidden text-ellipsis whitespace-nowrap text-[0.68rem] uppercase tracking-[0.08em] text-[hsl(var(--rm-text-tertiary))]"
              title={method.source ?? 'inherent'}
            >
              {method.source ?? 'inherent'}
            </span>
            {method.trait_id ? (
              <p
                className="mt-1 overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.68rem] text-[hsl(var(--rm-text-tertiary))]"
                title={method.trait_id}
              >
                {method.trait_id}
              </p>
            ) : null}
          </li>
        )
      })}
    </ul>
  )
}

function renderStructFieldDetails(
  viewModel: InspectorViewModel,
  nodeById: Map<string, GraphNode>,
  onNavigateRelation: (nodeId: string) => void,
) {
  if (!viewModel.item || !Array.isArray(viewModel.item.struct_fields)) {
    return null
  }

  if (viewModel.item.struct_fields.length === 0) {
    return <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">No struct fields extracted.</p>
  }

  return (
    <div className="space-y-2">
      <p className="text-[0.68rem] uppercase tracking-[0.08em] text-[hsl(var(--rm-text-tertiary))]">
        Shape: {viewModel.item.struct_shape ?? 'unknown'}
      </p>
      <ul className={inspectorTw.list}>
        {viewModel.item.struct_fields.map((field, index) => {
          const fieldName =
            field.name ??
            (typeof field.index === 'number' ? `#${field.index}` : `field#${index + 1}`)
          const fieldTypeLabel = field.type_expr
          const fieldTypeId = field.type_id?.trim() ?? ''
          const targetNode = fieldTypeId ? nodeById.get(fieldTypeId) : null
          const builtinType = isRustBuiltinType(fieldTypeLabel, fieldTypeId)
          const canJumpToType = Boolean(fieldTypeId && targetNode && !builtinType)
          const jumpTitle = !fieldTypeId
            ? 'missing type_id'
            : targetNode
              ? `${targetNode.label} (${targetNode.kind}) · ${fieldTypeId}`
              : 'target not found'

          if (canJumpToType) {
            return (
              <li className="min-w-0" key={`${fieldName}-${field.type_expr}-${index}`}>
                <button
                  type="button"
                  onClick={() => {
                    onNavigateRelation(fieldTypeId)
                  }}
                  className={inspectorTw.relationCard}
                  title={jumpTitle}
                >
                  <div className={inspectorTw.relationCardTop}>
                    <p className={inspectorTw.relationTitle} title={fieldName}>
                      {fieldName}
                    </p>
                    <span className={inspectorTw.relationJumpHint}>
                      Jump
                      <ArrowRight className="h-3 w-3" />
                    </span>
                  </div>
                  <p className={inspectorTw.signatureValue} title={fieldTypeLabel}>
                    {fieldTypeLabel}
                  </p>
                  <div className="mt-1 flex items-center justify-between gap-2">
                    <p className={inspectorTw.signatureMeta} title={fieldTypeId}>
                      {fieldTypeId}
                    </p>
                    <span
                      className="overflow-hidden text-ellipsis whitespace-nowrap text-[0.66rem] uppercase tracking-[0.08em] text-[hsl(var(--rm-text-tertiary))]"
                      title={field.visibility}
                    >
                      {field.visibility}
                    </span>
                  </div>
                </button>
              </li>
            )
          }

          return (
            <li className={inspectorTw.listItem} key={`${fieldName}-${field.type_expr}-${index}`}>
              <div className="mb-1.5 flex items-center justify-between gap-2">
                <p
                  className="overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.72rem] text-[hsl(var(--rm-text))]"
                  title={fieldName}
                >
                  {fieldName}
                </p>
                <span
                  className={inspectorTw.relationJumpHint}
                  title={field.visibility}
                >
                  {field.visibility}
                </span>
              </div>
              <p className={inspectorTw.signatureValue} title={fieldTypeLabel}>
                {fieldTypeLabel}
              </p>
              {fieldTypeId ? (
                <p className={inspectorTw.signatureMeta} title={fieldTypeId}>
                  {fieldTypeId}
                </p>
              ) : null}
            </li>
          )
        })}
      </ul>
    </div>
  )
}

function renderEnumDetails(viewModel: InspectorViewModel) {
  if (!viewModel.item || !Array.isArray(viewModel.item.enum_variants)) {
    return null
  }

  if (viewModel.item.enum_variants.length === 0) {
    return <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">No enum variants extracted.</p>
  }

  return (
    <ul className={inspectorTw.list}>
      {viewModel.item.enum_variants.map((variant) => (
        <li className={inspectorTw.listItem} key={variant.name}>
          <span
            className="block overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.72rem] text-[hsl(var(--rm-text))]"
            title={variant.name}
          >
            {variant.name}
          </span>
          <span
            className="mt-1 block overflow-hidden text-ellipsis whitespace-nowrap text-[0.68rem] uppercase tracking-[0.08em] text-[hsl(var(--rm-text-tertiary))]"
            title={variant.kind}
          >
            {variant.kind}
          </span>
        </li>
      ))}
    </ul>
  )
}

function renderTraitDetails(viewModel: InspectorViewModel) {
  if (!viewModel.item || !Array.isArray(viewModel.item.trait_methods)) {
    return null
  }

  if (viewModel.item.trait_methods.length === 0) {
    return <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">No trait methods extracted.</p>
  }

  return (
    <ul className={inspectorTw.list}>
      {viewModel.item.trait_methods.map((methodEntry, methodIndex) => {
        const method =
          typeof methodEntry === 'string'
            ? {
                name: methodEntry,
                receiver: undefined,
                fn_signature: undefined,
              }
            : methodEntry

        const params = method.fn_signature?.params ?? []
        const returnType = method.fn_signature?.return_type
        const receiver = method.receiver

        return (
          <li className={inspectorTw.listItem} key={`${method.name}-${methodIndex}`}>
            <span
              className="block overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.72rem] text-[hsl(var(--rm-text))]"
              title={method.name}
            >
              {method.name}
            </span>

            {receiver ? (
              <div className="mt-2">
                <p className={inspectorTw.signatureKey}>Receiver</p>
                <p className={inspectorTw.signatureMeta} title={receiver}>
                  {receiver}
                </p>
              </div>
            ) : null}

            <div className="mt-2">
              <p className={inspectorTw.signatureKey}>Params</p>
              {params.length === 0 ? (
                <p className={inspectorTw.monoLine} title="(none)">
                  (none)
                </p>
              ) : (
                <ul className="mt-1 space-y-2">
                  {params.map((param, index) => {
                    const paramName = param.name ?? `_arg${index + 1}`
                    const paramDisplay = `${paramName}: ${param.type_expr}`
                    return (
                      <li key={`${method.name}-${paramName}-${index}`} className="min-w-0 overflow-hidden">
                        <p className={inspectorTw.signatureValue} title={paramDisplay}>
                          {paramDisplay}
                        </p>
                        {param.type_id ? (
                          <p className={inspectorTw.signatureMeta} title={param.type_id}>
                            {param.type_id}
                          </p>
                        ) : null}
                      </li>
                    )
                  })}
                </ul>
              )}
            </div>

            <div className="mt-2">
              <p className={inspectorTw.signatureKey}>Return</p>
              {returnType ? (
                <>
                  <p className={inspectorTw.signatureValue} title={returnType.type_expr}>
                    {returnType.type_expr}
                  </p>
                  {returnType.type_id ? (
                    <p className={inspectorTw.signatureMeta} title={returnType.type_id}>
                      {returnType.type_id}
                    </p>
                  ) : null}
                </>
              ) : (
                <p className={inspectorTw.monoLine} title="()">
                  ()
                </p>
              )}
            </div>
          </li>
        )
      })}
    </ul>
  )
}

function renderFnSignature(viewModel: InspectorViewModel) {
  if (!viewModel.item?.fn_signature) {
    return null
  }

  const params = viewModel.item.fn_signature.params
  const returnType = viewModel.item.fn_signature.return_type

  return (
    <div className={inspectorTw.signatureGroup}>
      <div className={inspectorTw.signatureRow}>
        <p className={inspectorTw.signatureKey}>Params</p>
        {params.length === 0 ? (
          <p className={inspectorTw.monoLine} title="(none)">
            (none)
          </p>
        ) : (
          <ul className="mt-1 space-y-2">
            {params.map((param, index) => {
              const paramName = param.name ?? `_arg${index + 1}`
              const paramDisplay = `${paramName}: ${param.type_expr}`
              return (
                <li key={`${paramName}-${index}`} className="min-w-0 overflow-hidden">
                  <p className={inspectorTw.signatureValue} title={paramDisplay}>
                    {paramDisplay}
                  </p>
                  {param.type_id ? (
                    <p className={inspectorTw.signatureMeta} title={param.type_id}>
                      {param.type_id}
                    </p>
                  ) : null}
                </li>
              )
            })}
          </ul>
        )}
      </div>

      <div className={inspectorTw.signatureRow}>
        <p className={inspectorTw.signatureKey}>Return</p>
        {returnType ? (
          <>
            <p className={inspectorTw.signatureValue} title={returnType.type_expr}>
              {returnType.type_expr}
            </p>
            {returnType.type_id ? (
              <p className={inspectorTw.signatureMeta} title={returnType.type_id}>
                {returnType.type_id}
              </p>
            ) : null}
          </>
        ) : (
          <p className={inspectorTw.monoLine} title="()">
            ()
          </p>
        )}
      </div>
    </div>
  )
}

function renderMethodContext(
  viewModel: InspectorViewModel,
  nodeById: Map<string, GraphNode>,
  onNavigateRelation: (nodeId: string) => void,
) {
  if (!viewModel.item) {
    return (
      <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">
        No method ownership metadata.
      </p>
    )
  }

  const ownerId = viewModel.item.owner_id
  const ownerNode = ownerId ? nodeById.get(ownerId) : null
  const ownerTitle =
    ownerNode && ownerId
      ? `${ownerNode.label} (${ownerNode.kind}) · ${ownerId}`
      : ownerId ?? 'Unavailable'

  return (
    <div className={inspectorTw.signatureGroup}>
      <div className={inspectorTw.signatureRow}>
        <p className={inspectorTw.signatureKey}>Owner</p>
        {ownerId ? (
          <button
            type="button"
            onClick={() => {
              onNavigateRelation(ownerId)
            }}
            className={cn(inspectorTw.relationCard, 'mt-2 px-2.5 py-2')}
            disabled={!ownerNode}
            title={ownerTitle}
          >
            <div className={inspectorTw.relationCardTop}>
              <p className={inspectorTw.relationTitle}>
                {ownerNode ? `${ownerNode.label} (${ownerNode.kind})` : ownerId}
              </p>
              <span className={inspectorTw.relationJumpHint}>
                {ownerNode ? 'Jump' : 'N/A'}
                {ownerNode ? <ArrowRight className="h-3 w-3" /> : null}
              </span>
            </div>
            <p className="overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.68rem] text-[hsl(var(--rm-text-secondary))]">
              {ownerId}
            </p>
          </button>
        ) : (
          <p className={inspectorTw.monoLine}>Unavailable</p>
        )}
      </div>

      <div className={inspectorTw.signatureRow}>
        <p className={inspectorTw.signatureKey}>Source</p>
        <p className={inspectorTw.signatureValue}>{viewModel.item.source ?? 'unknown'}</p>
      </div>

      <div className={inspectorTw.signatureRow}>
        <p className={inspectorTw.signatureKey}>Owner kind</p>
        <p className={inspectorTw.signatureValue}>{viewModel.item.owner_kind ?? 'unknown'}</p>
      </div>

      {viewModel.item.trait_id ? (
        <div className={inspectorTw.signatureRow}>
          <p className={inspectorTw.signatureKey}>Trait</p>
          <p className={inspectorTw.signatureMeta} title={viewModel.item.trait_id}>
            {viewModel.item.trait_id}
          </p>
        </div>
      ) : null}
    </div>
  )
}

function renderRelationList(
  relations: RelationSummaryItem[],
  onNavigateRelation: (nodeId: string) => void,
) {
  return (
    <ul className={inspectorTw.list}>
      {relations.slice(0, 12).map((relation, index) => (
        <li key={`${relation.peerId}-${relation.edgeKind}-${index}`} className="min-w-0">
          <button
            type="button"
            className={inspectorTw.relationCard}
            disabled={relation.peerKind === 'unknown'}
            onClick={() => {
              onNavigateRelation(relation.peerId)
            }}
            title={relation.peerKind === 'unknown' ? 'Target node is unavailable' : relation.peerId}
          >
            <div className={inspectorTw.relationCardTop}>
              <p className={inspectorTw.relationTitle} title={relation.relationLabel}>
                {relation.relationLabel}
              </p>
              <span className={inspectorTw.relationJumpHint}>
                {relation.peerKind === 'unknown' ? 'N/A' : 'Jump'}
                {relation.peerKind === 'unknown' ? null : <ArrowRight className="h-3 w-3" />}
              </span>
            </div>
            <p
              className="overflow-hidden text-ellipsis whitespace-nowrap font-mono text-[0.68rem] text-[hsl(var(--rm-text-secondary))]"
              title={`${relation.peerId} (${relation.peerKind})`}
            >
              {relation.peerId} ({relation.peerKind})
            </p>
            <p
              className="mt-1 overflow-hidden text-ellipsis whitespace-nowrap text-[0.66rem] uppercase tracking-[0.08em] text-[hsl(var(--rm-text-tertiary))]"
              title={relation.sourceContext}
            >
              {relation.sourceContext}
            </p>
          </button>
        </li>
      ))}
    </ul>
  )
}

export function InspectorDrawer({
  open,
  viewModel,
  canGoBack,
  onGoBack,
  onNavigateRelation,
  nodeById,
  callViewMode,
  callViewRootNodeId,
  onOpenCallGraph,
  onOpenStructMethodView,
  onClose,
}: InspectorDrawerProps) {
  const [copyState, setCopyState] = useState<'idle' | 'done' | 'failed'>('idle')

  const location = viewModel?.location ?? null
  const vscodeUri = location ? buildVscodeFileUri(location) : null
  const selectedNodeKind = viewModel?.node.kind ?? null
  const selectedNodeId = viewModel?.node.id ?? null
  const canOpenCallGraph = selectedNodeKind ? isCallableNodeKind(selectedNodeKind) : false
  const callGraphButtonDisabled =
    !selectedNodeId ||
    !canOpenCallGraph ||
    (callViewMode === 'call_subgraph' && callViewRootNodeId === selectedNodeId)
  const callGraphButtonText =
    callViewMode === 'call_subgraph' && callViewRootNodeId === selectedNodeId
      ? 'In Call Graph'
      : 'Open Call Graph'
  const callGraphButtonTitle =
    selectedNodeId ?? 'Select a function or method node first.'

  const relationLabelSummary = useMemo(() => {
    if (!viewModel) {
      return []
    }

    return Object.entries(viewModel.relationCountByLabel).sort(([left], [right]) =>
      left.localeCompare(right),
    )
  }, [viewModel])
  const structTraitRelations = useMemo(() => {
    if (!viewModel || viewModel.node.kind !== 'struct') {
      return []
    }

    const uniqueRelations: RelationSummaryItem[] = []
    const dedupe = new Set<string>()

    for (const relation of viewModel.relations) {
      if (relation.edgeKind !== 'impl' && relation.edgeKind !== 'inherit') {
        continue
      }

      const key = `${relation.peerId}::${relation.edgeKind}`
      if (dedupe.has(key)) {
        continue
      }
      dedupe.add(key)
      uniqueRelations.push(relation)
    }

    return uniqueRelations
  }, [viewModel])
  const traitRelationSummary = useMemo(() => {
    const countByLabel = structTraitRelations.reduce<Record<string, number>>((acc, relation) => {
      acc[relation.relationLabel] = (acc[relation.relationLabel] ?? 0) + 1
      return acc
    }, {})

    return Object.entries(countByLabel).sort(([left], [right]) => left.localeCompare(right))
  }, [structTraitRelations])

  const handleCopyLocation = async () => {
    if (!location) {
      return
    }

    try {
      const copied = await copyLocation(location)
      setCopyState(copied ? 'done' : 'failed')
    } catch {
      setCopyState('failed')
    }

    setTimeout(() => {
      setCopyState('idle')
    }, 1500)
  }

  return (
    <aside className={cn(inspectorTw.root, !open && 'pointer-events-none')}>
      <header className={inspectorTw.header}>
        <div className={inspectorTw.titleWrap}>
          <p className={inspectorTw.title}>{viewModel?.node.label ?? 'Inspector'}</p>
          <p className={inspectorTw.subtitle}>{viewModel?.node.kind ?? 'No active node'}</p>
        </div>
        <div className={inspectorTw.headerActions}>
          {canOpenCallGraph ? (
            <button
              type="button"
              onClick={() => {
                if (selectedNodeId) {
                  onOpenCallGraph(selectedNodeId)
                }
              }}
              disabled={callGraphButtonDisabled}
              className={cn(buttonVariants({ variant: 'surface' }), 'h-8 px-2.5 py-1 text-[0.68rem]')}
              title={callGraphButtonTitle}
            >
              {callGraphButtonText}
            </button>
          ) : null}
          <button
            type="button"
            onClick={onGoBack}
            disabled={!canGoBack}
            className={cn(iconButtonVariants({ variant: 'ghost' }))}
            aria-label="Go back in relation history"
          >
            <ChevronLeft className="h-4 w-4" />
          </button>
          <button
            type="button"
            onClick={onClose}
            className={cn(iconButtonVariants({ variant: 'ghost' }))}
            aria-label="Close inspector"
          >
            <X className="h-4 w-4" />
          </button>
        </div>
      </header>

      <div className={inspectorTw.body}>
        {!viewModel ? (
          <section className={cn(panelVariants({ tone: 'muted' }), 'px-3 py-3')}>
            <p className="text-[0.8rem] text-[hsl(var(--rm-text-secondary))]">
              Select a node to inspect metadata, relation context, and source location.
            </p>
          </section>
        ) : (
          <>
            <section className={inspectorTw.section}>
              <p className={inspectorTw.sectionTitle}>Source</p>
              {location ? (
                <div className="space-y-2">
                  <div className={inspectorTw.monoBlock}>
                    <p
                      className={inspectorTw.monoLine}
                      title={locationToClipboardText(location)}
                    >
                      {locationToClipboardText(location)}
                    </p>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    <a
                      href={vscodeUri ?? '#'}
                      className={cn(buttonVariants({ variant: 'surface' }))}
                    >
                      <ExternalLink className="h-3.5 w-3.5" />
                      Open in VS Code
                    </a>
                    <button
                      type="button"
                      onClick={handleCopyLocation}
                      className={cn(buttonVariants({ variant: 'surface' }))}
                    >
                      <Copy className="h-3.5 w-3.5" />
                      {copyState === 'done'
                        ? 'Copied'
                        : copyState === 'failed'
                          ? 'Copy failed'
                          : 'Copy path'}
                    </button>
                  </div>
                </div>
              ) : (
                <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">
                  This node does not expose source path/span metadata.
                </p>
              )}
            </section>

            {viewModel.node.kind === 'struct' ? (
              <section className={inspectorTw.section}>
                <p className={inspectorTw.sectionTitle}>Struct Fields</p>
                {renderStructFieldDetails(viewModel, nodeById, onNavigateRelation)}
              </section>
            ) : null}

            {viewModel.node.kind === 'struct' ? (
              <section className={inspectorTw.section}>
                <p className={inspectorTw.sectionTitle}>Trait Implement</p>
                {traitRelationSummary.length === 0 ? (
                  <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">
                    No trait implementation relation.
                  </p>
                ) : (
                  <div className="mb-2 flex flex-wrap gap-2">
                    {traitRelationSummary.map(([label, count]) => (
                      <span
                        key={label}
                        title={`${label} ${count}`}
                        className="inline-flex rounded-full border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface-muted))] px-2 py-1 text-[0.68rem] font-semibold uppercase tracking-[0.08em] text-[hsl(var(--rm-text-secondary))]"
                      >
                        {label} {count}
                      </span>
                    ))}
                  </div>
                )}
                {structTraitRelations.length > 0
                  ? renderRelationList(structTraitRelations, onNavigateRelation)
                  : null}
              </section>
            ) : (
              <section className={inspectorTw.section}>
                <p className={inspectorTw.sectionTitle}>Downstream Relations</p>
                {relationLabelSummary.length === 0 ? (
                  <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">No direct relation.</p>
                ) : (
                  <div className="mb-2 flex flex-wrap gap-2">
                    {relationLabelSummary.map(([label, count]) => (
                      <span
                        key={label}
                        title={`${label} ${count}`}
                        className="inline-flex rounded-full border border-[hsl(var(--rm-border))] bg-[hsl(var(--rm-surface-muted))] px-2 py-1 text-[0.68rem] font-semibold uppercase tracking-[0.08em] text-[hsl(var(--rm-text-secondary))]"
                      >
                        {label} {count}
                      </span>
                    ))}
                  </div>
                )}
                {renderRelationList(viewModel.relations, onNavigateRelation)}
              </section>
            )}

            {viewModel.node.kind === 'struct' ? (
              <section className={inspectorTw.section}>
                <p className={inspectorTw.sectionTitle}>Struct Methods</p>
                {renderStructDetails(viewModel, onOpenStructMethodView)}
              </section>
            ) : null}

            {viewModel.node.kind === 'enum' ? (
              <section className={inspectorTw.section}>
                <p className={inspectorTw.sectionTitle}>Enum Variants</p>
                {renderEnumDetails(viewModel)}
              </section>
            ) : null}

            {viewModel.node.kind === 'trait' ? (
              <section className={inspectorTw.section}>
                <p className={inspectorTw.sectionTitle}>Trait Methods</p>
                {renderTraitDetails(viewModel)}
              </section>
            ) : null}

            {viewModel.node.kind === 'method' ? (
              <section className={inspectorTw.section}>
                <p className={inspectorTw.sectionTitle}>Method Context</p>
                {renderMethodContext(viewModel, nodeById, onNavigateRelation)}
              </section>
            ) : null}

            {viewModel.node.kind === 'fn' || viewModel.node.kind === 'method' ? (
              <section className={inspectorTw.section}>
                <p className={inspectorTw.sectionTitle}>
                  {viewModel.node.kind === 'method' ? 'Method Signature' : 'Function Signature'}
                </p>
                {renderFnSignature(viewModel) ?? (
                  <p className="text-[0.75rem] text-[hsl(var(--rm-text-tertiary))]">
                    No function signature metadata.
                  </p>
                )}
              </section>
            ) : null}

            {viewModel.warnings.length > 0 ? (
              <section className={inspectorTw.section}>
                <p className={inspectorTw.sectionTitle}>Node Warnings</p>
                <ul className={inspectorTw.list}>
                  {viewModel.warnings.map((warning, index) => (
                    <li className={inspectorTw.listItem} key={`${warning.message}-${index}`}>
                      <p
                        className="overflow-hidden text-ellipsis whitespace-nowrap text-[0.72rem] leading-5 text-[hsl(var(--rm-text-secondary))]"
                        title={`[${warning.severity}] ${warning.message}`}
                      >
                        [{warning.severity}] {warning.message}
                      </p>
                    </li>
                  ))}
                </ul>
              </section>
            ) : null}
          </>
        )}
      </div>
    </aside>
  )
}
