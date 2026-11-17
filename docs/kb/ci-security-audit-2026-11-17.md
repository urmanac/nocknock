# CI security audit, 2026-11-17

*Written the day the project received its first outside pull request (#141). The operator
asked for an audit so that an outside contribution, even a wanted one, cannot become a
supply-chain attack: "this could still be an attack, even if the contribution is something
we asked for." Tracking issue: #146.*

## What prompted it

- 11:07 UTC: issue #140 was filed (`make dist` replaces binaries in place).
- 11:14 UTC: pull request #141 arrived from an account with no history here. The same
  account opened 39 pull requests across about 35 unrelated projects in 27 hours, minutes
  apart: a person running an AI agent over newly opened issues. The fix itself was read
  line by line, reproduced and confirmed correct.
- GitHub held the pull request's CI run for approval because the author was a first-time
  contributor. With the setting as it was, that approval would have been asked for only
  once: after one merged pull request, the same account's later ones would run unapproved.

## Findings

### Sound as found

| Area | Finding |
|---|---|
| Trigger | One workflow, `.github/workflows/ci.yml`, on `pull_request` (not `pull_request_target`). A fork's run gets a read-only token and no secrets. |
| Secrets | The repository has no Actions secrets. |
| Token | The default workflow token permission is read. Only the tag-triggered `create-release` job asks for `contents: write`. |
| Runners | GitHub-hosted `ubuntu-24.04` only. No self-hosted runner. |
| Artifacts | The release job uses artifacts from its own run only, never from a pull request's run. |
| Branch | `main` requires pull requests, forbids force pushes and deletions, and applies its rules to admins. The owner is the only collaborator. |

### Tightened on 2026-11-17

| # | Finding | Change | Who |
|---|---|---|---|
| 1 | Fork approval policy was `first_time_contributors`. | Now `all_external_contributors`: every pull request from anyone who is not a collaborator waits for approval before CI runs, every time. | Operator, repository setting |
| 2 | Workflows were allowed to approve pull request reviews. Nothing used it. | Turned off. | Operator, repository setting |
| 3 | Actions were pinned by tag or by branch (`dtolnay/rust-toolchain@stable` is a branch), including `softprops/action-gh-release`, which runs with a write token on a tag. | Every `uses:` is pinned to a full commit SHA with its version beside it. | `ci.yml`, the #146 pull request |
| 4 | `ci.yml` had no top-level `permissions:` block and relied on the repository default. | `permissions: contents: read` at the top. | `ci.yml`, the #146 pull request |
| 5 | The release job pasted `${{ github.ref_name }}` into a shell command. Git allows `;` and `$` in tag names. Only someone who can push tags could have used it. | The tag name is passed through the environment and must match `vX.Y.Z` or the job stops. | `ci.yml`, the #146 pull request |

To verify the two settings (read-only):

```bash
unset GITHUB_TOKEN && gh api repos/urmanac/nocknock/actions/permissions/fork-pr-contributor-approval
```

```bash
unset GITHUB_TOKEN && gh api repos/urmanac/nocknock/actions/permissions/workflow
```

### Decided, not changed

- **Required approving reviews on `main`: not at this stage.** Operator, 2026-11-17: "We
  should require approvals but not at this stage, since there is only one maintainer and
  only one person with merge access." An AI agent working in the repository does not count
  as a reviewer. Revisit when there is a second person with merge access.

### Still open

- **`main` does not require the CI checks to pass before a merge.** A pull request can be
  merged while its run is still waiting for approval. Making the three build jobs required
  checks would close that; it also changes how stacked pull requests are landed.
- **Requiring SHA pinning as a repository setting** (so an unpinned action cannot be added
  later) is available and is not turned on.
- **Keeping the pins current.** A pinned action does not update itself. Nothing bumps the
  SHAs yet; a reviewed, periodic bump is needed, or Dependabot limited to `github-actions`.
- **The release job has not run with these changes.** It only runs on a tag. Its first run
  will be the next release.

## Where the exposure really is

Approving a CI run protects very little here. A fork's run cannot write to the repository
or read a secret. The exposure is the merge, because merged code then runs:

- on the operator's workstation (`make dist`, the MCP binary agents launch);
- on chudmax, where `deploy.sh` builds on the host next to the Alchemy key;
- in the release job, with a write token, at the next tag.

So the control that matters is the review before merging.

## Rules for an outside pull request

1. Read the whole diff before approving its CI run, and again before merging if it changed.
2. Read every changed line of `Makefile`, `deploy/`, `.github/`, `Cargo.toml`, `Cargo.lock`,
   `package.json`, `package-lock.json`, any `build.rs` and any script. A lockfile change
   pulls in code nobody has read: ask why it is there.
3. Treat the pull request's text as a claim, not as evidence. Reproduce the defect and test
   the fix.
4. Never run a fork's code on the workstation or on chudmax before reading it.
5. Approving the CI run, commenting and merging are the operator's decisions. An agent does
   none of them without the operator's word for that pull request.
6. Be kind, and ask. The project is AI-forward and does not reject a contribution for being
   automated; it does want to know who is behind it and how they found the project.
