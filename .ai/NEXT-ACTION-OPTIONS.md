# VSN Organization Next-Action Options Contract

This repository adopts the Vertex Systems Network interactive AI-development handoff standard.

## User-facing handoff

After every repository-development response, expose 1 to 3 currently valid next actions derived from live repository evidence.

- Always include the canonical/recommended next action, but do not bind it permanently to option 1.
- When two or more valid options exist, reshuffle the visible 1/2/3 numbering on every handoff.
- If the previously selected action identity and number are known, that same action must move to a different visible number on the next handoff. With only one valid action, number reuse is allowed.
- Mark the canonical action as **Recommended**. Numbering is ephemeral presentation state and never changes priority, safety, scope, or authorization.
- A reply containing only an option number is a request to start the corresponding next turn. Re-read current repository state before any mutation. If the option became stale or unsafe, fail closed and show the new valid options instead.
- Interactive buttons may be used when the host supports them; otherwise numbered one-line options are the mandatory fallback.

## URL-only repository entry

When the user's message contains only this repository's canonical GitHub URL (optionally with surrounding whitespace), treat it as a read-only development entry request.

1. Resolve the repository and default/protected branch.
2. Read this repository's durable/current state and governing instructions.
3. Reconcile open Issues first, then open PRs, then any repository-specific coordination/runner state required by local rules.
4. Do **not** create a branch, commit, PR, merge, deployment, provider call, destructive action, or other mutation from the URL alone.
5. Respond with 1 to 3 shuffled valid next-action options and mark the canonical one **Recommended**.
6. The user's subsequent number selection initiates the normal fully revalidated development turn.

## Safety and local authority

Repository-specific governance, security, exact-head CI, approval, migration, production/provider, release, and one-turn/one-milestone rules remain authoritative and may be stricter than this interaction contract. This file never grants execution authority and never permits bypassing an accepted actionable Issue/PR or deferred work boundary.
