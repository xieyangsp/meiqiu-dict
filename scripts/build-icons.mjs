// Icon pipeline. Reads two source PNGs and emits every size Tauri and the
// tray need.
//
// Sources (square, RGBA, ideally >= 1024):
//   src-tauri/icons/source-active.png   (active / "capture on" / app brand)
//   src-tauri/icons/source-idle.png     (idle / "capture off")
//
// Outputs (all under src-tauri/icons/):
//   icon-256.png  icon-512.png  icon-1024.png            (raw scaled exports)
//   32x32.png  128x128.png  128x128@2x.png  icon.png     (Tauri bundle defaults)
//   icon.ico                                             (Windows multi-size)
//   tray-active.png  tray-idle.png                       (32x32 tray)

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

// Sources are pre-scaled pixel art at 2048; lanczos preserves features when
// shrinking, nearest would lose tiny details like the mouth at 32x32.
const RESIZE_OPTS = { kernel: sharp.kernel.lanczos3, fit: 'contain', background: { r: 0, g: 0, b: 0, alpha: 0 } };

// Tray active-state circle background. Purple stands out against typical
// dark/light taskbars; idle has no background so the two states differ
// strongly even at 16-24px.
const TRAY_ACTIVE_CIRCLE = '#8b5cf6';

// App icons trim the transparent margin and scale so the longer side
// fills the canvas; fit:'contain' pads the shorter side with transparency.
// The dog face is wider than tall, so without trim every export ends up
// shrunk to fit a square inner area.
function tightPipeline(source, size) {
  return sharp(source).trim().resize(size, size, RESIZE_OPTS);
}

async function exportPng(source, size, outName) {
  const out = resolve(iconsDir, outName);
  await tightPipeline(source, size).png({ compressionLevel: 9 }).toFile(out);
  console.log('wrote', out);
}

async function exportIco(source, sizes, outName) {
  // png-to-ico accepts an array of PNG buffers, one per embedded size.
  const buffers = await Promise.all(
    sizes.map((s) => tightPipeline(source, s).png({ compressionLevel: 9 }).toBuffer()),
  );
  const ico = await pngToIco(buffers);
  const out = resolve(iconsDir, outName);
  writeFileSync(out, ico);
  console.log('wrote', out);
}

// Tray export keeps the source's natural transparent margin (no trim) so
// the artwork sits comfortably inside the inscribed circle. When a circle
// color is given, the artwork is composited over a filled SVG circle that
// fills the canvas.
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
  // App icon set: derive from active variant.
  await exportPng(sourceActive, 256, 'icon-256.png');
  await exportPng(sourceActive, 512, 'icon-512.png');
  await exportPng(sourceActive, 1024, 'icon-1024.png');

  // Tauri bundle defaults.
  await exportPng(sourceActive, 32, '32x32.png');
  await exportPng(sourceActive, 128, '128x128.png');
  await exportPng(sourceActive, 256, '128x128@2x.png');
  await exportPng(sourceActive, 512, 'icon.png');

  // Windows multi-size ICO.
  await exportIco(sourceActive, [16, 24, 32, 48, 64, 128, 256], 'icon.ico');

  // Tray two-state icons: active has a purple circle, idle stays transparent.
  await exportTrayPng(sourceActive, 32, 'tray-active.png', TRAY_ACTIVE_CIRCLE);
  await exportTrayPng(sourceIdle, 32, 'tray-idle.png', null);
}

await main();

