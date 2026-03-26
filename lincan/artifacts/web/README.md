# React + TypeScript + Vite

This template provides a minimal setup to get React working in Vite with HMR and some ESLint rules.

Currently, two official plugins are available:

- [@vitejs/plugin-react](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react) uses [Oxc](https://oxc.rs)
- [@vitejs/plugin-react-swc](https://github.com/vitejs/vite-plugin-react/blob/main/packages/plugin-react-swc) uses [SWC](https://swc.rs/)

## React Compiler

The React Compiler is enabled on this template. See [this documentation](https://react.dev/learn/react-compiler) for more information.

Note: This will impact Vite dev & build performances.

## Expanding the ESLint configuration

If you are developing a production application, we recommend updating the configuration to enable type-aware lint rules:

```js
export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      // Other configs...

      // Remove tseslint.configs.recommended and replace with this
      tseslint.configs.recommendedTypeChecked,
      // Alternatively, use this for stricter rules
      tseslint.configs.strictTypeChecked,
      // Optionally, add this for stylistic rules
      tseslint.configs.stylisticTypeChecked,

      // Other configs...
    ],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.node.json', './tsconfig.app.json'],
        tsconfigRootDir: import.meta.dirname,
      },
      // other options...
    },
  },
])
```

You can also install [eslint-plugin-react-x](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-x) and [eslint-plugin-react-dom](https://github.com/Rel1cx/eslint-react/tree/main/packages/plugins/eslint-plugin-react-dom) for React-specific lint rules:

```js
// eslint.config.js
import reactX from 'eslint-plugin-react-x'
import reactDom from 'eslint-plugin-react-dom'

export default defineConfig([
  globalIgnores(['dist']),
  {
    files: ['**/*.{ts,tsx}'],
    extends: [
      // Other configs...
      // Enable lint rules for React
      reactX.configs['recommended-typescript'],
      // Enable lint rules for React DOM
      reactDom.configs.recommended,
    ],
    languageOptions: {
      parserOptions: {
        project: ['./tsconfig.node.json', './tsconfig.app.json'],
        tsconfigRootDir: import.meta.dirname,
      },
      // other options...
    },
  },
])
```

## Artifact Contract (Parser Handoff)

RustMap web consumes parser JSON with strict validation (`src/lib/artifact-contract.ts`).

### New/Updated item fields

- `item.struct_shape`: `"named" | "tuple" | "unit"` (for `kind=struct`)
- `item.struct_fields[]`:
  - `name?`
  - `index?`
  - `visibility`
  - `type_expr`
  - `type_id?`
- `item.methods[]` now includes `method_id?` (graph node id for method node navigation)
- `item.trait_methods[]` now supports rich method metadata:
  - `name`
  - `receiver?`
  - `fn_signature` (`params`, `return_type?`)
- `item.owner_id`, `item.owner_kind`, `item.source`, `item.trait_id` are used by `kind=method` nodes
- `item.fn_signature` is available on both `kind=fn` and `kind=method`

### Graph changes

- `graph_index.nodes` can include `kind=method`
- `graph_index.edges.kind` now includes `call` and `contain`
- `graph_index.by_edge_kind.call` groups all call edges
- `graph_index.by_edge_kind.contain` groups type-reference edges
- `module_use` edges are removed; `use` imports are kept only for local symbol resolution

### Call edge semantics

- Parser emits direct call edges only (no precomputed transitive closure)
- Only workspace-resolved call targets are emitted as edges
- Unresolved/external targets are represented as warnings (`code=unresolved_call`)

### Migration notes for frontend owner

- Keep validation strict, but accept `edge.kind = "call" | "contain"` and `node.kind = "method"`
- Use `method_id` to map struct inspector method entries to method graph nodes
- Consume method metadata fields in inspector (`owner_id`, `owner_kind`, `source`, `trait_id`, `fn_signature`)
- Consume struct detail fields in inspector (`struct_shape`, `struct_fields`)
- Implement downstream-only call subgraph view:
  - depth range: `1..6`
  - default depth: `3`
  - cycle-safe traversal using visited set
  - entry points: struct method action button + fn/method inspector action
  - render only `call` edges and reachable nodes in the secondary canvas
