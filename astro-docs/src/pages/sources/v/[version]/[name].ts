import { defaultVersion, docSource, entries, versions } from '../../../../lib/docs.js';

export function getStaticPaths() {
  return versions.flatMap(version => entries.map(entry => ({
    params: { version: version.id, name: `${entry.slug}.md` },
    props: { entry, version: version.id },
  })));
}

export async function GET({ props }: { props: { entry: { file: string }; version?: string } }) {
  const source = docSource(props.entry, props.version || defaultVersion);
  return new Response(source, {
    headers: {
      'Content-Type': 'text/plain; charset=utf-8',
      'X-Documentation-Version': props.version || defaultVersion,
    },
  });
}
