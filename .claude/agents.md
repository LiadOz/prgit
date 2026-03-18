# Agent Guidelines

## Testing

When running tests in a sandbox environment, always use `--features testkit-local` to use a local p4d process instead of Docker containers (which are blocked by the sandbox).

```bash
cargo test --features testkit-local
```
