To optimize for clarity and prevent agent instruction drift, I have condensed your workflow into a **State-Machine-based Protocol**. By prioritizing mandatory "Gates" and clear "Decision Matrices," the agent is forced to process the workflow linearly rather than getting lost in dense descriptive text.

Here is the refined source code.

***

### Part 1: The Protocol

# Trellis Development Protocol

## 1. Execution Philosophy
- **Follow Gates strictly**: Do not bypass `prd.md` or `.spec` review.
- **Agent is the executor, Human is the authority**: Never run `git commit`. 
- **Verification is non-negotiable**: Code is not "done" until all mechanical and review-style validations pass.

## 2. Decision Matrix
Start by identifying the task path. If in doubt, default to **Standard**.

| Path | Task Nature | Mandatory Actions |
| :--- | :--- | :--- |
| **A. Q&A** | Info only | Answer directly. |
| **B. Quick Fix** | Tiny, obvious, zero risk | Implement -> Verify -> Finish. |
| **C. Standard** | Non-trivial / Code change | Brainstorm -> `prd.md` -> Gate 1. |
| **D. Spec-driven** | API/Contract/Kernel/Concurrency | Brainstorm -> `prd.md` -> `.spec` -> Gate 2. |

## 3. The Lifecycle

### Step 1: Initialization
- Run `get_context.py` and `task.py list`.
- Start task: `python3 ./.trellis/scripts/task.py start <task>`

### Step 2: Planning (The Brainstorm Gate)
- Run `$brainstorm`.
- Create `prd.md` covering: Goals, Scope, Acceptance Criteria, and Spec requirement.
- **[GATE 1]**: STOP. Wait for Human Review of `prd.md`.

### Step 3: Specification (The Spec Gate)
- If Spec-driven (Path D), run `$spec-creator`.
- Must include: `LOCAL MODEL`, `ALGORITHM`, `GLOBAL INVARIANTS`, and `EDIT SCOPE`.
- **[GATE 2]**: STOP. Wait for Human Review of `.spec`.

### Step 4: Implementation
- **Must Read**: `prd.md`, `.spec`, and relevant `kernel/` or `guides/` documentation before editing.
- Implement only the approved scope.

### Step 5: Verification (The Review Gate)
1. **Mechanical**: Run `make` commands appropriate to the task.
2. **Review-style**:
   - Always: `$code-style-review` and `/trellis:finish-work`.
   - If Spec exists: `$spec-linux-validator`.
   - If Ext2/Concurrency-sensitive: `$ext2-concurrency-review`.
- **[GATE 3]**: STOP. Provide diffs and report paths to Human. Wait for Human Commit.

### Step 6: Recording
- Once Human completes the commit, finalize:
  - `add_session.py`
  - `task.py archive <task>`
  - `task.py finish`

## 4. Forbidden Actions
- **NEVER** run `git commit`.
- **NEVER** skip Brainstorm for non-trivial tasks.
- **NEVER** code before the corresponding Gate (1 or 2) is cleared.
- **NEVER** mark a task as complete without the session record.

***

### Part 2: Commands & Automation

```bash
# Task Management
python3 ./.trellis/scripts/get_context.py
python3 ./.trellis/scripts/task.py list
python3 ./.trellis/scripts/task.py start <task-slug>

# Brainstorming
/trellis:brainstorm

# Spec Creation
$spec-creator
$contract-spec

# Mandatory Verification
$code-style-review
$spec-linux-validator
$ext2-concurrency-review

# Finalization
python3 ./.trellis/scripts/add_session.py --title "<title>" --commit "<hash>" --summary "<summary>"
python3 ./.trellis/scripts/task.py archive <task-slug>
python3 ./.trellis/scripts/task.py finish
```