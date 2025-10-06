# Prgit 

Pargit bridges Perforce and Git. It provides a Git view of a Perforce client while keeping Perforce as the source of truth.

## Why

By exposing your Perforce client as a Git repository, you can leverage Git's workflow features missing from Perforce:

**Lightweight Git Branches**

Git branches are instantaneous to create and switch between. You can experiment with different approaches, context-switch between tasks, or work on multiple features without the overhead of multiple Perforce clients.

**Git Bisect**

You can use git bisect to binary search through your Perforce history to find which change introduced a bug.