import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';

const require = createRequire(import.meta.url);

const SETS = ['lucide', 'feather', 'simple-icons'];
// The catalog is split: the general-purpose UI sets (lucide + feather) are
// embedded in the wasm/binary; the ~3700 simple-icons brand logos are loaded at
// runtime (desktop: include_str at startup; web: fetched from the daemon) so
// they never bloat the wasm first-load.
const CORE_SETS = new Set(['lucide', 'feather']);
const OUT_DIR = path.resolve('crates/op-editor-ui/assets');
const OUT_CORE = path.join(OUT_DIR, 'iconify-catalog-core.json');
const OUT_BRANDS = path.join(OUT_DIR, 'iconify-catalog-brands.json');

function attr(tag, name) {
  const match = tag.match(new RegExp(`\\b${name}="([^"]*)"`));
  return match ? match[1] : undefined;
}

function num(value, fallback = 0) {
  if (value === undefined || value === '') return fallback;
  const parsed = Number(value);
  return Number.isFinite(parsed) ? parsed : fallback;
}

function circlePath(cx, cy, r) {
  return `M${cx - r} ${cy}A${r} ${r} 0 1 0 ${cx + r} ${cy}A${r} ${r} 0 1 0 ${cx - r} ${cy}Z`;
}

function ellipsePath(cx, cy, rx, ry) {
  return `M${cx - rx} ${cy}A${rx} ${ry} 0 1 0 ${cx + rx} ${cy}A${rx} ${ry} 0 1 0 ${cx - rx} ${cy}Z`;
}

function rectPath(x, y, w, h, r) {
  if (r <= 0) return `M${x} ${y}H${x + w}V${y + h}H${x}Z`;
  const rr = Math.min(r, w / 2, h / 2);
  return [
    `M${x + rr} ${y}`,
    `H${x + w - rr}`,
    `A${rr} ${rr} 0 0 1 ${x + w} ${y + rr}`,
    `V${y + h - rr}`,
    `A${rr} ${rr} 0 0 1 ${x + w - rr} ${y + h}`,
    `H${x + rr}`,
    `A${rr} ${rr} 0 0 1 ${x} ${y + h - rr}`,
    `V${y + rr}`,
    `A${rr} ${rr} 0 0 1 ${x + rr} ${y}`,
    'Z',
  ].join('');
}

function pointsPath(points, close) {
  const nums = points
    .trim()
    .split(/[\s,]+/)
    .map(Number)
    .filter(Number.isFinite);
  if (nums.length < 4) return null;
  const pairs = [];
  for (let i = 0; i + 1 < nums.length; i += 2) pairs.push([nums[i], nums[i + 1]]);
  return `M${pairs.map((p) => `${p[0]} ${p[1]}`).join('L')}${close ? 'Z' : ''}`;
}

function pathsFromBody(body) {
  const paths = [];
  for (const match of body.matchAll(/<path\b[^>]*?\bd="([^"]+)"[^>]*>/gi)) {
    paths.push(match[1]);
  }
  for (const match of body.matchAll(/<line\b[^>]*>/gi)) {
    const tag = match[0];
    paths.push(
      `M${num(attr(tag, 'x1'))} ${num(attr(tag, 'y1'))}L${num(attr(tag, 'x2'))} ${num(attr(tag, 'y2'))}`,
    );
  }
  for (const match of body.matchAll(/<circle\b[^>]*>/gi)) {
    const tag = match[0];
    paths.push(circlePath(num(attr(tag, 'cx')), num(attr(tag, 'cy')), num(attr(tag, 'r'))));
  }
  for (const match of body.matchAll(/<ellipse\b[^>]*>/gi)) {
    const tag = match[0];
    paths.push(
      ellipsePath(
        num(attr(tag, 'cx')),
        num(attr(tag, 'cy')),
        num(attr(tag, 'rx')),
        num(attr(tag, 'ry')),
      ),
    );
  }
  for (const match of body.matchAll(/<rect\b[^>]*>/gi)) {
    const tag = match[0];
    paths.push(
      rectPath(
        num(attr(tag, 'x')),
        num(attr(tag, 'y')),
        num(attr(tag, 'width')),
        num(attr(tag, 'height')),
        num(attr(tag, 'rx'), num(attr(tag, 'ry'))),
      ),
    );
  }
  for (const match of body.matchAll(/<polyline\b[^>]*?\bpoints="([^"]+)"[^>]*>/gi)) {
    const d = pointsPath(match[1], false);
    if (d) paths.push(d);
  }
  for (const match of body.matchAll(/<polygon\b[^>]*?\bpoints="([^"]+)"[^>]*>/gi)) {
    const d = pointsPath(match[1], true);
    if (d) paths.push(d);
  }
  for (let i = 1; i < paths.length; i += 1) {
    if (paths[i].startsWith('m')) paths[i] = `M${paths[i].slice(1)}`;
  }
  return paths;
}

function styleFromBody(body) {
  const hasStroke = /\bstroke=|\bstroke-width=|\bstroke-linecap=|\bfill="none"/i.test(body);
  const hasFill = /\bfill="(?!none)[^"]*"/i.test(body);
  return hasStroke && !hasFill ? 'stroke' : 'fill';
}

const icons = [];
for (const collection of SETS) {
  const data = require(`../node_modules/@iconify-json/${collection}/icons.json`);
  const names = Object.keys(data.icons).sort((a, b) => a.localeCompare(b));
  for (const name of names) {
    const icon = data.icons[name];
    const paths = pathsFromBody(icon.body);
    if (paths.length === 0) continue;
    icons.push({
      collection,
      name,
      width: icon.width ?? data.width ?? 24,
      height: icon.height ?? data.height ?? 24,
      style: styleFromBody(icon.body),
      d: paths.join(' '),
    });
  }
}

for (const [alias, target] of Object.entries({ home: 'house', unlock: 'lock-open' })) {
  if (
    !icons.some((icon) => icon.collection === 'lucide' && icon.name === alias) &&
    icons.some((icon) => icon.collection === 'lucide' && icon.name === target)
  ) {
    const source = icons.find((icon) => icon.collection === 'lucide' && icon.name === target);
    icons.push({ ...source, name: alias });
  }
}

icons.sort((a, b) => {
  const setDelta = SETS.indexOf(a.collection) - SETS.indexOf(b.collection);
  return setDelta || a.name.localeCompare(b.name);
});

const coreIcons = icons.filter((icon) => CORE_SETS.has(icon.collection));
const brandIcons = icons.filter((icon) => !CORE_SETS.has(icon.collection));

fs.mkdirSync(OUT_DIR, { recursive: true });
fs.writeFileSync(OUT_CORE, JSON.stringify({ icons: coreIcons }));
fs.writeFileSync(OUT_BRANDS, JSON.stringify({ icons: brandIcons }));
console.log(`wrote ${coreIcons.length} core icons to ${OUT_CORE}`);
console.log(`wrote ${brandIcons.length} brand icons to ${OUT_BRANDS}`);
