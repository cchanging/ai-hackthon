# Refactor Patterns

## 1. Replace generic special-casing with trait elevation

Bad pattern:
- VFS checks concrete filesystem type
- `InodeHandle` downcasts to ext2
- generic layer branches for one filesystem

Preferred fix:
- add or refine a trait/capability method
- dispatch through inode/file/superblock abstraction
- let concrete filesystems implement the behavior behind the interface

## 2. Return ownership to the state owner

Bad pattern:
- generic layer flips readonly/readwrite mode
- generic layer mutates mount-private flags
- state transitions are orchestrated outside the owning filesystem/mount object

Preferred fix:
- expose a stable owner-facing API
- keep state transitions and validation inside the owner
- let the owner reject illegal transitions with explicit errors

## 3. Replace one-test helper logic with missing contract identification

Bad pattern:
- helper added only for one xfstests case
- duplicated routing logic near syscall/VFS entry
- "temporary" conditional path

Preferred fix:
- ask what contract is missing
- add the missing trait/callback/capability
- keep the generic layer as dispatcher rather than policy holder

## 4. Preserve explicit unsupported semantics

Bad pattern:
- unsupported behavior turned into `Ok(())`
- swallowed or downgraded errors

Preferred fix:
- propagate explicit errors
- align with Linux-visible semantics
- document intentional divergence only when necessary

## 5. Prefer capability probing over concrete matching

Bad pattern:
- `if fs == ext2`
- `match concrete_fs_type`
- filesystem-name branching

Preferred fix:
- capability/interface probing through trait methods
- per-filesystem callback tables
- abstract feature support checks
