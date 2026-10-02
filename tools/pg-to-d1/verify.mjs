// Compares a Postgres database with a D1 database: row count, max id and money sums per table.
//   node verify.mjs <postgres url> <d1 database> --remote|--local [more wrangler args]
// Runs wrangler as `node $WRANGLER_JS` if set, else `npx wrangler@4.145.0`. Exits 1 on any difference.
import { execFileSync } from 'node:child_process';
import { psql, TABLES } from './tables.mjs';

const [url, ...d1] = process.argv.slice(2);
if (!url || d1.length === 0) {
  console.error('usage: node verify.mjs <postgres url> <d1 database> --remote|--local [wrangler args...]');
  process.exit(2);
}

function wrangler(args) {
  if (process.env.WRANGLER_JS) return execFileSync('node', [process.env.WRANGLER_JS, ...args], { encoding: 'utf8' });
  return execFileSync('npx', ['wrangler@4.145.0', ...args], { encoding: 'utf8', shell: process.platform === 'win32' });
}

const statsSql = (t) =>
  `SELECT COUNT(*) AS n, COALESCE(MAX(id), 0) AS max_id${t.sums.map((c) => `, COALESCE(SUM(${c}), 0) AS ${c}`).join('')} FROM ${t.name}`;

let differences = 0;
for (const table of TABLES) {
  const keys = ['n', 'max_id', ...table.sums];
  const pg = execFileSync(psql, ['-X', '-A', '-t', '-F', '|', '-d', url, '-c', statsSql(table)], { encoding: 'utf8' }).trim().split('|');
  const d1Row = JSON.parse(wrangler(['d1', 'execute', ...d1, '--json', '--command', statsSql(table)]))[0].results[0];
  const diff = keys.filter((k, i) => String(d1Row[k]) !== String(pg[i]));
  const shown = keys.map((k, i) => `${k}=${pg[i]}${diff.includes(k) ? ` (D1: ${d1Row[k]})` : ''}`).join(' ');
  console.log(`${diff.length ? 'DIFF' : 'same'}  ${table.name}: ${shown}`);
  differences += diff.length;
}
console.log(differences ? `\n${differences} difference(s)` : '\nall tables match');
process.exit(differences ? 1 : 0);
