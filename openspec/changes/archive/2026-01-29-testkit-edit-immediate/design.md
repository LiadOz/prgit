## Context

The p4rs testkit provides helper methods on `ChangelistBuilder` to simplify test setup. The `edit_file` method queues a P4 edit operation and writes content to the file. However, P4 syncs files as read-only, and the queued edit doesn't execute until `submit()`, causing the immediate write to fail.

## Goals / Non-Goals

**Goals:**
- Content-providing edit helpers execute P4 edit immediately so files are writable
- Maintain the fluent API pattern for test setup

**Non-Goals:**
- Changing the default batching behavior of `ChangelistBuilder`
- Modifying non-content-providing helpers like `edit()`

## Decisions

**Execute P4 edit directly before writing content**

In `edit_file` and `edit_file_with_opts`, call the P4 edit command directly (not through the batching queue) before writing content. This ensures only the edit runs immediately, leaving other queued operations untouched.

Alternative considered: Using `flush()` - rejected because it would execute all pending operations, not just the edit.

Alternative considered: Using `immediate()` mode - rejected because it would change behavior for the entire builder chain.

## Risks / Trade-offs

**Extra P4 round-trip** → Acceptable for test code where correctness matters more than performance.
