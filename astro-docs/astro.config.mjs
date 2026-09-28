import { defineConfig } from 'astro/config';

export default defineConfig({
  output: 'static',
  base: process.env.DOCS_BASE || '/',
  server: { host: '127.0.0.1' },
});
