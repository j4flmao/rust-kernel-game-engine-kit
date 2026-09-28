# Writing documentation

## Content location

All site pages live in `astro-docs/docs/`. The site does not read the temporary root `docs/` directory at runtime or build time. Navigation is declared in `src/lib/docs.js`; add a slug, title, and group there when adding a page.

## Comark pipeline

The loader uses `createMarkdownParser` from `comark` and `renderHtmlFromDocument` from `@comark/html`. Parsing and rendering happen during Astro's static build. The browser receives HTML. Frontmatter, standard Markdown, tables, code fences, and Comark components are supported by the parser.

The layout owns the page H1. Write a matching H1 in Markdown for readers opening the source; the loader removes the first H1 from rendered content. H2/H3 headings populate the on-page table of contents.

## Links and code

Use relative Markdown links such as `[Runtime](runtime.md)`. The loader maps known Markdown files to local routes. Use full repository URLs for Rust source references. Include a language on fenced code blocks and specify whether commands run from the repository root or the site folder.

## Callouts

```md
::callout{type="warning"}
Native Vulkan validation requires a suitable host and driver.
::
```

Supported callout types are `info` and `warning`. Content is maintained by repository authors; do not feed untrusted uploaded Markdown to this build pipeline.

## Accuracy rules

Verify names and defaults against the implementation. Separate current behavior from proposed design. Describe test scope precisely: compiling a native example is not a hardware rendering test. Keep source links and runnable commands next to explanations. Update the relevant guide when changing a public contract.

## Validate locally

Use fenced `mermaid` blocks for architecture flowcharts and sequence diagrams. The site loads Mermaid locally on pages containing diagrams, with strict rendering and a source fallback. Explain whether each diagram represents ownership, execution order or a design target.

`npm run check` builds the site, checks local links and anchors, then builds the Rust library and executes Markdown Rust examples with rustdoc. It requires the repository checkout and Rust toolchain. The `astro-docs.yml` workflow runs the same checks without deployment.

The source link beneath each article serves its Markdown from the local static site, so unpublished docs can be inspected without a GitHub branch link.

```sh
npm ci
npm run build
npm run preview
```

Check headings, internal links, narrow screens, and code/table overflow. Build output is `dist/`. Set `DOCS_BASE` before build when eventually serving below a URL prefix.

References: [Comark parser](https://comark.dev/reference/parse), [HTML renderer](https://comark.dev/rendering/html), [component syntax](https://comark.dev/syntax/components).
