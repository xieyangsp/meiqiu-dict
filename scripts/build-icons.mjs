import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import sharp from 'sharp';
import pngToIco from 'png-to-ico';

const __dirname = dirname(fileURLToPath(import.meta.url));
const iconsDir = resolve(__dirname, '..', 'src-tauri', 'icons');
mkdirSync(iconsDir, { recursive: true });

const sourceActive = resolve(iconsDir, 'source-active.png');
const sourceIdle = resolve(iconsDir, 'source-idle.png');

// Lanczos preserves features when shrinking 2048px sources to tray sizes.
const RESIZE_OPTS = { kernel: sharp.kernel.lanczos3, fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } };

// Purple stands out against typical dark/light taskbars; idle has no background.
const TRAY_ACTIVE_CIRCLE = '#8b5cf6';

// Trim the transparent margin so the artwork fills the canvas instead of
// being padded by fit:'contain'; the source aspect ratio is wider than tall.
function tightPipeline(source, size) {
  return sharp(source).trim().resize(size, size, RESIZE_OPTS);
}

async function exportPng(source, size, outName) {
  const out = resolve(iconsDir, outName);
  await tightPipeline(source, size).png({ compressionLevel: 9 }).toFile(out);
  console.log('wrote', out);
}

async function exportIco(source, sizes, outName) {
  const buffers = await Promise.all(
    sizes.map((s) => tightPipeline(source, s).png({ compressionLevel: 9 }).toBuffer()),
  );
  const ico = await pngToIco(buffers);
  const out = resolve(iconsDir, outName);
  writeFileSync(out, ico);
  console.log('wrote', out);
}

// Tray keeps the source's natural margin (no trim) so it sits inside an
// inscribed circle; a fill SVG is composited beneath when circleColor is set.
async function exportTrayPng(source, size, outName, circleColor) {
  const artwork = await sharp(source)
    .resize(size, size, RESIZE_OPTS)
    .png({ compressionLevel: 9 })
    .toBuffer();

  const out = resolve(iconsDir, outName);
  if (circleColor) {
    const r = size / 2;
    const circle = Buffer.from(
      `<svg xmlns="http://www.w3.org/2000/svg" width="${size}" height="${size}"><circle cx="${r}" cy="${r}" r="${r}" fill="${circleColor}"/></svg>`,
    );
    await sharp(circle)
      .composite([{ input: artwork }])
      .png({ compressionLevel: 9 })
      .toFile(out);
  } else {
    writeFileSync(out, artwork);
  }
  console.log('wrote', out);
}

async function main() {
  await exportPng(sourceActive, 256, 'icon-256.png');
  await exportPng(sourceActive, 512, 'icon-512.png');
  await exportPng(sourceActive, 1024, 'icon-1024.png');

  await exportPng(sourceActive, 32, '32x32.png');
  await exportPng(sourceActive, 128, '128x128.png');
  await exportPng(sourceActive, 256, '128x128@2x.png');
  await exportPng(sourceActive, 512, 'icon.png');

  await exportIco(sourceActive, [16, 24, 32, 48, 64, 128, 256], 'icon.ico');

  await exportTrayPng(sourceActive, 32, 'tray-active.png', TRAY_ACTIVE_CIRCLE);
  await exportTrayPng(sourceIdle, 32, 'tray-idle.png', null);
}

await main();

