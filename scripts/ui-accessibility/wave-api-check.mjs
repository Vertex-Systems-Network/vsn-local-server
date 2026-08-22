import { mkdir, writeFile } from 'node:fs/promises';

const key = (process.env.WAVE_API_KEY ?? '').trim();
const target = (process.env.WAVE_TEST_URL ?? '').trim();

if (!key) {
  console.error('WAVE_API_KEY is required. Configure it as a GitHub Actions secret.');
  process.exit(2);
}
if (!target) {
  console.error('WAVE_TEST_URL is required. Configure vars.WAVE_TEST_URL or workflow_dispatch input wave_url.');
  process.exit(2);
}

let targetUrl;
try {
  targetUrl = new URL(target);
} catch {
  console.error('WAVE_TEST_URL must be an absolute http(s) URL.');
  process.exit(2);
}
if (!['http:', 'https:'].includes(targetUrl.protocol)) {
  console.error('WAVE_TEST_URL must use http or https.');
  process.exit(2);
}

const endpoint = new URL('https://wave.webaim.org/api/request');
endpoint.searchParams.set('key', key);
endpoint.searchParams.set('url', targetUrl.toString());
endpoint.searchParams.set('format', 'json');
endpoint.searchParams.set('reporttype', '3');

const response = await fetch(endpoint, {
  headers: { 'user-agent': 'VSN-UI-Accessibility-CI/1.0' },
  signal: AbortSignal.timeout(120_000),
});

const text = await response.text();
let report;
try {
  report = JSON.parse(text);
} catch {
  console.error(`WAVE returned non-JSON HTTP ${response.status}.`);
  process.exit(1);
}

await mkdir('dist-accessibility', { recursive: true });
await writeFile('dist-accessibility/wave-report.json', `${JSON.stringify(report, null, 2)}\n`, 'utf8');

if (!response.ok || report?.status?.success !== true) {
  console.error(`WAVE evaluation failed (HTTP ${response.status}).`);
  process.exit(1);
}

const errors = Number(report?.categories?.error?.count ?? 0);
const contrast = Number(report?.categories?.contrast?.count ?? 0);
const alerts = Number(report?.categories?.alert?.count ?? 0);
const summary = {
  target: targetUrl.toString(),
  errors,
  contrast_errors: contrast,
  alerts_for_manual_review: alerts,
  total_elements: Number(report?.statistics?.totalelements ?? 0),
  credits_remaining: report?.statistics?.creditsremaining ?? null,
};
await writeFile('dist-accessibility/wave-summary.json', `${JSON.stringify(summary, null, 2)}\n`, 'utf8');

console.log(`WAVE target: ${summary.target}`);
console.log(`WAVE errors: ${errors}`);
console.log(`WAVE contrast errors: ${contrast}`);
console.log(`WAVE alerts requiring manual review: ${alerts}`);

if (errors > 0 || contrast > 0) {
  console.error('WAVE gate failed: errors and contrast errors must both be zero.');
  process.exit(1);
}

console.log('WAVE gate passed. Alerts remain informational/manual-review items.');
