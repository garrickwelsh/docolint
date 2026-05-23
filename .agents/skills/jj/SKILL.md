---
name: jj
description: Manage jj (Jujutsu) version control with conventional commits and structured descriptions. Use when user mentions: commit, jj, description, descriptions, message, messages, branch, bookmark, change, changes, changeid, commitid, push, pull, fetch, rebase, or when writing/updating descriptions on jj changes.
---

# jj (Jujutsu) Version Control

## Preconditions

Before any workflow, verify:

```bash
which jj >/dev/null 2>&1 || { echo "jj not installed"; exit 1; }
jj workspace root >/dev/null 2>&1 || { echo "not a jj project"; exit 1; }
```

If either fails, stop and tell user.

## Workflows

### 1. Commit (user: "commit this")

1. Gather context:
   - `jj status`
   - `jj diff --git`
   - `jj log -r '@-' --no-graph` (parent change for context)
2. Read relevant source files to understand what changed and why
3. Write conventional commit message (see [REFERENCE.md](REFERENCE.md) for format)
4. Pipe message to `jj describe --stdin`
5. Run `jj new` to create empty change for next work

### 2. Set description on change (user: "set description on X", "describe change X", "update message for X")

1. Gather context:
   - `jj diff --git -r X`
   - `jj log -r X --no-graph`
   - Read relevant source files if needed
2. Write conventional commit message
3. Run `jj describe --stdin -r X`
4. Do **not** run `jj new`

### 2b. Write descriptions for multiple changes (user: "write descriptions", "write changes", "describe all changes")

1. Run `jj log -r 'draft()' --no-graph` to list un-described changes
2. For each change without a meaningful description:
   - `jj diff --git -r X`
   - Read relevant source files
   - Write conventional commit message
   - Run `jj describe --stdin -r X`
3. Do **not** run `jj new`

### 3. Read-only describe (user: "describe this change", "what does X do", "explain this diff")

1. Run `jj diff --git -r X`
2. Run `jj log -r X --no-graph`
3. Read relevant source files
4. Explain the change to user in natural language
5. **No mutation** — do not run `jj describe`

### 4. Push (user: "push")

1. Run `jj git push --tracked --deleted`
2. Show output and stderr to user

### 5. Fetch (user: "fetch", "pull")

1. Run `jj git fetch`
2. Show output and stderr to user

### 6. Bookmark operations

| User request         | Command                              |
|----------------------|--------------------------------------|
| "list bookmarks"     | `jj bookmark list`                   |
| "create bookmark X"  | `jj bookmark create X`               |
| "set bookmark X to Y"| `jj bookmark set X -r Y`             |
| "move bookmark X"    | `jj bookmark move X -r Y`            |
| "delete bookmark X"  | `jj bookmark delete X`               |

Show output to user.

### 7. Rebase (user: "rebase X onto Y")

1. Determine scope:
   - Single change only → use `-r`: `jj rebase -r X -d Y`
   - Change + all descendants (subtree) → use `-s`: `jj rebase -s X -d Y`
2. Show output and stderr to user

## Error handling

Capture stderr from all commands. If a command fails:
- Show the error output to user
- Tell user manual intervention is required
- Do not attempt automatic recovery

## Commit message format

Always use conventional commit format with structured body:

```
<type>(<scope>): <subject>

<implementation details as bullet list>

Why: <reason for change>
What: <what the change does>
```

See [REFERENCE.md](REFERENCE.md) for type reference and key jj commands.
