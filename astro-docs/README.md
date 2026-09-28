# Engine documentation site

Local Astro + Comark documentation for rust-kernel-game-engine-kit.

```sh
npm ci
npm run dev
```

Run commands in this directory. Use Node 22.19 or newer to satisfy the dependency tree. Build with `npm run build`; inspect production output with `npm run preview`.

## Structure

- `docs/`: site-owned Markdown guides and engineering references.
- `src/lib/docs.js`: navigation, Comark parsing, links, HTML rendering.
- `src/layouts/Docs.astro`: responsive documentation shell.
- `src/pages/[...slug].astro`: generated static routes.
- `src/styles/docs.css`: shared documentation styles.
- `public/`: static assets.

The root repository's temporary `docs/` is not a build dependency. Engineering reference pages were imported into this site's content directory; maintain the copies here for this site. Guides distinguish source-verified behavior from design targets.

No deployment is configured. Static output goes to `dist/`. A future subpath deployment can set `DOCS_BASE` (for example `/engine/`) before building.

## Validation

Run `npm run check` to build, validate internal links/anchors and execute Rust examples from Markdown against the repository library. Rust and its native linker are required. The Astro documentation workflow runs these checks without publishing.

Mermaid diagrams are bundled locally and loaded only on pages containing diagrams. Every diagram retains its text source as a fallback. Article source links serve local Markdown copies, including before the branch is published.
