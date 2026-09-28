import { readFile } from 'node:fs/promises';
import { entries } from '../../lib/docs.js';
export function getStaticPaths() {
  return entries.map(entry => ({ params: { name: `${entry.slug}.md` }, props: { entry } }));
}
export async function GET({ props }: { props: { entry: { file: string } } }) {
  const source = await readFile(new URL(`../../../${props.entry.file}`, import.meta.url), 'utf8');
  return new Response(source, { headers: { 'Content-Type': 'text/plain; charset=utf-8' } });
}
