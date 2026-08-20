import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const roots = ['contracts', 'providers'];
const files = [];
function walk(dir) {
  for (const entry of fs.readdirSync(dir, {withFileTypes:true})) {
    const full=path.join(dir,entry.name);
    if (entry.isDirectory()) walk(full);
    else if (entry.isFile() && entry.name.endsWith('.json')) files.push(full);
  }
}
for (const rel of roots) walk(path.join(root,rel));
let failed=false;
for (const file of files.sort()) {
  const rel=path.relative(root,file);
  try { JSON.parse(fs.readFileSync(file,'utf8')); console.log(`OK ${rel}`); }
  catch(error) { failed=true; console.error(`FAIL ${rel}: ${error.message}`); }
}
console.log(`checked=${files.length}`);
process.exit(failed?1:0);
