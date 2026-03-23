## Context

The shelver's `apply_changes` method handles file edits by comparing file types derived from disk attributes (`determine_file_type`) for both the old (P4-synced) and new (git) files. When they differ, it calls `p4 reopen -t <new_type>`. The problem: `determine_file_type` creates a brand new `FileType` from scratch (text/binary/symlink + executable), discarding all P4-specific modifiers like `+C`, `+k`, `+l`.

The `FileType::Display` impl also had a bug: it output `text+x+C` instead of `text+xC`, which P4 rejects.

## Goals / Non-Goals

**Goals:**
- Preserve all P4 file type modifiers when editing files through the shelver
- Only change the executable bit and base type (text/binary/symlink) — these are the only things git tracks
- Fix `FileType::Display` to combine modifiers correctly

**Non-Goals:**
- Adding new file type detection (e.g. binary detection from content)
- Changing how `p4 add` determines types (only affects existing files via edit)

## Decisions

### 1. Query P4 for the depot file type instead of guessing from disk

**Decision:** On edit, use `p4 opened` or `p4 fstat` to get the depot file's actual type after `p4 edit`, then apply only the executable/symlink changes from git on top of it.

**Why:** After `p4 edit`, the file is opened with its depot type. We can read this type, then selectively modify only the bits git controls. Alternative considered: parsing the sync output for type info — fragile and not always available.

**Implementation:** After `p4 edit`, call `p4 opened <file>` to get the current type. Compare the executable and base-type bits against what git says. If they differ, construct a new type by starting from the depot type and toggling only the executable bit / base type. Then `reopen` with that.

### 2. FileType modifier combination

**Decision:** Already fixed — `FileType::Display` now collects all modifier characters into a single string after one `+`.

**Why:** P4 rejects `text+x+C` but accepts `text+xC`.

## Risks / Trade-offs

**[Extra P4 command per edited file]** → `p4 opened` after `p4 edit` adds one command per edited file. Mitigation: this is a local operation against the P4 workspace metadata, not a server round-trip. The overhead is negligible compared to the `p4 edit` and `p4 shelve` calls.

**[Edge case: base type changes]** → If git changes a file from text to symlink, the base type must change and modifiers may not apply. Decision: when the base type changes, use the new base type without preserving old modifiers (symlinks don't have `+C` etc.).
