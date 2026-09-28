import { readFile } from 'node:fs/promises';
import { createMarkdownParser } from 'comark';
import { renderHtmlFromDocument } from '@comark/html';
import { codeToHtml, bundledLanguages } from 'shiki';

const repo = 'https://github.com/j4flmao/rust-kernel-game-engine-kit/blob/main/';
export const entries = [
  ['overview', 'Overview', 'Start here', 'src/content/overview.md'],
  ['getting-started', 'Getting started', 'Start here', 'src/content/getting-started.md'],
  ['runtime', 'Kernel & frame lifecycle', 'Engine guide', 'src/content/runtime.md'],
  ['first-plugin', 'Build your first game plugin', 'Practical guides', 'first-plugin.md'],
  ['end-to-end', 'Input to headless rendering', 'Practical guides', 'end-to-end.md'],
  ['operations', 'Configuration & troubleshooting', 'Practical guides', 'operations.md'],
  ['ecs-memory', 'Entities, components & memory', 'Practical guides', 'ecs-memory.md'],
  ['message-bus', 'Send and receive messages', 'Practical guides', 'message-bus.md'],
  ['subsystems', 'Audio, physics & scripting', 'Practical guides', 'subsystems.md'],
  ['source-map', 'Source & test map', 'Practical guides', 'source-map.md'],
  ['ui-guide', 'Create and lay out UI', 'Practical guides', 'ui-guide.md'],
  ['replication', 'Replication & persistence', 'Practical guides', 'replication.md'],
  ['rendering', 'Rendering & platform support', 'Engine guide', 'src/content/rendering.md'],
  ['examples', 'Sudoku & Rubik', 'Engine guide', '../examples/README.md'],
  ['benchmarks', 'Benchmarks & reports', 'Engine guide', 'src/content/benchmarks.md'],
  ['architecture', 'Architecture', 'Engineering reference', '../docs/ARCHITECTURE.md'],
  ['schedule', 'Runtime schedule', 'Engineering reference', '../docs/RUNTIME_SCHEDULE_PLAN.md'],
  ['ecs', 'ECS value index', 'Engineering reference', '../docs/ECS_VALUE_INDEX.md'],
  ['gpu', 'GPU-driven rendering', 'Engineering reference', '../docs/GPU_DRIVEN_RENDERING_PLAN.md'],
  ['vulkan', 'Vulkan platform', 'Engineering reference', '../docs/VULKAN_PLATFORM_PLAN.md'],
  ['3d', '3D rendering', 'Engineering reference', '../docs/3D_RENDERING_PLAN.md'],
  ['ui', 'UI rendering', 'Engineering reference', '../docs/UI_RENDERING_PLAN.md'],
  ['testing', 'Testing & benchmarks', 'Delivery & reliability', '../docs/TESTING_BENCHMARK_PLAN.md'],
  ['ci', 'CI/CD', 'Delivery & reliability', '../docs/CI_CD_PLAN.md'],
  ['security', 'Security & stability', 'Delivery & reliability', '../docs/SECURITY_STABILITY_PLAN.md'],
  ['roadmap', 'Roadmap', 'Delivery & reliability', '../docs/ROADMAP.md'],
  ['authoring', 'Writing documentation', 'Contributing', 'src/content/authoring.md'],
].map(([slug, title, group, original]) => ({ slug, title, group, original, file: `docs/${slug}.md`, reference: original.startsWith('../docs/') }));

const parse = createMarkdownParser({ autoClose: false });
export function href(slug = '') {
  return `${import.meta.env.BASE_URL.replace(/\/$/, '')}/${slug ? `${slug}/` : ''}`;
}
export async function loadDoc(entry) {
  const source = await readFile(new URL(`../../${entry.file}`, import.meta.url), 'utf8');
  const document = await parse(source);
  const headings = [];
  function visit(nodes) {
    for (const node of nodes) {
      if (!Array.isArray(node)) continue;
      const [tag, attrs, ...children] = node;
      if (/^h[23]$/.test(tag)) {
        const text = (parts) => parts.map(p => typeof p === 'string' ? p : Array.isArray(p) ? text(p.slice(2)) : '').join('');
        headings.push({ id: attrs.id, title: text(children), depth: Number(tag[1]) });
      }
      if (tag === 'a' && typeof attrs.href === 'string' && !/^(https?:|mailto:|#|\/)/.test(attrs.href)) {
        const [path, anchor] = attrs.href.split('#');
        const target = entries.find(e => e.file.split('/').at(-1) === path.split('/').at(-1) || e.original.split('/').at(-1) === path.split('/').at(-1));
        attrs.href = target ? href(target.slug) + (anchor ? `#${anchor}` : '') : new URL(path, repo + (entry.reference ? 'docs/' : '')).href;
      }
      visit(children);
    }
  }
  // Page layout owns the single H1; original headings remain in the source files.
  if (Array.isArray(document.nodes[0]) && document.nodes[0][0] === 'h1') document.nodes.shift();
  visit(document.nodes);
  const html = await renderHtmlFromDocument(document, {
    components: {
      pre: async ([, attrs, ...children], { render }) => {
        const code = children.find(node => Array.isArray(node) && node[0] === 'code');
        if (!code) return `<pre>${await render(children)}</pre>`;
        const language = String(code[1].language || code[1].class || attrs.language || 'text').replace(/^language-/, '');
        const lang = language in bundledLanguages ? language : 'text';
        const text = code.slice(2).filter(value => typeof value === 'string').join('');
        if (language === 'mermaid') {
          const escaped = text.replaceAll('&', '&amp;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
          return `<figure class="diagram"><div class="mermaid">${escaped}</div><details><summary>Diagram source and text fallback</summary><pre><code>${escaped}</code></pre></details></figure>`;
        }
        const html = await codeToHtml(text, { lang, theme: 'github-dark' });
        return `<div class="code-block"><div class="code-toolbar"><span>${lang}</span><button type="button" class="copy-code">Copy code</button></div>${html}</div>`;
      },
      callout: async ([, attrs, ...children], { render }) => {
        const type = ['info', 'warning'].includes(attrs.type) ? attrs.type : 'info';
        return `<aside class="callout ${type}">${await render(children)}</aside>`;
      },
    },
  });
  return { ...entry, html, headings, source: href(`sources/${entry.slug}.md`).replace(/\/$/, '') };
}
