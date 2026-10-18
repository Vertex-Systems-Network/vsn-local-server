# Frameworks

**Projects → Create** sets up a new project with the framework's own installer, inside your workspace.

| Language | Frameworks | Needs |
| --- | --- | --- |
| PHP | WordPress, Laravel, Drupal, Symfony, CodeIgniter 4, CakePHP, Yii 2, Slim, OpenCart, Magento Open Source (Mage-OS mirror) | PHP + Composer (WordPress: WP-CLI) |
| JavaScript | Next.js, Vite (React/Vue), Astro, Nuxt, Express, NestJS | Node + npm |
| Ruby | Rails | Ruby |
| Shopify | Theme | Shopify CLI |

Installers run without prompts and only write into the folder you choose. Creation stops after 15 minutes if an installer hangs.

**Update framework** (in Spaces) runs the right update commands (`composer update`, `npm`/`pnpm`/`yarn` update) based on the files in the project.
