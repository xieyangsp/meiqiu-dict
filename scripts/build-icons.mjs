// Placeholder icon generator. Produces:
//   src-tauri/icons/icon-256.png  icon-512.png  icon-1024.png  icon.ico
//   src-tauri/icons/tray-active.png  tray-idle.png  (32x32)
// Uses Node built-ins only. Replace source.png with the real artwork and
// drop this script afterwards.

import { deflateSync } from 'node:zlib';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const outDir = resolve(__dirname, '..', 'src-tauri', 'icons');
mkdirSync(outDir, { recursive: true });

// ---- PNG encoder (zero deps) ----

function crc32(buf) {
  let c;
  if (!crc32.table) {
    const t = new Uint32Array(256);
    for (let n = 0; n < 256; n++) {
      c = n;
      for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
      t[n] = c >>> 0;
    }
    crc32.table = t;
  }
  let crc = 0xffffffff;
  for (let i = 0; i < buf.length; i++) crc = (crc >>> 8) ^ crc32.table[(crc ^ buf[i]) & 0xff];
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const len = Buffer.alloc(4);
  len.writeUInt32BE(data.length, 0);
  const typeBuf = Buffer.from(type, 'ascii');
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(Buffer.concat([typeBuf, data])), 0);
  return Buffer.concat([len, typeBuf, data, crc]);
}

function encodePng(width, height, rgba) {
  const sig = Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;   // bit depth
  ihdr[9] = 6;   // color type: RGBA
  ihdr[10] = 0;
  ihdr[11] = 0;
  ihdr[12] = 0;
  const stride = width * 4;
  const filtered = Buffer.alloc((stride + 1) * height);
  for (let y = 0; y < height; y++) {
    filtered[y * (stride + 1)] = 0;
    rgba.copy(filtered, y * (stride + 1) + 1, y * stride, y * stride + stride);
  }
  const idat = deflateSync(filtered);
  return Buffer.concat([sig, chunk('IHDR', ihdr), chunk('IDAT', idat), chunk('IEND', Buffer.alloc(0))]);
}

// ---- Pixel generation ----

function fillCircle(size, [r, g, b]) {
  const buf = Buffer.alloc(size * size * 4, 0);
  const cx = (size - 1) / 2;
  const cy = (size - 1) / 2;
  const radius = size / 2 - 0.5;
  for (let y = 0; y < size; y++) {
    for (let x = 0; x < size; x++) {
      const dx = x - cx;
      const dy = y - cy;
      const d = Math.sqrt(dx * dx + dy * dy);
      // Edge anti-alias: 1px gradient at the rim.
      const edge = radius - d;
      let alpha;
      if (edge >= 1) alpha = 255;
      else if (edge <= 0) alpha = 0;
      else alpha = Math.round(edge * 255);
      const i = (y * size + x) * 4;
      buf[i] = r;
      buf[i + 1] = g;
      buf[i + 2] = b;
      buf[i + 3] = alpha;
    }
  }
  return buf;
}

// ---- ICO wrapper (embeds a PNG) ----

function encodeIco(pngBuf, size) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(0, 0);   // reserved
  header.writeUInt16LE(1, 2);   // type icon
  header.writeUInt16LE(1, 4);   // count
  const entry = Buffer.alloc(16);
  entry[0] = size >= 256 ? 0 : size;
  entry[1] = size >= 256 ? 0 : size;
  entry[2] = 0; // palette
  entry[3] = 0; // reserved
  entry.writeUInt16LE(1, 4);    // planes
  entry.writeUInt16LE(32, 6);   // bit count
  entry.writeUInt32LE(pngBuf.length, 8);
  entry.writeUInt32LE(22, 12);  // offset
  return Buffer.concat([header, entry, pngBuf]);
}

// ---- Write files ----

const BLACK = [20, 20, 20];
const GRAY = [160, 160, 160];

function write(name, buf) {
  const p = resolve(outDir, name);
  writeFileSync(p, buf);
  console.log('wrote', p);
}

// App icon (active dark variant)
for (const size of [256, 512, 1024]) {
  write(`icon-${size}.png`, encodePng(size, size, fillCircle(size, BLACK)));
}
const ico256 = encodePng(256, 256, fillCircle(256, BLACK));
write('icon.ico', encodeIco(ico256, 256));

// Default sizes Tauri expects.
write('32x32.png', encodePng(32, 32, fillCircle(32, BLACK)));
write('128x128.png', encodePng(128, 128, fillCircle(128, BLACK)));
write('128x128@2x.png', encodePng(256, 256, fillCircle(256, BLACK)));
write('icon.png', encodePng(512, 512, fillCircle(512, BLACK)));

// Tray two-state icons.
write('tray-active.png', encodePng(32, 32, fillCircle(32, BLACK)));
write('tray-idle.png', encodePng(32, 32, fillCircle(32, GRAY)));
