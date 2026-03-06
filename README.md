# prgit

A bidirectional bridge between Perforce and Git. P4 submitted changes mirror into git commits; git branch pushes shelve back into P4.

## The problem

Your team uses Perforce. It handles large repos, binary assets, and fine-grained access control well. But developers want to use git.

Not because git is better at version control — but because git's authoring experience is better. Local branches are instant. You can stage, stash, rebase, and iterate without touching a server. You can work offline. And the entire modern tooling ecosystem — IDEs, code review, CI, AI assistants — assumes git.

## What prgit does

prgit gives your team git for the authoring loop while keeping Perforce as the system of record. You write code in git, push a branch, and it becomes a shelved changelist in P4 that goes through your normal review and submit process. No migration, no new workflows on the P4 side, no special commands to learn. Just git.

Most users only need to know the submission side of P4 — how to review a shelved changelist and submit it. You don't need to understand P4 branches, client mappings, streams, or integration. Those are concerns for the people setting up the bridge and the integration specialists, not for the developer writing code.

## The prgit flow

You use git commands to do P4 things:

- `git clone` — get the repo (mirrored from P4 submitted changes)
- `git pull` — get latest (new P4 submissions mirrored to git)
- `git push` — shelve your branch in P4
- Review and submit happen in P4, through your existing process

Multiple users share one git repo. Everyone sees each other's branches. Shelved changelists are tied to branches — push again and the shelve updates.

## Further reading

- [Why prgit and not other tools](docs/why-prgit.md)
