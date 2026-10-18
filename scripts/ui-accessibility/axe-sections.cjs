const fs = require('node:fs');
const path = require('node:path');
const puppeteer = require('puppeteer');
const axe = require('axe-core');

const baseUrl = process.env.UI_TEST_URL || 'http://127.0.0.1:4173/';
const outDir = process.env.UI_A11Y_OUT || 'dist-accessibility';
const renderedDir = path.join(outDir, 'rendered');
const tags = ['wcag2a', 'wcag2aa', 'wcag21a', 'wcag21aa', 'wcag22aa'];
const sections = [
  'Overview',
  'Spaces',
  'Mail',
  'API client',
  'Projects',
  'Source control',
  'Services',
  'Runtimes',
  'Analytics',
  'Processes',
  'Containers',
  'Database Studio',
  'Files',
  'Terminal',
  'Networking',
  'Deploy',
  'Remote',
  'Settings',
  'Updates',
  'About',
];

function slug(value) {
  return value.toLowerCase().replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
}

function compactViolation(v) {
  return {
    id: v.id,
    impact: v.impact,
    help: v.help,
    helpUrl: v.helpUrl,
    tags: v.tags,
    nodes: v.nodes.map((node) => ({
      target: node.target,
      html: node.html,
      failureSummary: node.failureSummary,
    })),
  };
}

(async () => {
  fs.mkdirSync(renderedDir, { recursive: true });
  const browser = await puppeteer.launch({
    headless: true,
    executablePath: process.env.PUPPETEER_EXECUTABLE_PATH || undefined,
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });

  const report = {
    standard: 'WCAG 2.2 AA',
    tags,
    url: baseUrl,
    sections: [],
    browserConsoleErrors: [],
  };

  // Every section is scanned in Light, Dark and Light + reduced motion, including each tab inside a section.
  // Waits cover the longest GSAP entrance (fade + stagger) so contrast is never sampled mid-animation.
  const modes = [
    { id: 'light', features: [{ name: 'prefers-color-scheme', value: 'light' }] },
    { id: 'dark', features: [{ name: 'prefers-color-scheme', value: 'dark' }] },
    {
      id: 'reduced-motion',
      features: [
        { name: 'prefers-color-scheme', value: 'light' },
        { name: 'prefers-reduced-motion', value: 'reduce' },
      ],
    },
  ];
  const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

  try {
    const page = await browser.newPage();
    page.on('console', (msg) => {
      if (msg.type() === 'error') report.browserConsoleErrors.push(msg.text());
    });
    page.on('pageerror', (error) => report.browserConsoleErrors.push(String(error)));

    let totalViolations = 0;
    let scans = 0;
    const scan = async (mode, section, tab) => {
      const name = [mode, section, tab].filter(Boolean).join(' / ');
      fs.writeFileSync(
        path.join(renderedDir, `${slug(name)}.html`),
        await page.content(),
        'utf8',
      );
      const result = await page.evaluate(async (runTags) => {
        return window.axe.run(document, {
          runOnly: { type: 'tag', values: runTags },
          resultTypes: ['violations', 'incomplete'],
        });
      }, tags);
      const violations = result.violations.map(compactViolation);
      const incomplete = result.incomplete.map(compactViolation);
      totalViolations += violations.length;
      scans += 1;
      report.sections.push({ mode, section, tab: tab || null, violations, incomplete });
      console.log(`${name}: ${violations.length} violation rule(s), ${incomplete.length} incomplete/manual result(s)`);
    };

    for (const mode of modes) {
      await page.emulateMediaFeatures(mode.features);
      await page.goto(baseUrl, { waitUntil: 'networkidle0', timeout: 30_000 });
      // Start-up loader stays ~1.4 s and fades out over 0.5 s.
      await page.waitForSelector('.splash', { hidden: true, timeout: 15_000 });
      await pause(1500);
      await page.addScriptTag({ content: axe.source });
      for (const section of sections) {
        if (section !== 'Overview') {
          const found = await page.evaluate((label) => {
            const button = [...document.querySelectorAll('nav button')]
              .find((candidate) => (candidate.textContent || '').includes(label));
            if (!button) return false;
            button.click();
            return true;
          }, section);
          if (!found) throw new Error(`Navigation button not found for section: ${section}`);
          await pause(1400);
        }
        const tabLabels = await page.evaluate(() =>
          [...document.querySelectorAll('.content [role="tab"]')].map((t) => (t.textContent || '').trim()),
        );
        if (!tabLabels.length) {
          await scan(mode.id, section);
          continue;
        }
        for (const tab of tabLabels) {
          await page.evaluate((label) => {
            [...document.querySelectorAll('.content [role="tab"]')]
              .find((t) => (t.textContent || '').trim() === label)
              ?.click();
          }, tab);
          await pause(1400);
          await scan(mode.id, section, tab);
        }
      }
    }

    report.totalViolationRulesAcrossSections = totalViolations;
    fs.writeFileSync(
      path.join(outDir, 'axe-wcag22-sections.json'),
      `${JSON.stringify(report, null, 2)}\n`,
      'utf8',
    );

    if (totalViolations > 0) {
      console.error(`WCAG 2.2 AA gate failed: ${totalViolations} violation rule occurrence(s) across ${scans} scans.`);
      process.exitCode = 1;
    } else {
      console.log(`WCAG 2.2 AA gate passed across ${scans} scans (${sections.length} Desktop sections × Light, Dark, reduced motion, every tab).`);
    }
  } finally {
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
