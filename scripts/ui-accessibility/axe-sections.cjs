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
  'Projects',
  'Services',
  'Runtimes',
  'Processes',
  'Containers',
  'Database Studio',
  'Files',
  'Terminal',
  'Networking',
  'Remote',
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
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });

  const report = {
    standard: 'WCAG 2.2 AA',
    tags,
    url: baseUrl,
    sections: [],
    browserConsoleErrors: [],
  };

  try {
    const page = await browser.newPage();
    page.on('console', (msg) => {
      if (msg.type() === 'error') report.browserConsoleErrors.push(msg.text());
    });
    page.on('pageerror', (error) => report.browserConsoleErrors.push(String(error)));

    await page.goto(baseUrl, { waitUntil: 'networkidle0', timeout: 30_000 });
    await new Promise((resolve) => setTimeout(resolve, 500));
    await page.addScriptTag({ content: axe.source });

    let totalViolations = 0;
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
        await new Promise((resolve) => setTimeout(resolve, 200));
      }

      fs.writeFileSync(
        path.join(renderedDir, `${slug(section)}.html`),
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
      report.sections.push({ section, violations, incomplete });
      console.log(`${section}: ${violations.length} violation rule(s), ${incomplete.length} incomplete/manual result(s)`);
    }

    report.totalViolationRulesAcrossSections = totalViolations;
    fs.writeFileSync(
      path.join(outDir, 'axe-wcag22-sections.json'),
      `${JSON.stringify(report, null, 2)}\n`,
      'utf8',
    );

    if (totalViolations > 0) {
      console.error(`WCAG 2.2 AA gate failed: ${totalViolations} violation rule occurrence(s) across ${sections.length} sections.`);
      process.exitCode = 1;
    } else {
      console.log(`WCAG 2.2 AA gate passed across all ${sections.length} Desktop sections.`);
    }
  } finally {
    await browser.close();
  }
})().catch((error) => {
  console.error(error);
  process.exit(1);
});
