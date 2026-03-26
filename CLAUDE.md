# Project Stack
- `frontend/` — Angular
- `backend/` — Rust
- `admin/` — Next.js (Admin Panel)
- `mobile/` — Flutter

## Dev Commands
- Backend : `cd backend && cargo run`
- Frontend : `cd frontend && npm start`
- Admin   : `cd admin && npm run dev`
- Mobile  : `cd mobile && flutter run`

---

# Linear Workflow

## Before starting any ticket
Pull the ticket from Linear: use mcp__linear-server__get_issue with the issue ID
Read comments on the immediately preceding ticket (e.g. if starting E3-003, read E3-002 comments) using mcp__linear-server__list_comments — look for handoff notes left by the previous agent
Read the full description, acceptance criteria, technical notes, and API tests
Set the ticket status to In Progress via mcp__linear-server__save_issue

## Branch naming
Always create a new branch before starting any implementation
Branch name format: {ticket-id}/{short-description}
Example: ifi-15/tenant-schema, ifi-16/tenant-registration-form
If the current ticket depends on (is blocked by) a previous ticket that is not yet merged, branch off that ticket's branch instead of main
Check the ticket's "Blocked by" field in Linear to determine the parent branch
Otherwise always branch from main

## Committing
After implementation is verified (build passes, API tests pass):

Use atomic commits: group logically related files into one commit — do not commit file-by-file, but also do not dump all changes in a single commit if they span unrelated concerns
Follow Conventional Commits format (already used in this repo):
feat(scope): description — new feature or endpoint
fix(scope): description — bug fix
refactor(scope): description — code restructure without behavior change
chore(scope): description — tooling, deps, config
docs(scope): description — documentation only
Scope is optional but preferred (e.g. feat(tenant): add registration form)
Keep commit messages concise — describe what changed and why, not how
Example groupings for a schema + API + UI ticket:
Commit 1: feat(schema): add Tenant and TenantStatusLog models with migration
Commit 2: feat(tenant): add tenant registration tRPC procedure
Commit 3: feat(ui): add public tenant registration form

## While working
Add implementation notes as comments via mcp__linear-server__save_comment
If a blocker is discovered, add a comment with the blocker details and leave status as In Progress

## API testing requirement
For every ticket that introduces tRPC procedures, run ALL curl test cases specified in the ticket's "API Tests" section
Test the running dev server (pnpm dev) — do not skip tests
Post test results (pass/fail per case) as a comment on the ticket before marking Done

## After completing a ticket
Run all API tests listed in the ticket and post results as a comment

Post a completion comment using this structure:

What was built

{Bullet list of features/endpoints/UI pages implemented}
Key files changed

{file path} — {what changed}
API test results

✅ {test case}: passed — {actual response summary}
❌ {test case}: failed — {error}
Handoff notes

{Context the next ticket needs: schema fields added, helpers introduced, patterns to follow}
{Any deviations from the original spec and why}
Formatting rule: wrap all code references in backticks — enum values, field names, model names, function names, status constants, file paths, and CLI commands.

Set status to In Review (needs human review) or Done (self-contained + tests pass)

Create atomic commits following Conventional Commits (see "Committing" section)

Push the branch and open a GitHub PR to main (see "Creating a PR" section)

Post the PR URL as a follow-up comment on the Linear ticket

## Finding your next ticket
Work in epic dependency order: E1 → E2 → E3/E4/E7 (parallel) → E5 → E6
Within an epic, work tickets in numeric order unless a specific one is unblocked earlier
Use mcp__linear-server__list_issues filtered by project + status=Todo to find next work
Pull the full ticket with mcp__linear-server__get_issue before starting — never work from memory alone

## Status meanings
| Status      | Meaning                                              |
|-------------|------------------------------------------------------|
| Backlog     | Not yet scheduled                                    |
| Todo        | Scheduled for current session                        |
| In Progress | Actively being worked on                             |
| In Review   | Built, awaiting review / testing                     |
| Done        | Verified complete — all AC checked, API tests passed |
| Canceled    | Descoped or not needed                               |

---

# GitHub Workflow

## Creating a PR
After all commits are pushed, open a GitHub PR targeting main:

Title format: [{TICKET-ID}] {ticket title}

Example: [IFI-15] Database Schema — Core Tables (Tenants, Users, Sessions)
PR description format:

## Overview
{1–2 sentences describing what this PR does and why}

- {Bullet expanding on a key detail, decision, or scope point}
- {Another notable aspect — e.g. what was removed, replaced, or why an approach was chosen}

## Changes
- `{file or area}` — {what changed}
- ...

## Testing
- {Step-by-step instructions to verify the changes work}
- {Include any seed data, env vars, or setup needed}

## Notes
{Optional paragraph for deviations from spec, caveats, or anything the reviewer should know.
Omit this section if there is nothing notable.}

## Linear
{Linear ticket URL}

After the PR is created, post its URL as a separate comment on the Linear ticket (after the implementation summary comment)
Use mcp__linear-server__save_comment with the PR URL and a one-line summary
