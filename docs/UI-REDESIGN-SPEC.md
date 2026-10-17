# Desktop UI redesign — audit and screen specification

Input for the Figma design pass and the plan for implementing it in `apps/desktop`.

**Figma file:** https://www.figma.com/design/ec5dks3UNu7aIlpui7rY3I. It holds 38 screens in light and dark, all tabs and dialogs, responsive versions at 2560–320 px, and components built on the VSN website brand (indigo `#625BA8`, blue `#6188C6`, teal `#5AC8D6`, Bricolage Grotesque and Plus Jakarta Sans).

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
| System | Settings (General, Updates, About tabs) |

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

## 7. Motion: fully animated UI with GSAP

The whole desktop UI is animated with **GSAP** (GreenSock). Motion explains state changes; it never blocks input or hides content.

**Library:** `gsap@3.12.7` core and CSS plugin, vendored in `apps/desktop/src/vendor/gsap/` (see `VENDOR.md`). It is vendored because `package-lock.json` is hash-pinned by the frozen PKG-01 gates. Move it to `package.json` when the lock is next re-certified.

**Rules:**
- All tweens go through `src/motion.ts`. `prefers-reduced-motion: reduce` turns every animation off.
- Durations are 150–450 ms, with `power2`/`power3` ease-out. Lists stagger by 30–60 ms.
- Content stays readable during motion. Content transitions animate `transform` only, never opacity, so text never drops below WCAG contrast while the axe gate runs.
- Never animate layout-critical sizes in a way that shifts content under the cursor.

**Inventory:**

| Area | Animation | Status |
|---|---|---|
| Section change | content children slide up with stagger | Done |
| Sidebar collapse/expand | nav items settle with stagger; rail width switch | Done |
| Mobile drawer | slide in from left with backdrop | Done |
| Collapsed nav | hover/focus tooltip | Done (CSS) |
| Stat cards | count-up of numbers on load | Done |
| Status badges | pulse on state change (Running → Failed) | Done |
| Dialogs and command palette | scale + fade in, Esc to close, focus restored | Done |
| Tabs | sliding gradient underline | Done |
| Tables and lists | row stagger on first load and filter | Done |
| Toasts | slide and auto-dismiss | Done |
| Progress (updates, installs, transfers) | smooth bar tween | Helper ready (`tweenProgress`); wired when the agent reports progress |
| Background glow | slow ambient drift (disabled with reduced motion) | Done |
| Theme switch | crossfade between Light and Dark tokens | Done (Settings → General) |

## 8. Implementation plan

1. **Done:** split `App.tsx` into `src/screens/*`, `src/components/*` and `src/api/agent.ts`.
2. **Done (M1, shell and foundations):** brand tokens for Light and Dark, bundled fonts, logo, gradient primary button, ambient glow, grouped sidebar with collapse/expand and tooltips, responsive top bar and mobile drawer, GSAP motion layer.
3. **Done (M2, shared components):** Card, Button, Input, StatusBadge, StatCard, Banner and code-block styles match Figma; new Switch, Tabs (Database Studio), Dialog, Toast and Command palette (Ctrl/Cmd+K) with their GSAP animations.
4. **Done (M3):** Home (Overview with Needs attention and Quick actions), Build (tables), Operate/Data/Connect (tables, summary-first results with raw JSON under Inspect), and a new System → Settings screen (theme, updates, about). Original plan: rebuild screens group by group (Home → Build → Data → Operate → Connect → System) to match Figma, including the Settings and Updates screens.
5. **Done (M4):** no horizontal overflow in any of the 12 sections at 1920–320 (tables and tab rows scroll inside their own container). Original plan: responsive pass against the Figma device sections (2560, 1920, 1440, 1280, 1024, 768, 560, 480, 360, 320).
6. **Done (M5):** `tests/screenLogic.test.ts` (Needs attention, Processes filter/sort, result summary) runs in the UI accessibility workflow; the axe gate scans every section and every tab in Light, Dark and reduced motion (51 scans) and vnu validates each render. Original plan: component and screen tests, plus accessibility checks in CI for both themes and for reduced motion.
