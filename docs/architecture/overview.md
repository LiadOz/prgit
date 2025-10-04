# prgit Architecture Overview

## Purpose

prgit exposes Perforce clients as Git repositories, enabling Git workflows while maintaining Perforce as the source of truth.

## System Components

### Server
- Manages Perforce client ↔ Git repository mappings
- Exposes Git repositories accessible via standard Git protocol
- Handles bidirectional synchronization between Perforce and Git
- Implements server-side Git hooks for workflow enforcement
- REST API for client communication
- Deployment: Single multi-tenant instance or per-user instances

### Client
- CLI tool for server configuration
- Initiates new client/repository mappings
- Communicates with server via REST API

### Sync Engine
Handles bidirectional synchronization:

**Perforce → Git:**
- Imports Perforce changelists into Git master branch
- Preserves near-perfect history within client scope
- Detects changelists that originated from prgit branches
- Reuses original Git commit history for multi-user consistency

**Git → Perforce:**
- Converts Git branches into Perforce shelves
- Updates shelves on branch push
- Deletes shelves on branch deletion

**History Preservation:**
When multiple users work with the same client mapping, the sync engine ensures Git history consistency. If a changelist originated from a prgit branch, the original Git commits are reused across all users' repositories, maintaining connected history.

### Hook System
Server-side Git hooks enforce workflow:
- Blocks direct pushes to master
- Manages shelf lifecycle (create/update/delete on branch operations)
- Enforces branch → shelf → review → submit workflow

## Core Workflow

1. **Setup**: Client requests server to configure a P4 client mapping
2. **Server Init**: Creates Git repository from Perforce client
3. **Clone**: Users clone the exposed Git repository
4. **Development**: Work locally, commit to feature branches
5. **Push**: Branch push creates/updates corresponding Perforce shelf
6. **Review**: Shelf reviewed and submitted in Perforce
7. **Sync**: Server imports submitted changes to Git master
8. **Merge**: Submitted shelf merged with `-s ours` strategy, preserving branch history while keeping Perforce content canonical

## Key Design Principles

**One-to-One Mapping**: Each Perforce client maps to exactly one Git repository

**Perforce as Source of Truth**: All content changes flow through Perforce submission

**Branch History Preservation**: Git branch history remains connected via ours merges, maintaining clean Git graph

**Multi-User Consistency**: Original Git commits reused across users when syncing P4 changelists that originated from prgit

## Component Details

Detailed documentation for each component:
- [Server](./server.md) - TBD
- [Client](./client.md) - TBD
- [Sync Engine](./sync-engine.md) - TBD
- [Hook System](./hooks.md) - TBD

