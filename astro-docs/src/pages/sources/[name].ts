import { entries, docSource } from '../../lib/docs.js';
export function getStaticPaths() {
  return entries.map(entry => ({ params: { name: `${entry.slug}.md` }, props: { entry } }));
}
export async function GET({ props }: { props: { entry: { file: string } } }) {
  const source = docSource(props.entry);
  return new Response(source, { headers: { 'Content-Type': 'text/plain; charset=utf-8' } });
}
