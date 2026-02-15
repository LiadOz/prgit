---
alwaysApply: True
---

- This is a rust project.
- The project is divided into components, each component is managed by a different agent, that agent can only see the code of the component it is managing. Meaning sometimes you may not see the full code of the project.
- The project aims at 100% test coverage, all the code should be covered by tests, this is checked by a pipeline.
- testing formatting and cargo commands cannot run inside components, to do that you may run with `prgit_run <command> <args>`
- The `openspec` CLI requires network permissions to run properly (it silently fails without network access). Always use `required_permissions: ["network"]` when running openspec commands.
- If an openspec command fails, do NOT attempt to do what that command was supposed to do manually (e.g., don't manually move/create directories if `openspec new` or `openspec archive` fails). Report the failure to the user instead.