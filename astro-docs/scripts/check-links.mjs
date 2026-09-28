import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const dist = fileURLToPath(new URL('../dist/', import.meta.url));
const base = (process.env.DOCS_BASE || '/').replace(/\/$/, '');
const files = readdirSync(dist, { recursive: true }).filter(file => file.endsWith('.html'));
const errors = [];
for (const file of files) {
  const html = readFileSync(path.join(dist, file), 'utf8');
  const origin = new URL(file.replaceAll('\\', '/'), 'https://docs.local/');
  for (const match of html.matchAll(/href="([^"]+)"/g)) {
    const href = match[1].replaceAll('&amp;', '&');
    if (/^(https?:|mailto:|tel:)/.test(href)) continue;
    const url = new URL(href, origin);
    let pathname = decodeURIComponent(url.pathname);
    if (base && pathname.startsWith(`${base}/`)) pathname = pathname.slice(base.length);
    let target = path.join(dist, pathname);
    if (pathname.endsWith('/')) target = path.join(target, 'index.html');
    if (!existsSync(target)) { errors.push(`${file}: missing ${href}`); continue; }
    if (url.hash && target.endsWith('.html')) {
      const id = decodeURIComponent(url.hash.slice(1));
      const targetHtml = readFileSync(target, 'utf8');
      if (!targetHtml.includes(`id="${id}"`)) errors.push(`${file}: missing anchor ${href}`);
    }
  }
}
if (errors.length) { console.error(errors.join('\n')); process.exit(1); }
console.log(`Checked local links and anchors in ${files.length} pages.`);
