# UI Accessibility Quality Gate

VSN Desktop UI changes are validated against current web-accessibility standards in GitHub Actions using GitHub-hosted runners only.

## Required automated gates

### 1. W3C / Nu markup validation

The built `apps/desktop/dist/index.html` is checked with the Nu Html Checker (`vnu-jar`). Markup errors fail the job.

### 2. WCAG 2.2 AA regression gate

The built UI is served from loopback on the GitHub-hosted runner and audited with Axe using WCAG A/AA tags through WCAG 2.2. The scanner opens every primary Desktop sidebar section — Overview, Projects, Services, Runtimes, Processes, Containers, Database Studio, Files, Terminal, Networking and Remote — and fails when any automated violation is found.

The report also preserves Axe `incomplete` results for manual review. Browser console/page errors are recorded with the accessibility evidence so silent UI failures do not disappear from the report.

For pull requests that touch the Desktop UI or this accessibility gate, `W3C markup + WCAG 2.2 AA` is the deterministic required CI job. It never uses a local/self-hosted runner.

Automated accessibility tools cannot prove complete WCAG conformance. Keyboard flow, focus order, announcements, content meaning, zoom/reflow, pointer-target usability and other human-evaluation criteria remain review requirements.

### 3. Official WAVE API gate

The official WebAIM WAVE API is integrated through `scripts/ui-accessibility/wave-api-check.mjs`.

Repository configuration:

- GitHub Actions secret: `WAVE_API_KEY`
- GitHub Actions variable: `WAVE_TEST_URL` containing a publicly reachable staging/page URL

The WAVE API requires both an API key and a publicly reachable URL. The CI gate fails when WAVE reports either accessibility errors or contrast errors. WAVE alerts are preserved in the report for manual review rather than automatically treated as failures.

WAVE is run on `main` pushes when `WAVE_TEST_URL` is configured, or manually through `workflow_dispatch` with an optional `wave_url` override. Pull-request-local UI cannot be sent directly to the hosted WAVE API because localhost is not publicly reachable.

## Evidence

The workflow uploads:

- `w3c-vnu.txt`
- `axe-wcag22-sections.json`
- `preview.log`
- `wave-report.json` when WAVE runs
- `wave-summary.json` when WAVE runs

## Policy

- Baseline target: WCAG 2.2 AA.
- Do not disable an accessibility rule merely to make CI green.
- Fix semantic HTML, labels, focus behavior, contrast and interaction defects in product code.
- Any documented exception must identify the affected WCAG criterion, reason, owner and expiry/review date.
- Axe incomplete/manual results and WAVE alerts require human review even when the automated gates pass.
