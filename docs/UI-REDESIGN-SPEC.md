# Desktop UI redesign — audit and screen specification

Input for the Figma design pass. After the designs are approved, they will be implemented in `apps/desktop`.

## 1. Audit of the current UI (v0.38.1)

| Area | Finding | Impact |
|---|---|---|
| Code structure | All UI is in `apps/desktop/src/App.tsx`: 58 KB packed into 79 lines, with every component on one line | Very hard to review, test or change safely |
| Styling | `styles.css` is 8.6 KB on 6 lines; colors are hard-coded hex values and there are no design tokens | No theming, inconsistent spacing and type |
| Themes | Dark only | No light mode, ignores the OS setting |
| Data display | 13 screens dump raw JSON (`JsonView`) instead of tables, cards or forms | Not usable for non-developers |
| Navigation | 11 flat sidebar items; Database Studio stacks 5 large panels on one page | Overloaded pages, no hierarchy |
| Feedback | One global `Refresh` button; errors appear as a banner at the top | No per-action loading state, no toasts, no empty states |
| Tests | One acceptance test (`agentOverviewState`) | Changes to the UI are untested |
| Accessibility | `accessibility.css` covers focus and reduced-motion basics only | Contrast, keyboard flows and screen-reader labels are unverified |

**Conclusion:** a full redesign is justified. The agent API (`agent_call` → `command`) stays the same; only the presentation layer changes.

## 2. Design principles

1. **Status at a glance:** every screen shows what is running, what is stopped and what is broken before showing any detail.
2. **Human views first, raw data on demand:** use tables, cards and forms; raw JSON goes behind an "Inspect" drawer.
3. **One primary action per screen.**
4. **Light and dark themes** from a single token set, following the OS setting by default.
5. **Keyboard-first:** a command palette (`Ctrl+K`) and full keyboard navigation, meeting WCAG 2.2 AA.

## 3. App shell

- **Sidebar** (collapsible, 232 px / 64 px): grouped navigation (below), machine switcher at the top, agent connection status at the bottom.
- **Top bar:** page title, breadcrumbs, the screen's primary action, command-palette trigger, theme toggle.
- **Toasts** for action results and an **Inspect drawer** (right side) for raw JSON.
- **Empty, loading and error states** designed for every screen.

### Navigation groups

| Group | Screens |
|---|---|
| Home | Overview |
| Build | Projects, Runtimes, Services |
| Data | Database Studio |
| Operate | Processes, Containers, Terminal, Files |
| Connect | Networking (domains, HTTPS, DNS), Remote |
| System | Settings, Updates, About |

## 4. Screens

Each item below needs a desktop frame (1440×900) and a narrow frame (1024×768) in Figma.

### 4.1 Overview
- Health cards: agent status, running services, active projects, open database connections.
- A "Needs attention" list of failed services, expiring certificates and available updates.
- Recent activity timeline.
- Quick actions: New project, Open terminal, Open Database Studio.

### 4.2 Projects
- Project list (table or cards) showing name, path, runtime, status and local domain.
- Project detail with tabs: Overview, Services, Domain/HTTPS, Preview, Logs.
- "New project" wizard: choose a folder → detect the runtime → assign a `.test` domain → start.

### 4.3 Runtimes
- Installed runtimes (PHP, Node, Python, …) with their versions, the default version and an install/remove action.
- Catalog of available versions.

### 4.4 Services
- Table of services with status, port and start/stop/restart actions.
- Detail drawer with live logs and configuration.

### 4.5 Database Studio (split into sub-pages)
- **Connections:** SQLite, Postgres, MySQL, MongoDB, Redis; TLS badge; add-connection form.
- **Browser:** schema tree on the left and a data grid with pagination on the right.
- **Query:** SQL editor, results grid, transaction controls (Postgres) and a cancel button.
- **Jobs:** background query jobs showing status, duration and cancel.
- **Model analyzer:** schema visualization and findings.

### 4.6 Processes
- Sortable table of process, PID, CPU, memory and owner project, with a kill action (with confirmation).

### 4.7 Containers
- Backend status (Docker/Podman), container list and start/stop actions.

### 4.8 Terminal
- Tabbed terminals with session picker and scrollback.

### 4.9 Files
- Workspace roots, tree and list view, preview pane, and move/rename/delete with confirmation.

### 4.10 Networking
- Local domains (`.test`) and HTTPS certificate status, DNS responder status and a domain plan preview.

### 4.11 Remote
- Paired machines, connection state and a pairing flow.

### 4.12 Settings / Updates / About
- Theme, startup behavior, data folders, update channel and check, version and licenses.

## 5. Design tokens (Figma variables)

- **Color:** `bg`, `surface`, `surface-raised`, `border`, `text`, `text-muted`, `accent`, `success`, `warning`, `danger`, `info` — light and dark modes.
- **Type scale:** 12 / 13 / 14 / 16 / 20 / 24 / 32, using Inter (UI) and JetBrains Mono (code).
- **Spacing:** 4-pt grid (4, 8, 12, 16, 24, 32, 48).
- **Radius:** 6 / 10 / 14. **Elevation:** 3 levels.

## 6. Component inventory

Button (primary/secondary/ghost/danger), IconButton, Input, Select, Switch, Tabs, Table/DataGrid, Card, StatusBadge, Toast, Dialog/Confirm, Drawer, CommandPalette, EmptyState, Skeleton, CodeEditor, LogViewer, Tree.

## 7. Implementation plan (after Figma approval)

1. Split `App.tsx` into `src/screens/*`, `src/components/*` and `src/api/agent.ts` (typed `agent_call` wrapper) with no visual change.
2. Add the token CSS (light and dark) and the base components.
3. Rebuild the screens one group at a time, in this order: Home → Build → Data → Operate → Connect → System.
4. Add component and screen tests; run accessibility checks in CI.
