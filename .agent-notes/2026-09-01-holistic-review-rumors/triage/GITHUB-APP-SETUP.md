<!-- CAVEAT LECTOR: written by Claude (Fable 5.1) for Finch as setup instructions; not authored, audited, or endorsed by Finch. Read with the ground rules in ../../README.md. -->

# A separate GitHub identity for Claude

Goal: pull requests, review comments, and branch pushes for the triage
lanes are attributed to an identity that is not Finch's account, so his
name signs only his words. Two shapes exist; the first is recommended.

## Option A: a GitHub App (recommended)

Pull requests and comments appear as `<app-name>[bot]`; tokens are
installation tokens that expire after an hour and are minted on demand
from a private key held in 1Password. The App cannot sign commits, so
commits it pushes are unsigned and authored by the bot identity (below).

### 1. Create the App

The App is owned by the account that installs it. For a repo under the
`oxidecomputer` organization, an org owner creates it under the org
(Organization settings, Developer settings, GitHub Apps, New GitHub App);
if you are not an org owner, create it under your personal account
(Settings, Developer settings, GitHub Apps, New GitHub App) and have an
org owner approve its installation on the repository.

- Name: something like `rumors-triage-claude` (this is the `[bot]` name
  shown on every pull request and comment; choose it once).
- Homepage URL: the repository URL.
- Webhook: uncheck "Active" (no webhook needed).
- Repository permissions: Contents read and write (push branches),
  Pull requests read and write (open, comment, review, label), Issues
  read and write (labels and comments share this), Metadata read
  (automatic), Workflows read and write only if a lane must edit
  `.github/workflows/` (the P1 gate lane does: T15, T20, T28), Checks
  read (to read CI results).
- Where can this App be installed: "Only on this account".
- Create it. Note the App ID shown on the App's page.

### 2. Private key and installation

- On the App's page, "Generate a private key". It downloads a `.pem`
  file. Store it in 1Password immediately (a Document or a Secure Note
  field named `private-key`) and delete the downloaded file. Nothing
  writes it to disk on stelmaria afterwards.
- "Install App", choose the `oxidecomputer/rumors` repository only.
  Note the installation ID from the URL of the installation page
  (`.../settings/installations/<installation-id>`).
- Create a label on the repository named `triage:stop`.

### 3. Minting a token on demand

An installation token is minted from a short-lived JWT signed with the
private key. The script below reads the key from 1Password at mint time
and prints a token good for one hour; wrap `gh` calls in it as
`GH_TOKEN=$(./mint-token) gh ...`. Put it at
`~/.local/bin/rumors-claude-token` (chezmoi-managed files publish
themselves; keep it out of the dotfiles repo or mark it ignored).

    #!/usr/bin/env bash
    # Mint a one-hour GitHub App installation token for the Claude identity.
    # Requires: openssl, jq, op (1Password CLI, signed in).
    set -euo pipefail
    APP_ID="<app id>"
    INSTALLATION_ID="<installation id>"
    KEY_REF="op://<vault>/<item>/private-key"
    now=$(date +%s)
    header=$(printf '{"alg":"RS256","typ":"JWT"}' | openssl base64 -A | tr '+/' '-_' | tr -d '=')
    payload=$(printf '{"iat":%d,"exp":%d,"iss":"%s"}' "$((now - 60))" "$((now + 540))" "$APP_ID" \
      | openssl base64 -A | tr '+/' '-_' | tr -d '=')
    signature=$(printf '%s.%s' "$header" "$payload" \
      | openssl dgst -sha256 -sign <(op read "$KEY_REF") | openssl base64 -A | tr '+/' '-_' | tr -d '=')
    jwt="$header.$payload.$signature"
    curl -sS -X POST \
      -H "Authorization: Bearer $jwt" \
      -H "Accept: application/vnd.github+json" \
      "https://api.github.com/app/installations/$INSTALLATION_ID/access_tokens" \
      | jq -r .token

Notes: the App private key never touches disk (`op read` streams it to
`openssl` through a process substitution); the JWT lives ten minutes; the
installation token one hour, so a long lane re-mints before each push.
Pushing over HTTPS with the token: `git -C <worktree> push
https://x-access-token:$(rumors-claude-token)@github.com/oxidecomputer/rumors.git <branch>`,
or set a credential helper that calls the script. Never write the token
into a remote URL that persists in `.git/config`.

### 4. Commit authorship

Commits the lanes push should be authored by the bot, not by you, so the
history's attribution matches the pull request's. In each lane worktree:

    git -C <worktree> config user.name "<app-name>[bot]"
    git -C <worktree> config user.email "<app-id>+<app-name>[bot]@users.noreply.github.com"
    git -C <worktree> config commit.gpgsign false

The App ID in the email is what makes GitHub link the commits to the
App's avatar. These commits are unsigned: a GitHub App has no signing
key, and signing them with your key would attribute them to you. `main`
has carried only signed commits so far and has no rule enforcing that;
merging bot-authored unsigned commits by rebase-merge is a deliberate
change to that convention and is yours to make. If you want every commit
on `main` signed, Option B is the only shape that provides it.

## Option B: a machine user account

A second GitHub user (for example `plaidfinch-claude`), invited to the
repository with write access, with its own SSH signing key held in the
1Password agent, and a fine-grained personal access token (scoped to the
one repository: Contents, Pull requests, Issues, Workflows if needed,
Metadata) stored in 1Password and exposed as `GH_TOKEN` through
`op run` or `op read` at call time. Pull requests and comments appear as
that user; commits can be authored by it and signed with its key, so the
signed-`main` convention survives. The costs: a second account to keep
(2FA, email), and a token that lives until revoked rather than an hour.

## What Claude does with either

- Reads the token only through the mint script or `op`, never from a
  file, and never echoes it.
- Uses it for: pushing lane branches, opening draft pull requests,
  submitting `COMMENT` reviews, adding the `triage:stop` label, and
  marking a pull request ready. Never for approving or merging.
- Reports the identity's name once in `WORKFLOW.md`'s Identity section
  when it exists, so a fresh session knows which account it is speaking
  as.
