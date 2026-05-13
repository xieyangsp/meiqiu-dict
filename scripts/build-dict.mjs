// Build src-tauri/resources/ecdict.db from the ECDICT csv.
//
// Source : scripts/data/ecdict.csv (gitignored; ~63MB, ~770k rows)
// Output : src-tauri/resources/ecdict.db (gitignored; bundled at build time)
//
// Schema is intentionally tiny. lang_src/lang_tgt are cheap seams reserved
// for future non-en-zh dictionaries (per AGENTS.md), but ECDICT is uniformly
// en-zh.
//
// Uses node:sqlite (built into Node >= 22) so the script has no native build step.

import { createReadStream, mkdirSync, statSync, existsSync, rmSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { DatabaseSync } from 'node:sqlite';

import { parse } from 'csv-parse';

const __dirname = dirname(fileURLToPath(import.meta.url));
const csvPath = resolve(__dirname, 'data', 'ecdict.csv');
const outDir = resolve(__dirname, '..', 'src-tauri', 'resources');
const outPath = resolve(outDir, 'ecdict.db');

if (!existsSync(csvPath)) {
  console.error(`ecdict.csv not found at ${csvPath}`);
  console.error('Place the ECDICT csv there (see https://github.com/skywind3000/ECDICT).');
  process.exit(1);
}

mkdirSync(outDir, { recursive: true });
if (existsSync(outPath)) rmSync(outPath);

const db = new DatabaseSync(outPath);
db.exec('PRAGMA journal_mode = OFF');
db.exec('PRAGMA synchronous = OFF');
db.exec('PRAGMA locking_mode = EXCLUSIVE');
db.exec('PRAGMA temp_store = MEMORY');

db.exec(`
  CREATE TABLE entries (
    word TEXT PRIMARY KEY,
    phonetic TEXT NOT NULL DEFAULT '',
    translation TEXT NOT NULL DEFAULT '',
    lang_src TEXT NOT NULL,
    lang_tgt TEXT NOT NULL
  ) WITHOUT ROWID;
`);

const insert = db.prepare(
  `INSERT OR IGNORE INTO entries (word, phonetic, translation, lang_src, lang_tgt)
   VALUES (?, ?, ?, 'en', 'zh')`,
);

const parser = createReadStream(csvPath, { encoding: 'utf8' }).pipe(
  parse({
    columns: true,
    skip_empty_lines: true,
    relax_quotes: true,
    relax_column_count: true,
  }),
);

const t0 = Date.now();
let total = 0;
let inserted = 0;

// Batch into transactions of 5k rows for throughput. node:sqlite has no
// transaction() helper, so drive BEGIN/COMMIT manually.
const BATCH = 5000;
let batch = [];
function flush() {
  if (!batch.length) return;
  db.exec('BEGIN');
  try {
    for (const row of batch) {
      const info = insert.run(row[0], row[1], row[2]);
      if (info.changes) inserted += 1;
    }
    db.exec('COMMIT');
  } catch (e) {
    db.exec('ROLLBACK');
    throw e;
  }
  batch = [];
}

for await (const row of parser) {
  total += 1;
  const word = String(row.word ?? '').trim().toLowerCase();
  if (!word) continue;
  const translation = String(row.translation ?? '').trim();
  if (!translation) continue;
  batch.push([word, String(row.phonetic ?? '').trim(), translation]);
  if (batch.length >= BATCH) flush();
}
flush();

db.exec('VACUUM');
db.exec('ANALYZE');
db.close();

const ms = Date.now() - t0;
const mb = (statSync(outPath).size / 1024 / 1024).toFixed(1);
console.log(`wrote ${outPath}`);
console.log(`rows: ${inserted} kept / ${total} read, size: ${mb} MiB, time: ${ms} ms`);
