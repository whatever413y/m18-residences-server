// Re-encodes the stored receipt images as WebP, like the admin app does for new uploads
// (long edge at most 1600 px, quality 80, EXIF orientation applied, first frame only).
//   node backfill.mjs            dry run: downloads and converts in memory, writes nothing
//   node backfill.mjs --apply    for each object that gets smaller: copies the original to
//                                receipts-originals/<key> (deleted after 30 days by the bucket's
//                                lifecycle rule), then overwrites <key> with the WebP.
// Keys never change, so the database doesn't either. WebP, PDF and anything that wouldn't shrink
// are left untouched; rerunning skips what is already WebP.
// Credentials: CLOUDFLARE_ACCOUNT_ID and CLOUDFLARE_API_TOKEN (needs R2 Edit) from the secrets file
// ($M18_SECRETS_FILE, default ../../../m18-residences-infra/.env), used as S3 credentials
// (access key = token id, secret = SHA-256 of the token). Bucket: $R2_BUCKET or m18-residences.
import { CopyObjectCommand, GetObjectCommand, HeadObjectCommand, ListObjectsV2Command, PutObjectCommand, S3Client } from '@aws-sdk/client-s3';
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import sharp from 'sharp';

const apply = process.argv.includes('--apply');
const bucket = process.env.R2_BUCKET ?? 'm18-residences';
const PREFIX = 'receipts/';
const BACKUP_PREFIX = 'receipts-originals/';
const MAX_EDGE = 1600;
const QUALITY = 80;
const CONCURRENCY = 4;

const here = path.dirname(fileURLToPath(import.meta.url));
const secretsFile = process.env.M18_SECRETS_FILE ?? path.join(here, '..', '..', '..', 'm18-residences-infra', '.env');
const secrets = Object.fromEntries(
  fs
    .readFileSync(secretsFile, 'utf8')
    .split(/\r?\n/)
    .map((l) => l.match(/^\s*([A-Z_][A-Z0-9_]*)\s*=\s*(.*)$/))
    .filter(Boolean)
    .map((m) => [m[1], m[2].trim().replace(/^["']|["']$/g, '')]),
);
for (const name of ['CLOUDFLARE_ACCOUNT_ID', 'CLOUDFLARE_API_TOKEN']) {
  if (!secrets[name]) throw new Error(`${name} is missing in ${secretsFile}`);
}

const verify = await fetch('https://api.cloudflare.com/client/v4/user/tokens/verify', {
  headers: { Authorization: `Bearer ${secrets.CLOUDFLARE_API_TOKEN}` },
}).then((r) => r.json());
if (!verify.success) throw new Error(`Cloudflare token check failed: ${JSON.stringify(verify.errors)}`);

const s3 = new S3Client({
  region: 'auto',
  endpoint: `https://${secrets.CLOUDFLARE_ACCOUNT_ID}.r2.cloudflarestorage.com`,
  credentials: {
    accessKeyId: verify.result.id,
    secretAccessKey: createHash('sha256').update(secrets.CLOUDFLARE_API_TOKEN).digest('hex'),
  },
});

/** The same magic-byte check as the server (crates/shared_rs/src/files.rs). */
function sniff(b) {
  const ascii = (from, to) => b.subarray(from, to).toString('latin1');
  if (b[0] === 0xff && b[1] === 0xd8 && b[2] === 0xff) return 'image/jpeg';
  if (ascii(0, 8) === '\x89PNG\r\n\x1a\n') return 'image/png';
  if (ascii(0, 4) === 'GIF8') return 'image/gif';
  if (ascii(0, 4) === 'RIFF' && ascii(8, 12) === 'WEBP') return 'image/webp';
  if (ascii(4, 8) === 'ftyp' && ['avif', 'avis'].includes(ascii(8, 12))) return 'image/avif';
  if (ascii(0, 5) === '%PDF-') return 'application/pdf';
  return 'unknown';
}
const CONVERTIBLE = new Set(['image/jpeg', 'image/png', 'image/gif', 'image/avif']);

async function listAll(prefix) {
  const objects = [];
  let token;
  do {
    const page = await s3.send(new ListObjectsV2Command({ Bucket: bucket, Prefix: prefix, ContinuationToken: token }));
    objects.push(...(page.Contents ?? []));
    token = page.IsTruncated ? page.NextContinuationToken : undefined;
  } while (token);
  return objects;
}

const copySource = (key) => `${bucket}/${key.split('/').map(encodeURIComponent).join('/')}`;

async function processObject({ Key: key, Size: size }) {
  const got = await s3.send(new GetObjectCommand({ Bucket: bucket, Key: key }));
  const original = Buffer.from(await got.Body.transformToByteArray());
  const type = sniff(original);
  const row = { key, type, size, storedType: got.ContentType, newSize: size, action: 'kept' };
  if (!CONVERTIBLE.has(type)) return { ...row, action: type === 'unknown' ? 'unknown type' : 'kept' };

  const webp = await sharp(original)
    .rotate()
    .resize({ width: MAX_EDGE, height: MAX_EDGE, fit: 'inside', withoutEnlargement: true })
    .webp({ quality: QUALITY })
    .toBuffer();
  if (webp.length >= original.length) return { ...row, action: 'not smaller' };
  if (!apply) return { ...row, newSize: webp.length, action: 'would convert' };

  await s3.send(new CopyObjectCommand({ Bucket: bucket, Key: BACKUP_PREFIX + key, CopySource: copySource(key) }));
  const backup = await s3.send(new HeadObjectCommand({ Bucket: bucket, Key: BACKUP_PREFIX + key }));
  if (backup.ContentLength !== original.length) throw new Error(`${key}: backup is ${backup.ContentLength} bytes, original ${original.length}`);
  await s3.send(new PutObjectCommand({ Bucket: bucket, Key: key, Body: webp, ContentType: 'image/webp' }));
  return { ...row, newSize: webp.length, action: 'converted' };
}

const objects = await listAll(PREFIX);
console.log(`${apply ? 'APPLY' : 'DRY RUN'}: ${objects.length} objects under ${bucket}/${PREFIX}`);

const results = [];
const failures = [];
let next = 0;
await Promise.all(
  Array.from({ length: CONCURRENCY }, async () => {
    while (next < objects.length) {
      const object = objects[next++];
      try {
        results.push(await processObject(object));
      } catch (error) {
        failures.push({ key: object.Key, error: error.message });
        console.error(`❌ ${object.Key}: ${error.message}`);
      }
    }
  }),
);

const mb = (n) => `${(n / 1024 / 1024).toFixed(1)} MB`;
const groups = Map.groupBy(results, (r) => `${r.type} → ${r.action}`);
for (const [group, rows] of [...groups].sort()) {
  const before = rows.reduce((s, r) => s + r.size, 0);
  const after = rows.reduce((s, r) => s + r.newSize, 0);
  console.log(`${group}: ${rows.length} files, ${mb(before)} → ${mb(after)}`);
}
const mismatched = results.filter((r) => r.storedType !== r.type && r.action !== 'converted');
for (const [pair, rows] of Map.groupBy(mismatched, (r) => `stored as ${r.storedType}, bytes are ${r.type}`)) {
  console.log(`note: ${rows.length} object(s) ${pair}`);
}
const before = results.reduce((s, r) => s + r.size, 0);
const after = results.reduce((s, r) => s + r.newSize, 0);
console.log(`total: ${mb(before)} → ${mb(after)} (${before ? Math.round((1 - after / before) * 100) : 0}% smaller)`);
if (failures.length) {
  console.error(`${failures.length} failure(s); rerun to retry (finished objects are skipped as WebP)`);
  process.exit(1);
}
