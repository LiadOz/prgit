## Context

ShelveClient operations sync files, add new files, and shelve changes. After completion, `p4 revert` undoes pending changes but leaves synced files in place. Added files that were reverted remain on disk as untracked files. Previous runs may not have completed cleanly.

## Goals / Non-Goals

**Goals:**
- Clean workspace at ShelveClient initialization (defensive - handles incomplete previous runs)
- Clean workspace at ShelveClient drop (proactive - keeps workspace clean for next run)
- Minimize leftover files between operations

**Non-Goals:**
- Preserving any workspace state between operations

## Decisions

**Cleanup at initialization, not completion**

Cleanup happens in `new()` because we cannot guarantee previous runs completed. This is defensive - we always start clean regardless of past state.

**Three-step cleanup sequence**

1. `p4 revert //...` - clear pending changes
2. `p4 sync //...#none` - remove synced files
3. Delete remaining directory contents - remove untracked files (e.g., reverted adds)

**Delete contents, not directory**

We delete the contents of client_root but keep the directory itself. This preserves the directory structure and any parent directory permissions.

## Risks / Trade-offs

**Extra P4 commands at startup** → Acceptable for guaranteed clean state.

**Deleting untracked files** → Could delete manually placed files, but shelve client roots should be managed exclusively by ShelveClient.
