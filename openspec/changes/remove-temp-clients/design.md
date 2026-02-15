## Context

ClientPool manages P4 client workspaces for shelving operations. When all pooled clients are in use and max_clients is reached, it currently creates temporary clients that are deleted when the lease is dropped. However, P4 refuses to delete clients that have pending changelists (shelves), causing these temp clients to become orphaned.

## Goals / Non-Goals

**Goals:**
- Prevent orphaned P4 clients by removing temp client creation
- Fail explicitly when pool is exhausted so callers can handle it
- Simplify the codebase by removing temp client complexity

**Non-Goals:**
- Implementing wait/retry logic (caller's responsibility)
- Auto-expanding the pool beyond max_clients

## Decisions

**Fail fast over implicit overflow**

When pool is exhausted, return `ClientPoolError::PoolExhausted` immediately. This makes resource exhaustion explicit rather than silently creating problematic temp clients.

**Remove ClientLeaseType entirely**

With only pooled clients remaining, the lease type distinction is unnecessary. All leases behave the same way - release back to pool on drop.

**Keep timeout-based reclamation**

The existing logic to reclaim timed-out clients remains useful for recovering from stuck operations without creating new clients.

## Risks / Trade-offs

**Operations may fail under load** → Acceptable. Callers can retry or users can increase max_clients. Better than accumulating orphaned clients.

**Loss of burst capacity** → The temp client mechanism provided implicit burst handling. Now requires explicit pool sizing or caller-side retry logic.
