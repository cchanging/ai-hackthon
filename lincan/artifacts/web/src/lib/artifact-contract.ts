import { z } from 'zod'

const spanSchema = z
  .object({
    start_line: z.number().int().nonnegative(),
    start_col: z.number().int().nonnegative(),
    end_line: z.number().int().nonnegative(),
    end_col: z.number().int().nonnegative(),
  })
  .strict()

const structMethodSchema = z
  .object({
    name: z.string().min(1),
    source: z.enum(['inherent', 'trait']).optional(),
    trait_id: z.string().min(1).optional(),
    method_id: z.string().min(1).optional(),
  })
  .passthrough()

const structFieldSchema = z
  .object({
    name: z.string().min(1).optional(),
    index: z.number().int().nonnegative().optional(),
    visibility: z.string().min(1),
    type_expr: z.string().min(1),
    type_id: z.string().min(1).optional(),
  })
  .passthrough()

const implDetailSchema = z
  .object({
    trait_id: z.string().min(1),
    trait_path: z.string().min(1),
  })
  .passthrough()

const enumVariantSchema = z
  .object({
    name: z.string().min(1),
    kind: z.string().min(1),
    fields: z.array(z.unknown()),
  })
  .passthrough()

const fnParamSchema = z
  .object({
    name: z.string().min(1).optional().nullable(),
    type_expr: z.string().min(1),
    type_id: z.string().min(1).optional(),
  })
  .passthrough()

const fnReturnSchema = z
  .object({
    type_expr: z.string().min(1),
    type_id: z.string().min(1).optional(),
  })
  .passthrough()

const fnSignatureSchema = z
  .object({
    params: z.array(fnParamSchema),
    return_type: fnReturnSchema.optional(),
  })
  .passthrough()

const traitMethodSchema = z
  .object({
    name: z.string().min(1),
    receiver: z.string().min(1).optional(),
    fn_signature: fnSignatureSchema,
  })
  .passthrough()

export const rustItemSchema = z
  .object({
    id: z.string().min(1),
    kind: z.string().min(1),
    name: z.string().min(1),
    container: z.string().min(1),
    file: z.string().min(1).optional(),
    span: spanSchema.optional(),
    struct_shape: z.enum(['named', 'tuple', 'unit']).optional(),
    struct_fields: z.array(structFieldSchema).optional(),
    methods: z.array(structMethodSchema).optional(),
    impl_details: z.array(implDetailSchema).optional(),
    enum_variants: z.array(enumVariantSchema).optional(),
    trait_methods: z
      .array(z.union([z.string().min(1), traitMethodSchema]))
      .optional(),
    fn_signature: fnSignatureSchema.optional(),
    owner_id: z.string().min(1).optional(),
    owner_kind: z.string().min(1).optional(),
    source: z.enum(['inherent', 'trait']).optional(),
    trait_id: z.string().min(1).optional(),
  })
  .passthrough()

const moduleSchema = z
  .object({
    id: z.string().optional(),
    name: z.string().optional(),
    path: z.string().optional(),
    items: z.array(rustItemSchema),
  })
  .passthrough()

const crateSchema = z
  .object({
    id: z.string().optional(),
    name: z.string().optional(),
    version: z.string().optional(),
    modules: z.array(moduleSchema),
  })
  .passthrough()

const graphNodeSchema = z
  .object({
    id: z.string().min(1),
    kind: z.string().min(1),
    label: z.string().min(1),
  })
  .strict()

const graphEdgeSchema = z
  .object({
    kind: z.enum(['impl', 'use', 'inherit', 'call', 'contain']),
    from: z.string().min(1),
    to: z.string().min(1),
    source_context: z.string().min(1),
  })
  .strict()

const graphEdgeRefSchema = z
  .object({
    from: z.string().min(1),
    to: z.string().min(1),
  })
  .strict()

const graphIndexSchema = z
  .object({
    nodes: z.array(graphNodeSchema),
    edges: z.array(graphEdgeSchema),
    by_kind: z.record(z.string(), z.array(z.string().min(1))),
    by_container: z.record(z.string(), z.array(z.string().min(1))),
    by_edge_kind: z.record(z.string(), z.array(graphEdgeRefSchema)),
  })
  .strict()

const dependencySchema = z
  .object({
    name: z.string().min(1),
    version: z.string().min(1),
  })
  .strict()

const warningSchema = z
  .object({
    severity: z.enum(['warn']),
    message: z.string().min(1),
    code: z.string().optional(),
    item_id: z.string().optional(),
  })
  .passthrough()

const workspaceSchema = z
  .object({
    root: z.string().min(1),
    members: z.array(z.string().min(1)),
  })
  .passthrough()

export const artifactContractSchema = z
  .object({
    workspace: workspaceSchema,
    dependencies: z.array(dependencySchema),
    crates: z.array(crateSchema),
    graph_index: graphIndexSchema,
    warnings: z.array(warningSchema),
  })
  .strict()

export type ArtifactContract = z.infer<typeof artifactContractSchema>
export type GraphNode = z.infer<typeof graphNodeSchema>
export type GraphEdge = z.infer<typeof graphEdgeSchema>
export type WarningItem = z.infer<typeof warningSchema>
export type RustItem = z.infer<typeof rustItemSchema>

export function formatArtifactContractError(error: z.ZodError): string {
  if (error.issues.length === 0) {
    return 'JSON contract validation failed with unknown error.'
  }

  const issueLines = error.issues.slice(0, 6).map((issue, index) => {
    const path = issue.path.length > 0 ? issue.path.join('.') : 'root'
    return `${index + 1}. ${path}: ${issue.message}`
  })

  const hiddenCount = error.issues.length - issueLines.length
  const suffix =
    hiddenCount > 0 ? `\n...and ${hiddenCount} more contract issue(s).` : ''

  return `JSON contract validation failed:\n${issueLines.join('\n')}${suffix}`
}
