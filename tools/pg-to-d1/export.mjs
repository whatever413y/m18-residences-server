// Exports the Postgres database (read-only) as SQLite INSERT statements for D1, keeping every id.
//   node export.mjs <postgres url> <output.sql>
// Then: wrangler d1 execute m18-residences --remote --file <output.sql>   (--local for a rehearsal)
// The output holds real tenant data: write it outside any repo.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import { psql, TABLES } from './tables.mjs';

const [url, out] = process.argv.slice(2);
if (!url || !out) {
  console.error('usage: node export.mjs <postgres url> <output.sql>');
  process.exit(2);
}

/** Runs one statement; the session is ISO/UTC so timestamps come out as `2025-08-20 01:03:40.123456`. */
function query(sql) {
  return execFileSync(psql, ['-X', '-q', '-v', 'ON_ERROR_STOP=1', '-d', url, '-c', "SET datestyle = 'ISO, YMD'", '-c', "SET timezone = 'UTC'", '-c', sql], {
    encoding: 'utf8',
    maxBuffer: 1 << 30,
  });
}

/** CSV as produced by COPY ... (FORMAT csv, FORCE_QUOTE *): quoted values are strings, empty unquoted ones are NULL. */
function parseCsv(text) {
  const rows = [];
  let row = [];
  let i = 0;
  while (i < text.length) {
    if (text[i] === '"') {
      let value = '';
      i++;
      for (;;) {
        if (i >= text.length) throw new Error('unterminated quoted value');
        if (text[i] === '"' && text[i + 1] === '"') {
          value += '"';
          i += 2;
        } else if (text[i] === '"') {
          i++;
          break;
        } else {
          value += text[i++];
        }
      }
      row.push(value);
    } else {
      const start = i;
      while (i < text.length && text[i] !== ',' && text[i] !== '\n' && text[i] !== '\r') i++;
      const raw = text.slice(start, i);
      row.push(raw === '' ? null : raw);
    }
    if (text[i] === ',') {
      i++;
      continue;
    }
    if (text[i] === '\r') i++;
    rows.push(row);
    row = [];
    i++; // the newline
  }
  return rows;
}

const sqlString = (s) => `'${s.replace(/'/g, "''")}'`;

function literal(type, value, where) {
  if (value === null) return 'NULL';
  switch (type) {
    case 'int':
      if (!/^-?\d+$/.test(value)) throw new Error(`${where}: not an integer: ${value}`);
      return value;
    case 'bool':
      if (value !== 't' && value !== 'f') throw new Error(`${where}: not a boolean: ${value}`);
      return value === 't' ? '1' : '0';
    case 'ts':
      // Stored as text; the D1 adapter reads `YYYY-MM-DD HH:MM:SS[.ffffff]`.
      if (!/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}(\.\d+)?$/.test(value)) throw new Error(`${where}: unexpected timestamp: ${value}`);
      return sqlString(value);
    case 'text':
      return sqlString(value);
    default:
      throw new Error(`unknown column type ${type}`);
  }
}

const lines = [`-- M18 Residences: Postgres -> D1 export, ${new Date().toISOString()}. Contains tenant data; do not commit.`];
const summary = [];
for (const table of TABLES) {
  const cols = Object.keys(table.columns);
  const rows = parseCsv(query(`COPY (SELECT ${cols.join(', ')} FROM ${table.name} ORDER BY id) TO STDOUT WITH (FORMAT csv, FORCE_QUOTE *)`));
  for (const [r, row] of rows.entries()) {
    if (row.length !== cols.length) throw new Error(`${table.name} row ${r}: ${row.length} values for ${cols.length} columns`);
    const values = cols.map((c, k) => literal(table.columns[c], row[k], `${table.name}.${c} (row ${r})`));
    lines.push(`INSERT INTO ${table.name} (${cols.join(', ')}) VALUES (${values.join(', ')});`);
  }
  // Keep ids from ever being reused: continue each AUTOINCREMENT counter where the Postgres sequence stands.
  // The id column's own sequence, whatever its name (the old Prisma schema's are e.g. "Rooms_id_seq").
  const seqText = query(
    `SELECT COALESCE((SELECT last_value FROM pg_sequences WHERE format('%I.%I', schemaname, sequencename) = pg_get_serial_sequence('${table.name}', 'id')), 0)`,
  );
  const seq = seqText
    .split(/\r?\n/)
    .map((l) => l.trim())
    .find((l) => /^\d+$/.test(l));
  if (seq === undefined) throw new Error(`no sequence value for ${table.name}: ${seqText}`);
  lines.push(`UPDATE sqlite_sequence SET seq = MAX(seq, ${seq}) WHERE name = '${table.name}';`);
  lines.push(`INSERT INTO sqlite_sequence (name, seq) SELECT '${table.name}', ${seq} WHERE NOT EXISTS (SELECT 1 FROM sqlite_sequence WHERE name = '${table.name}');`);
  summary.push(`${table.name}: ${rows.length} rows, ids continue after ${seq}`);
}
fs.writeFileSync(out, `${lines.join('\n')}\n`);
console.log(summary.join('\n'));
console.log(`wrote ${out}`);
