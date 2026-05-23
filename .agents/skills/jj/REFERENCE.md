# jj Reference

## Conventional Commit Types

| Type       | Use when                                  |
|------------|-------------------------------------------|
| `feat`     | New feature                               |
| `fix`      | Bug fix                                   |
| `chore`    | Maintenance, tooling, dependencies        |
| `docs`     | Documentation changes                     |
| `style`    | Formatting, whitespace, no code change    |
| `refactor` | Code restructuring, no behavior change    |
| `perf`     | Performance improvement                   |
| `test`     | Adding or updating tests                  |
| `build`    | Build system changes                      |
| `ci`       | CI/CD configuration changes               |
| `revert`   | Reverting a previous change               |

## Commit Message Structure

```
<type>(<scope>): <subject>

- <bullet of what changed - implementation details>
- <another bullet if needed>

Why: <reason for change>
What: <what the change does>
```

### Rules

- First line: `<type>(<scope>): <subject>` — max 72 chars
- Scope: optional, lowercase, single word or hyphenated
- Subject: imperative mood, no period, lowercase first letter
- Body: bullet list of implementation details
- `Why:` and `What:` labels required, both present
- Blank line between subject, body, and labels

### Example

```
feat(auth): add OIDC token refresh

- Add TokenRefreshService with retry logic
- Store refresh token in encrypted cookie
- Wire refresh interceptor into API client
- Add /auth/refresh endpoint to server

Why: tokens expire after 1h, users forced to re-login
What: auto-refresh 5min before expiry via background timer
```

## Key jj Commands

### Context gathering

| Command                            | Purpose                          |
|------------------------------------|----------------------------------|
| `jj status`                        | Show working copy state          |
| `jj diff --git`                    | Show working copy diff           |
| `jj diff --git -r X`               | Show diff for change X           |
| `jj log -r '@-' --no-graph`        | Show parent change               |
| `jj log -r X --no-graph`           | Show change X info               |
| `jj show X`                        | Show full change details         |
| `jj workspace root`                | Verify jj project root           |

### Change management

| Command                            | Purpose                          |
|------------------------------------|----------------------------------|
| `jj describe --stdin`              | Set description from stdin       |
| `jj describe --stdin -r X`         | Set description on change X      |
| `jj new`                           | Create new empty change          |
| `jj new -r X`                      | Create child of change X         |

### Sync

| Command                            | Purpose                          |
|------------------------------------|----------------------------------|
| `jj git push --tracked --deleted`  | Push tracked + deleted bookmarks |
| `jj git fetch`                     | Fetch from remote                |

### Bookmark management

| Command                            | Purpose                          |
|------------------------------------|----------------------------------|
| `jj bookmark list`                 | List all bookmarks               |
| `jj bookmark create X`             | Create bookmark X at @           |
| `jj bookmark set X -r Y`           | Set bookmark X to revision Y     |
| `jj bookmark move X -r Y`          | Move bookmark X to revision Y    |
| `jj bookmark delete X`             | Delete bookmark X                |

### Rebase

| Command                            | Purpose                          |
|------------------------------------|----------------------------------|
| `jj rebase -r X -d Y`              | Rebase only revision X onto Y    |
| `jj rebase -s X -d Y`              | Rebase X and all descendants onto Y (subtree) |

**When to use:**
- `-r` (revision): rebase a single change, leave descendants in place
- `-s` (subtree): rebase a change and its entire descendant tree — use when moving a feature branch with multiple stacked changes
