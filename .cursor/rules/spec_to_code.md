---
alwaysApply: False
---

- specs are written in ./docs/spec/**/<spec_name>.md
- Start by writing the simplest possible implementation that satisfies the spec.
- Code should be easily testable, prefer dependency injection so mocking is not needed.
- make sure to run `prgit_run cargo fmt` to format the code before testing.
- To check code run `prgit_run cargo make ci`