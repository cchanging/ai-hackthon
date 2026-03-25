# Codex Agents Engineering Practice Summary

## 1. Goals and Principles

When using Codex Agents for software engineering, the core objective is not to "make the model write more code," but to "continuously deliver verifiable incremental value under controlled constraints." In practice, this can be summarized into three principles:

1. Constraints first: scope, contract, and acceptance criteria must be defined before implementation begins.
2. Automation first: testing, build, and static checks should serve as default quality gates.
3. Iterate in small steps: decompose complex tasks into independently verifiable subtasks to reduce rework cost.

## 2. Effective Practices

### 2.1 Use Plan Mode for Task Governance

The value of Plan Mode is that it converts ideas into executable steps and makes risk management explicit.

1. In the planning stage, explicitly define four categories: Scope, Non-goals, Contract, and Acceptance.
2. Each step must produce verifiable outputs (code changes, test results, or documentation updates).
3. Stop and clarify immediately when blocked; do not continue under incorrect assumptions.

### 2.2 Improve Output Quality Through High-Quality Inputs

Inputs to agents should use structured constraints rather than conversational vision statements. High-quality inputs typically include:

1. Context and business objectives.
2. Clear boundaries (what to do and what not to do).
3. Concrete interfaces or data contracts.
4. Acceptance criteria and failure conditions.
5. Execution environment and command baseline.

For design tasks, first provide sketches or reference screenshots, then use a dedicated Designer Agent to produce a structured design specification, and finally hand it over to a Frontend/Rust Worker for implementation. This is significantly more effective than one-shot mixed instructions.

### 2.3 Multi-Agent Collaboration and Worktree Isolation

The key in multi-agent collaboration is not "the more parallelism, the better," but "clear responsibility boundaries and non-overlapping write scopes."

1. The Main Agent is responsible for global orchestration, contract enforcement, and final integration.
2. The Worker Agent is responsible for implementation within clearly scoped modules.
3. The Reviewer Agent is responsible for defect identification and regression risk detection.

### 2.4 Process Control Centered on Git

Git is not only a versioning tool; it is also a collaboration control surface. Effective practices include:

1. Keep each milestone commit single-purpose, with traceable commit messages.
2. Consistently enforce `fmt/lint/test/build` gates before merge.
3. Review based on diffs, prioritizing behavioral regressions and contract violations.

## 3. Common Issues and Mitigation Strategies

### 3.1 Loss of Initial Constraints Due to Insufficient Context

Symptoms:

1. The task drifts from the original scope in mid-to-late execution.
2. Outputs become inconsistent with the agreed contract.

Mitigation strategies:

1. Use `AGENTS.md` as the Single Source of Truth.
2. Before each key execution round, require the agent to restate current constraints and acceptance criteria.
3. Set stage checkpoints for long tasks to enforce "scope verification + risk review."
4. Place critical constraints at the beginning of prompts to avoid being diluted in long context.

### 3.2 Inability to Break Existing Structure in Secondary Development, Leading to Technical Debt Accumulation

Symptoms:

1. Patches keep accumulating under the premise of "minimal changes."
2. Local behavior works, but global complexity continues to rise.

Mitigation strategies:

1. Decompose behavior first, then decompose code: define target behavior before deciding structural changes.
2. Introduce a "refactoring budget": explicitly define the modules, interfaces, and test scope allowed to change.
3. Apply gradual replacement (Strangler Pattern): keep old and new paths in parallel and migrate incrementally.
4. Add tests before refactoring: structural changes without regression protection are high risk.
5. Use a Reviewer Agent to specifically inspect "complexity growth points" and "implicit coupling."

### 3.3 Conflicting Results from Parallel Agents

Symptoms:

1. Different agents modify the same file and create conflicts.
2. Locally optimal solutions are mutually incompatible.

Mitigation strategies:

1. Define file-level ownership during task assignment.
2. Define shared contracts (types, naming, error semantics) before parallel execution.
3. Let the Main Agent perform final integration; do not allow sub-agents to overwrite each other's changes.
