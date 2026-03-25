# Commands

## 1. 枚举所有依赖 `ostd` 的 workspace crate

```bash
cd /root/asterinas-codex

python3 - <<'PY'
import json, subprocess
meta = json.loads(subprocess.check_output(['cargo','metadata','--format-version','1'], cwd='/root/asterinas-codex'))
workspace = set(meta['workspace_members'])
packages = {p['id']: p for p in meta['packages']}
name_to_ids = {}
for p in meta['packages']:
    name_to_ids.setdefault(p['name'], []).append(p['id'])
rev = {pid: [] for pid in workspace}
for pid in workspace:
    p = packages[pid]
    for dep in p['dependencies']:
        targets = []
        if dep.get('path'):
            for qid in workspace:
                if packages[qid]['manifest_path'].startswith(dep['path'].rstrip('/') + '/') or packages[qid]['manifest_path'] == dep['path'].rstrip('/') + '/Cargo.toml':
                    targets.append(qid)
        if not targets:
            targets = [qid for qid in name_to_ids.get(dep['name'], []) if qid in workspace]
        for qid in targets:
            rev[qid].append(pid)
roots = [qid for qid in workspace if packages[qid]['name'] == 'ostd']
seen = set(roots)
stack = list(roots)
while stack:
    cur = stack.pop()
    for parent in rev.get(cur, []):
        if parent not in seen:
            seen.add(parent)
            stack.append(parent)
selected = sorted((packages[pid]['name'] for pid in seen))
print('\\n'.join(selected))
print('COUNT', len(selected))
PY
```

本次得到的包集合是：

- `aster-bigtcp`
- `aster-block`
- `aster-cmdline`
- `aster-console`
- `aster-framebuffer`
- `aster-i8042`
- `aster-input`
- `aster-kernel`
- `aster-logger`
- `aster-mlsdisk`
- `aster-network`
- `aster-pci`
- `aster-softirq`
- `aster-systree`
- `aster-time`
- `aster-uart`
- `aster-util`
- `aster-virtio`
- `device-id`
- `osdk-frame-allocator`
- `osdk-heap-allocator`
- `osdk-test-kernel`
- `ostd`
- `xarray`

## 2. 运行 `lockdep` 仓库级扫描

```bash
cd /root/asterinas-codex

cargo run --manifest-path tools/lockdep/Cargo.toml --bin cargo-lockdep -- \
  -p aster-bigtcp -p aster-block -p aster-cmdline -p aster-console -p aster-framebuffer -p aster-i8042 \
  -p aster-input -p aster-kernel -p aster-logger -p aster-mlsdisk -p aster-network -p aster-pci \
  -p aster-softirq -p aster-systree -p aster-time -p aster-uart -p aster-util -p aster-virtio \
  -p device-id -p osdk-frame-allocator -p osdk-heap-allocator -p osdk-test-kernel -p ostd -p xarray \
  --target x86_64-unknown-none \
  -- --quiet
```
