---
alwaysApply: True
---

- This is a rust project.
- The project is divided into components, each component is managed by a different agent, that agent can only see the code of the component it is managing. Meaning sometimes you may not see the full code of the project.
- The project aims at 100% test coverage, all the code should be covered by tests, this is checked by a pipeline.
- testing formatting and cargo commands cannot run inside components, to do that you may run with `prgit_run <command> <args>`