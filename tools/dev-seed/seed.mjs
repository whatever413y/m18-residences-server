// Fills the development database and bucket (env "development" in ../../wrangler.jsonc) with synthetic data:
// 6 rooms, 6 tenants, 12 months of readings and bills (all but the latest paid, each with a sample receipt),
// and placeholder payment QR images. Never touches production: it refuses any database or bucket not named *-dev.
//   node seed.mjs           seed an empty dev database
//   node seed.mjs --reset   empty the dev tables first
// Credentials: CLOUDFLARE_API_TOKEN and CLOUDFLARE_ACCOUNT_ID from the secrets file ($M18_SECRETS_FILE, default
// ../../../m18-residences-infra/.env). Wrangler: $WRANGLER_JS, else the e2e suite's pinned copy.
import { execFileSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import zlib from 'node:zlib';

const here = path.dirname(fileURLToPath(import.meta.url));
const server = path.resolve(here, '..', '..');
const DB = 'm18-residences-dev';
const BUCKET = 'm18-residences-dev';
for (const name of [DB, BUCKET]) if (!name.endsWith('-dev')) throw new Error(`refusing ${name}: not a -dev resource`);

const secretsFile = process.env.M18_SECRETS_FILE ?? path.resolve(server, '..', 'm18-residences-infra', '.env');
const secrets = Object.fromEntries(
  fs
    .readFileSync(secretsFile, 'utf8')
    .split(/\r?\n/)
    .map((l) => l.match(/^\s*([A-Z_][A-Z0-9_]*)\s*=\s*(.*)$/))
    .filter(Boolean)
    .map((m) => [m[1], m[2].trim().replace(/^["']|["']$/g, '')]),
);
for (const name of ['CLOUDFLARE_API_TOKEN', 'CLOUDFLARE_ACCOUNT_ID']) {
  if (!secrets[name]) throw new Error(`${name} is missing in ${secretsFile}`);
}
const wranglerJs =
  process.env.WRANGLER_JS ?? path.resolve(server, '..', 'shared-e2e', 'm18-residences', 'node_modules', 'wrangler', 'bin', 'wrangler.js');
const env = { ...process.env, CLOUDFLARE_API_TOKEN: secrets.CLOUDFLARE_API_TOKEN, CLOUDFLARE_ACCOUNT_ID: secrets.CLOUDFLARE_ACCOUNT_ID };
const wrangler = (args) => execFileSync('node', [wranglerJs, ...args], { cwd: server, env, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });

/** A grayscale PNG drawn by `pixel(x, y)` (0 black … 255 white). */
function png(width, height, pixel) {
  const raw = Buffer.alloc((width + 1) * height);
  for (let y = 0; y < height; y++) for (let x = 0; x < width; x++) raw[y * (width + 1) + 1 + x] = pixel(x, y);
  const chunk = (type, data) => {
    const len = Buffer.alloc(4);
    len.writeUInt32BE(data.length);
    const typed = Buffer.concat([Buffer.from(type), data]);
    const crc = Buffer.alloc(4);
    crc.writeUInt32BE(zlib.crc32(typed));
    return Buffer.concat([len, typed, crc]);
  };
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // bit depth; colour type 0 = grayscale
  const signature = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  return Buffer.concat([signature, chunk('IHDR', ihdr), chunk('IDAT', zlib.deflateSync(raw)), chunk('IEND', Buffer.alloc(0))]);
}

/** A fake QR code: three finder squares and deterministic blocks (not scannable). */
function fakeQr(seed) {
  const finder = (bx, by, cx, cy) => {
    const dx = bx - cx;
    const dy = by - cy;
    if (dx < 0 || dy < 0 || dx > 6 || dy > 6) return false;
    const ring = Math.max(Math.abs(dx - 3), Math.abs(dy - 3));
    return ring !== 2;
  };
  return png(290, 290, (x, y) => {
    const bx = Math.floor((x - 20) / 10);
    const by = Math.floor((y - 20) / 10);
    if (x < 20 || y < 20 || bx > 24 || by > 24) return 255;
    if (finder(bx, by, 0, 0) || finder(bx, by, 18, 0) || finder(bx, by, 0, 18)) return 0;
    if (bx < 8 && by < 8) return 255; // quiet space around the finder squares
    if (bx > 16 && by < 8) return 255;
    if (bx < 8 && by > 16) return 255;
    return (Math.imul(bx + 1, 73856093) ^ Math.imul(by + 1, 19349663) ^ Math.imul(seed + 1, 83492791)) >>> 0 & 4 ? 0 : 255;
  });
}

/** A fake receipt: a sheet with ruled lines. */
const fakeReceipt = () => png(400, 560, (x, y) => (x < 20 || x > 380 || y < 20 || y > 540 ? 200 : y % 40 === 0 && x > 50 && x < 350 ? 120 : 250));

const TENANTS = ['ALPHA', 'BRAVO', 'CHARLIE', 'DELTA', 'ECHO', 'FOXTROT'];
const MONTHS = 12;
const RATE = 12; // pesos per kWh
const q = (s) => `'${String(s).replace(/'/g, "''")}'`;
const now = new Date();
/** The 1st of the month `monthsAgo` months back, as stored (UTC text). */
const monthStart = (monthsAgo) =>
  new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() - monthsAgo, 1, 2)).toISOString().replace('T', ' ').slice(0, 19);

const sql = [];
if (process.argv.includes('--reset')) {
  sql.push('DELETE FROM additional_charge;', 'DELETE FROM bill;', 'DELETE FROM electricity_reading;', 'DELETE FROM tenant;', 'DELETE FROM room;');
  sql.push('DELETE FROM sqlite_sequence;');
}
const receipts = [];
let reading = 0;
let charge = 0;
TENANTS.forEach((name, i) => {
  const room = i + 1;
  const rent = 4000 + room * 250;
  sql.push(`INSERT INTO room (id, name, rent) VALUES (${room}, ${q(`Room ${room}`)}, ${rent});`);
  sql.push(`INSERT INTO tenant (id, room_id, name, is_active, join_date) VALUES (${room}, ${room}, ${q(name)}, 1, ${q(monthStart(MONTHS + 1))});`);
  let meter = 1000 * room;
  for (let m = MONTHS; m >= 1; m--) {
    reading++;
    const consumption = 90 + ((room * 37 + m * 23) % 120);
    const at = monthStart(m - 1);
    sql.push(
      `INSERT INTO electricity_reading (id, tenant_id, room_id, prev_reading, curr_reading, consumption, created_at, updated_at) ` +
        `VALUES (${reading}, ${room}, ${room}, ${meter}, ${meter + consumption}, ${consumption}, ${q(at)}, ${q(at)});`,
    );
    meter += consumption;
    const water = m % 3 === 0 ? 150 : 0;
    const paid = m > 1;
    const file = `${Date.parse(`${at.replace(' ', 'T')}Z`) / 1000 + 5 * 86400}-r${reading}`;
    sql.push(
      `INSERT INTO bill (id, reading_id, tenant_id, room_charges, electric_charges, total_amount, receipt_url, paid, created_at, updated_at) ` +
        `VALUES (${reading}, ${reading}, ${room}, ${rent}, ${consumption * RATE}, ${rent + consumption * RATE + water}, ` +
        `${paid ? q(file) : 'NULL'}, ${paid ? 1 : 0}, ${q(at)}, ${q(at)});`,
    );
    if (water) {
      sql.push(
        `INSERT INTO additional_charge (id, bill_id, amount, description, created_at, updated_at) VALUES (${++charge}, ${reading}, ${water}, 'Water', ${q(at)}, ${q(at)});`,
      );
    }
    if (paid) receipts.push(`receipts/${name}/${file}`);
  }
});

const tmp = fs.mkdtempSync(path.join(os.tmpdir(), 'm18-dev-seed-'));
try {
  const sqlFile = path.join(tmp, 'seed.sql');
  fs.writeFileSync(sqlFile, `${sql.join('\n')}\n`);
  wrangler(['d1', 'execute', DB, '--env', 'development', '--remote', '--file', sqlFile, '--yes']);
  console.log(`✅ ${DB}: ${TENANTS.length} rooms, ${TENANTS.length} tenants, ${reading} readings and bills, ${charge} charges`);

  const put = (key, file) => wrangler(['r2', 'object', 'put', `${BUCKET}/${key}`, '--file', file, '--content-type', 'image/png', '--remote']);
  for (const [i, name] of ['bpi', 'gcash', 'maya'].entries()) {
    const file = path.join(tmp, `${name}.png`);
    fs.writeFileSync(file, fakeQr(i));
    put(`payments/${name}.png`, file);
  }
  const receiptFile = path.join(tmp, 'receipt.png');
  fs.writeFileSync(receiptFile, fakeReceipt());
  for (const key of receipts) put(key, receiptFile);
  console.log(`✅ ${BUCKET}: 3 payment images, ${receipts.length} receipts`);
  console.log(`Tenant links: https://development-my.m18-residences.workers.dev/<NAME> for ${TENANTS.join(', ')}`);
} finally {
  fs.rmSync(tmp, { recursive: true, force: true });
}
