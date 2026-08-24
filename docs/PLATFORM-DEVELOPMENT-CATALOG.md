# VSN Platform Development Catalog — Planning View

Status: future planning catalog. Only entries explicitly marked `existing_certified` in `.ai/catalog/platforms.v1.json` are currently accepted VSN bootstrap templates.

Reviewed: 2026-08-24.

## Audit baseline

Current certified built-ins are **Laravel, Node.js, Django, Rust and Go**. They are implemented by `crates/vsn-project` and exposed through the existing project-provider contract. The broad catalog below is an extension plan, not a rewrite of 02.07.

## Capability groups

| Group | Planned platforms / profiles | Mode |
| --- | --- | --- |
| PHP frameworks | Laravel, Symfony, CodeIgniter, Generic PHP | local native |
| CMS | WordPress, Drupal, Joomla | local/container |
| PHP ecommerce | WooCommerce, OpenCart, PrestaShop, Adobe Commerce/Magento, Shopware | container recommended for reproducibility |
| JavaScript/TypeScript | Node.js, Express, Fastify, NestJS, Next.js, Nuxt, Astro, SvelteKit, Vite | local native |
| Python | Django, FastAPI, Flask | local native |
| Ruby/JVM/.NET | Rails, Spring Boot, ASP.NET Core | local native after runtime-provider prerequisites |
| Rust/Go web | Rust, Axum, Actix Web, Go, Gin, Fiber | local native; framework profiles reuse certified runtime families |
| Headless ecommerce | Medusa, Saleor | local/container |
| Headless CMS | Strapi, Directus | local/container |
| Basic web | Static HTML/CSS/JS | local native |
| SaaS-connected | Shopify, Wix, Webflow | local tooling + vendor-hosted platform |

The JSON catalog contains **41 initial entries** and is deliberately extensible. “All platforms” is not represented by a permanently closed hard-coded list: future official frameworks/providers can be added by versioned catalog/provider extensions through change control.

## New Project flow

The UI/CLI should not show a flat list of forty frameworks. Use progressive selection:

1. **What are you building?** Website, API, ecommerce, CMS, plugin/extension, theme, SaaS-platform app, static site, or custom.
2. **Platform/framework** filtered by category and available runtimes.
3. **Starter** — Hello/Minimal, Standard, Web App, API, Full Stack, Store, Plugin/Extension, Theme, Platform App.
4. **Runtime/toolchain** — compatible installed version, managed install candidate, or explicit missing dependency.
5. **Database/services** — only supported combinations; do not force a DB for a starter that does not need one.
6. **Environment** — local native, container profile or SaaS-connected.
7. **Domain/HTTPS** — `.test`/local HTTPS where supported; vendor preview URL for SaaS-connected workflows.
8. **Git** — initialize/use existing repo and define ignore/secret rules.
9. **AI initialization** — create/read project-specific plan pointers and lifecycle state; never write secrets to `.ai/`.
10. **Review/Create** — show exact commands, network/account requirements, files to create, rollback and acceptance health target before mutation.

## Provider acceptance contract

A platform is not “supported” until VSN proves, at minimum:

- provider descriptor/catalog validation;
- correct project detection or explicit new-project selection;
- prerequisite/runtime/tool resolution;
- dry-run/bootstrap plan without mutation;
- contained scaffold into the requested destination;
- bounded logs/timeouts and rollback on bootstrap failure;
- dependency install/build where applicable;
- deterministic start/health/preview;
- stop/cleanup;
- secret/account/tunnel handling for connected platforms;
- negative tests for invalid path, unsupported version/profile and unsafe network/headers where applicable;
- security and performance budgets;
- acceptance evidence bound to exact source.

## SaaS-connected accuracy

### Shopify

Official Shopify CLI can initialize themes, run `theme dev`, hot reload local source, and run app development against a development store. App localhost mode exists, but some Shopify-triggered features such as webhooks/app proxies require a reachable tunnel/platform callback. VSN should therefore manage local code, CLI, processes, preview, secrets and optional tunnel policy while clearly showing that the Shopify store/runtime is external.

Official research:
- https://shopify.dev/docs/storefronts/themes/tools/cli
- https://shopify.dev/docs/apps/build/cli-for-apps/test-apps-locally
- https://shopify.dev/docs/apps/build/cli-for-apps/networking-options

### Wix

The current Wix CLI scaffolds apps and Wix-managed headless projects, runs a local hot-reload development environment, and connects the work to a Wix account/development site. Wix manages the hosted platform. VSN should wrap the official CLI/account-aware workflow rather than emulate Wix locally.

Official research:
- https://dev.wix.com/docs/wix-cli
- https://dev.wix.com/docs/wix-cli/command-reference/project-commands/dev

### Webflow

The Webflow CLI brings site/CMS/forms/assets and code-component/DevLink workflows into local terminal/version-control workflows. VSN should manage Node tooling, local bundling/dev processes, environment tokens and sync/deploy actions while treating Webflow workspace/site/cloud as external.

Official research:
- https://developers.webflow.com/cli/reference/webflow-cli
- https://developers.webflow.com/devlink/reference/cli

## Local CMS/ecommerce research notes

- WordPress `wp-env` provides a local Docker WordPress environment for plugin/theme work.
- WooCommerce recommends local environments including WordPress Studio/wp-env and provides extension scaffolding with `create-woo-extension`; WooCommerce is built on WordPress, so it should be a composed provider/profile rather than an unrelated stack.
- Symfony provides official project creation through Symfony CLI or Composer and distinct minimal/webapp starter intent.
- CodeIgniter provides the Composer `codeigniter4/appstarter` skeleton, a direct fit for VSN's base-app model.
- Medusa is open source and its current docs explicitly support running backend/admin locally with a local PostgreSQL database; its provider can be local-native.

## Current web-framework research notes

- Next.js provides `create-next-app` and a local dev server.
- Nuxt 4 is current in the reviewed docs; Nuxt 3 reached EOL on 2026-07-31. Never freeze the catalog to an unsupported major merely because an old bootstrap command once worked.
- Astro provides `create astro` with official starters/templates and integrations.

## Design rule for base apps

Every language/framework family should have a minimal runnable path whenever the upstream ecosystem supports one. The simplest viable acceptance is:

`Create -> dependencies ready -> start -> localhost/.test health response -> stop -> cleanup`

Then richer starters (API, full-stack, ecommerce, plugin/theme) can extend that base without forcing the user through framework planning again.

## Extension strategy

Do not grow a single `match template { ... }` indefinitely. Preserve built-in certified behavior, then move future growth toward provider descriptors/profile manifests that can map normalized VSN intents to verified official scaffolds. The existing `ProjectProvider` SDK is the architectural seam for that evolution.
