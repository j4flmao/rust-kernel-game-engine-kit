import { spawnSync } from 'node:child_process';
import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import path from 'node:path';

const root = fileURLToPath(new URL('../', import.meta.url));
const repo = path.resolve(root, '..');
const build = spawnSync('cargo', ['build', '--locked', '--lib', '--message-format=json'], { cwd: repo, encoding: 'utf8' });
if (build.status !== 0) { console.error(build.stderr || build.error); process.exit(1); }
const artifacts = build.stdout.split('\n').filter(Boolean).map(line => JSON.parse(line));
const artifact = artifacts.find(item => item.reason === 'compiler-artifact' && item.target?.name === 'rust_kernel_game_engine_kit' && item.target.kind.includes('lib'));
const lib = artifact?.filenames.find(file => file.endsWith('.rlib'));
if (!lib) throw new Error('Cargo did not return a library artifact for documentation tests');
let count = 0;
for (const file of readdirSync(path.join(root, 'docs'))) {
  const source = path.join(root, 'docs', file);
  if (!file.endsWith('.md') || !/^```rust\s*$/m.test(readFileSync(source, 'utf8'))) continue;
  const result = spawnSync('rustdoc', ['--test', source, '--edition=2021', '--extern', `rust_kernel_game_engine_kit=${lib}`, '-L', `dependency=${path.join(path.dirname(lib), 'deps')}`], { cwd: repo, stdio: 'inherit' });
  if (result.status !== 0) process.exit(1);
  count++;
}
console.log(`Rust examples checked in ${count} Markdown files.`);
