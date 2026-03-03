## Context

The testkit provides a `P4Server` that starts a p4d Docker container and `TestClient` for isolated test workspaces. Currently there are no protections configured, so all users have implicit super access.

P4 security level 0 (default) allows users to be auto-created on first connection and doesn't require passwords. However, once protections are set, they restrict what operations users can perform.

## Goals / Non-Goals

**Goals:**
- Tests run as non-admin users by default
- Clear way to get admin access when needed
- Implement `protect` command for managing protection table
- Add typed error for permission denied scenarios

**Non-Goals:**
- Changing p4d security level (stays at 0)
- Password authentication (not enforced at security level 0)
- Complex protection schemes (simple admin/regular user split)

## Decisions

**Admin credentials as constants**

```rust
pub const ADMIN_USER: &str = "admin";
pub const ADMIN_PASSWORD: &str = "admin123";
```

Password is included for completeness even though security level 0 doesn't enforce it.

**Protection table setup**

On server start, before any tests run:
1. Connect as "admin" user (auto-created at security level 0)
2. Set protection table:
   ```
   super user admin * //...
   write user * * //...
   ```

This gives admin super access and everyone else write access (can read/write files but not admin operations).

**admin_p4 function signature**

```rust
pub fn admin_p4(p4: &P4) -> P4 {
    p4.clone().p4_user(ADMIN_USER).password(ADMIN_PASSWORD)
}
```

Takes any P4 object (to preserve port/connection settings) and returns a new P4 configured with admin credentials.

**protect command interface**

```rust
p4.protect().get().run()?;           // p4 protect -o
p4.protect().set(&table).run()?;     // p4 protect -i
```

ProtectionTable struct:
```rust
pub struct ProtectionTable {
    pub protections: Vec<Protection>,
}

pub struct Protection {
    pub access: AccessLevel,      // super, admin, write, read, list, ...
    pub kind: ProtectionKind,     // user, group
    pub name: String,             // username or group name
    pub host: String,             // host pattern
    pub path: String,             // depot path pattern
}
```

**PermissionDenied error**

Add to P4Error enum:
```rust
#[error("Permission denied: {0}")]
PermissionDenied(String),
```

Detection: Parse error messages for "don't have permission" or similar P4 patterns.

## Risks / Trade-offs

**Protections add test setup time** → Minimal impact, one-time setup per server start.

**Some tests may need admin access** → Use `admin_p4()` explicitly. This makes it clear which operations need elevated privileges.

**Error detection is string-based** → P4 doesn't have structured error codes for permissions. We match on error message patterns, which could break if P4 changes messages.
