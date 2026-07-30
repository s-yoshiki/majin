/**
 * Draws the source app icon as a PNG, with no image-library dependency.
 *
 * Committing only the generated icons would leave no way to tweak the artwork,
 * so the drawing lives here and `pnpm --filter @majin/desktop icon`
 * regenerates every size through the Tauri CLI.
 */

import { writeFileSync } from 'node:fs';
import { deflateSync } from 'node:zlib';

const SIZE = 1024;
const SAMPLES = 3; // supersampling factor per axis

const BG = [0x11, 0x14, 0x1a];
const BORDER = [0x26, 0x2c, 0x38];
const ACCENT = [0x7a, 0xa2, 0xf7];

/** Signed distance to a rounded rectangle centred on the canvas. */
function roundedBoxDistance(x, y, halfW, halfH, radius) {
  const dx = Math.abs(x) - halfW + radius;
  const dy = Math.abs(y) - halfH + radius;
  const outside = Math.hypot(Math.max(dx, 0), Math.max(dy, 0));
  return outside + Math.min(Math.max(dx, dy), 0) - radius;
}

/** Distance from a point to a line segment, giving capsules with round caps. */
function segmentDistance(px, py, ax, ay, bx, by) {
  const abx = bx - ax;
  const aby = by - ay;
  const t = Math.max(0, Math.min(1, ((px - ax) * abx + (py - ay) * aby) / (abx * abx + aby * aby)));
  return Math.hypot(px - (ax + abx * t), py - (ay + aby * t));
}

function mix(base, layer, alpha) {
  return [
    base[0] + (layer[0] - base[0]) * alpha,
    base[1] + (layer[1] - base[1]) * alpha,
    base[2] + (layer[2] - base[2]) * alpha,
  ];
}

/** Colour of a single sample point, in canvas coordinates. */
function sample(x, y) {
  const cx = x - SIZE / 2;
  const cy = y - SIZE / 2;

  const plate = roundedBoxDistance(cx, cy, SIZE * 0.44, SIZE * 0.44, SIZE * 0.16);
  if (plate > 0) return null; // transparent outside the plate

  let colour = plate > -6 ? BORDER : BG;

  // A `❯` prompt chevron, drawn as two capsules.
  const stroke = SIZE * 0.045;
  const chevron = Math.min(
    segmentDistance(cx, cy, -SIZE * 0.2, -SIZE * 0.17, -SIZE * 0.02, 0),
    segmentDistance(cx, cy, -SIZE * 0.02, 0, -SIZE * 0.2, SIZE * 0.17),
  );
  colour = mix(colour, ACCENT, coverage(chevron - stroke));

  // The cursor underscore next to it.
  const underscore = segmentDistance(cx, cy, SIZE * 0.06, SIZE * 0.17, SIZE * 0.22, SIZE * 0.17);
  colour = mix(colour, ACCENT, coverage(underscore - stroke));

  return colour;
}

/** Converts a signed distance into 0..1 coverage with a soft edge. */
function coverage(distance) {
  return Math.max(0, Math.min(1, 0.5 - distance));
}

function render() {
  const pixels = Buffer.alloc(SIZE * SIZE * 4);
  const step = 1 / SAMPLES;

  for (let y = 0; y < SIZE; y += 1) {
    for (let x = 0; x < SIZE; x += 1) {
      let r = 0;
      let g = 0;
      let b = 0;
      let a = 0;

      for (let sy = 0; sy < SAMPLES; sy += 1) {
        for (let sx = 0; sx < SAMPLES; sx += 1) {
          const colour = sample(x + (sx + 0.5) * step, y + (sy + 0.5) * step);
          if (colour) {
            r += colour[0];
            g += colour[1];
            b += colour[2];
            a += 255;
          }
        }
      }

      const total = SAMPLES * SAMPLES;
      const offset = (y * SIZE + x) * 4;
      // Premultiplied averaging would darken the edges; divide by the covered
      // sample count instead so edge pixels keep their own colour.
      const covered = a / 255;
      pixels[offset] = covered ? Math.round(r / covered) : 0;
      pixels[offset + 1] = covered ? Math.round(g / covered) : 0;
      pixels[offset + 2] = covered ? Math.round(b / covered) : 0;
      pixels[offset + 3] = Math.round(a / total);
    }
  }

  return pixels;
}

// --- Minimal PNG encoder ----------------------------------------------------

function crc32(buffer) {
  let crc = 0xffffffff;
  for (const byte of buffer) {
    crc ^= byte;
    for (let bit = 0; bit < 8; bit += 1) {
      crc = crc & 1 ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1;
    }
  }
  return (crc ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length);
  const body = Buffer.concat([Buffer.from(type, 'ascii'), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body));
  return Buffer.concat([length, body, crc]);
}

function encodePng(pixels) {
  const header = Buffer.alloc(13);
  header.writeUInt32BE(SIZE, 0);
  header.writeUInt32BE(SIZE, 4);
  header[8] = 8; // bit depth
  header[9] = 6; // colour type: RGBA
  header[10] = 0;
  header[11] = 0;
  header[12] = 0;

  // Each scanline is prefixed with its filter type; 0 means "none".
  const raw = Buffer.alloc(SIZE * (SIZE * 4 + 1));
  for (let y = 0; y < SIZE; y += 1) {
    raw[y * (SIZE * 4 + 1)] = 0;
    pixels.copy(raw, y * (SIZE * 4 + 1) + 1, y * SIZE * 4, (y + 1) * SIZE * 4);
  }

  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk('IHDR', header),
    chunk('IDAT', deflateSync(raw, { level: 9 })),
    chunk('IEND', Buffer.alloc(0)),
  ]);
}

const target = new URL('../app-icon.png', import.meta.url);
writeFileSync(target, encodePng(render()));
console.log(`wrote ${target.pathname} (${SIZE}x${SIZE})`);
