import { defineConfig } from 'vite';
import { readFileSync } from 'node:fs';
const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf8'));
export default defineConfig({ clearScreen:false, server:{port:1420,strictPort:true}, envPrefix:['VITE_','TAURI_'], define:{__APP_VERSION__:JSON.stringify(pkg.version)}, build:{target:'es2022',minify:'esbuild',sourcemap:!!process.env.TAURI_ENV_DEBUG} });
