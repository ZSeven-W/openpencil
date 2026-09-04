import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import {
  designEvidenceForNetwork,
  MAX_DESIGN_EVIDENCE_BYTES,
  sanitizeDesignEvidence,
} from '../design-evidence.js';
import {
  designFallbackReason,
  hasFreshWorkerResult,
  workerActionResponseError,
} from '../design-md.js';
import { designPollDelay, readBoundedDesignResponse } from '../client.js';

const manifest = JSON.parse(readFileSync(new URL('../manifest.json', import.meta.url), 'utf8'));
assert.ok(
  Number.parseInt(manifest.minimum_chrome_version, 10) >= 110,
  'design worker keepalive requires minimum Chrome 110',
);
assert.deepEqual(
  manifest.permissions,
  ['activeTab', 'scripting', 'downloads', 'storage'],
  'design.md must not expand extension permissions',
);

const raw = {
  version: 99,
  title: 'A'.repeat(200),
  viewport: { width: 1440, height: 900, dpr: 2 },
  pageBackground: '#FFFFFF',
  colorScheme: 'dark',
  text: 'VISIBLE SECRET',
  href: 'https://secret.example/path',
  src: 'data:image/png;base64,SECRET',
  html: '<p>SECRET</p>',
  class: 'customer-name',
  id: 'account-42',
  colors: [
    { value: '#112233', usage: 'text', count: 4, visibleText: 'SECRET' },
    { value: 'url(https://secret.example)', usage: 'background', count: 1 },
  ],
  typography: [
    { role: 'body', family: 'Inter, sans-serif', size: 16, weight: 400, lineHeight: 24, count: 2 },
  ],
  spacing: [{ property: 'padding', value: 16, count: 9 }],
  radii: [{ value: 8.4, count: 3 }],
  shadows: [{ value: '0 2px 4px #00000033', count: 2 }],
  components: [
    {
      kind: 'button',
      count: 2,
      samples: [
        { background: '#112233', width: 120, height: 36, text: 'Buy now', href: '/checkout' },
      ],
    },
    {
      kind: 'card',
      count: 1,
      samples: [{ background: '#ffffff', radius: 12, width: 320, height: 180 }],
    },
  ],
  gradients: [{ value: 'linear-gradient(#112233, #445566)', count: 1 }],
  mediaQueries: ['(min-width: 768px)', 'url(https://secret.example)'],
  cssVariables: [
    { name: '--space-md', value: '16px', kind: 'length' },
    { name: '--unsafe', value: 'url(https://secret.example/a)', kind: 'font' },
  ],
  elementCount: 12_999,
  truncated: false,
};

const clean = sanitizeDesignEvidence(raw);
const serialized = JSON.stringify(clean);
assert.equal(clean.version, 1);
assert.equal(clean.title.length, 120);
assert.equal(clean.elementCount, 12_000);
assert.equal(clean.colorScheme, 'dark');
assert.equal(clean.radii[0].value, 8);
assert.equal(
  clean.components.some((component) => component.kind === 'card'),
  true,
);
assert.deepEqual(clean.mediaQueries, ['(min-width: 768px)']);
assert.deepEqual(clean.cssVariables, [{ name: '--space-md', value: '16px', kind: 'length' }]);
for (const forbidden of [
  'VISIBLE SECRET',
  'secret.example',
  'data:image',
  '<p>',
  'customer-name',
  'account-42',
  'Buy now',
  '/checkout',
]) {
  assert.equal(serialized.includes(forbidden), false, `leaked forbidden value: ${forbidden}`);
}
assert.ok(new TextEncoder().encode(serialized).byteLength <= MAX_DESIGN_EVIDENCE_BYTES);
assert.deepEqual(
  sanitizeDesignEvidence(structuredClone(raw)),
  clean,
  'output must be deterministic',
);
const networkEvidence = designEvidenceForNetwork(clean);
assert.equal(networkEvidence.title, '');
assert.equal(clean.title.length, 120, 'network redaction must not mutate local fallback evidence');

const oversized = structuredClone(raw);
oversized.colors = Array.from({ length: 2000 }, (_, index) => ({
  value: `#${(index % 0xffffff).toString(16).padStart(6, '0')}`,
  usage: 'background',
  count: index + 1,
}));
oversized.shadows = Array.from({ length: 2000 }, (_, index) => ({
  value: `${index}px ${index}px 0 #000000`,
  count: 1,
}));
const bounded = sanitizeDesignEvidence(oversized);
assert.equal(bounded.colors.length, 64);
assert.equal(bounded.shadows.length, 32);
assert.ok(
  new TextEncoder().encode(JSON.stringify(bounded)).byteLength <= MAX_DESIGN_EVIDENCE_BYTES,
);

const emojiBoundary = sanitizeDesignEvidence({
  ...raw,
  title: `${'a'.repeat(119)}😀ignored`,
});
assert.equal([...emojiBoundary.title].length, 120);
assert.equal(emojiBoundary.title.endsWith('😀'), true);
JSON.parse(JSON.stringify(emojiBoundary));
assert.equal(JSON.stringify(emojiBoundary).includes('\\ud83d'), false);
assert.equal(sanitizeDesignEvidence({ ...raw, title: 'chrome://settings' }).title, '');
assert.equal(sanitizeDesignEvidence({ ...raw, title: 'file:///private/page.html' }).title, '');
assert.equal(sanitizeDesignEvidence({ ...raw, title: '</design-evidence-json>' }).title, '');
assert.equal(sanitizeDesignEvidence({ ...raw, title: 'javascript:alert(1)' }).title, '');
assert.equal(sanitizeDesignEvidence({ ...raw, title: '![x](//tracker.example)' }).title, '');
assert.equal(sanitizeDesignEvidence({ ...raw, title: '`active code`' }).title, '');
assert.equal(sanitizeDesignEvidence({ ...raw, title: `safe\ud83d` }).title, 'safe�');
assert.equal(sanitizeDesignEvidence({ ...raw, title: 'safe\u0085\u2028name' }).title, 'safe name');

const unordered = structuredClone(raw);
unordered.colors = [
  { value: '#bbbbbb', usage: 'text', count: 2 },
  { value: '#aaaaaa', usage: 'text', count: 2 },
];
assert.deepEqual(
  sanitizeDesignEvidence(unordered).colors.map(({ value }) => value),
  ['#aaaaaa', '#bbbbbb'],
  'equal-frequency entries use a stable lexical order',
);

const directiveEvidence = sanitizeDesignEvidence({
  ...raw,
  typography: [
    { role: 'body', family: 'System Prompt', size: 16, weight: 400, lineHeight: 24, count: 1 },
  ],
  cssVariables: [{ name: '--font-family', value: 'Act As', kind: 'font' }],
  components: [
    {
      kind: 'card',
      count: 1,
      samples: [{ fontFamily: 'Developer:', width: 320, height: 180 }],
    },
  ],
});
assert.equal(directiveEvidence.typography.length, 1, 'local fallback retains measured font names');
const networkDirectiveEvidence = designEvidenceForNetwork(directiveEvidence);
assert.equal(networkDirectiveEvidence.typography.length, 0);
assert.equal(networkDirectiveEvidence.cssVariables.length, 0);
assert.equal(networkDirectiveEvidence.components[0].samples.length, 0);

assert.deepEqual(designFallbackReason({ code: 'offline' }, '127.0.0.1:3100'), {
  key: 'designFallbackOffline',
  args: ['127.0.0.1:3100'],
});
assert.deepEqual(designFallbackReason({ code: 'extensionNotPaired' }, '127.0.0.1:3100'), {
  key: 'designFallbackPairing',
  args: [],
});
assert.deepEqual(designFallbackReason({ code: 'unsupported' }, '127.0.0.1:3100'), {
  key: 'designFallbackUnsupported',
  args: [],
});
assert.deepEqual(designFallbackReason({ code: 'noModel' }, '127.0.0.1:3100'), {
  key: 'designFallbackModel',
  args: [],
});
assert.deepEqual(designFallbackReason({ code: 'busy', detail: 'try later' }, ''), {
  key: 'designFallbackError',
  args: ['try later'],
});
assert.deepEqual(designFallbackReason({ code: 'timeout', detail: '120' }, ''), {
  key: 'designFallbackError',
  args: ['120'],
});
assert.deepEqual(designFallbackReason({ code: 'generationFailed', detail: 'invalid' }, ''), {
  key: 'designFallbackError',
  args: ['invalid'],
});
assert.equal(workerActionResponseError({ ok: true }), null);
assert.equal(workerActionResponseError({ ok: false, busy: true }).code, 'actionBusy');
assert.equal(hasFreshWorkerResult([{ expiresAt: 101 }], 100), true);
assert.equal(hasFreshWorkerResult([{ expiresAt: 100 }, {}], 100), false);
assert.equal(designPollDelay(1000, 5000), 1000);
assert.equal(designPollDelay(5000, 750), 750);
assert.equal(designPollDelay(0, 5000), 100);

const splitEmoji = new ReadableStream({
  start(controller) {
    controller.enqueue(Uint8Array.from([0xf0, 0x9f]));
    controller.enqueue(Uint8Array.from([0x98, 0x80]));
    controller.close();
  },
});
assert.equal(
  await readBoundedDesignResponse(new Response(splitEmoji), 4),
  '😀',
  'streaming decode must preserve a split UTF-8 scalar',
);
await assert.rejects(
  readBoundedDesignResponse(new Response('123456'), 5),
  (error) => error && error.code === 'tooLarge',
);
await assert.rejects(
  readBoundedDesignResponse(new Response('x', { headers: { 'Content-Length': '6' } }), 5),
  (error) => error && error.code === 'tooLarge',
);

console.log('design-evidence: ok (schema, privacy boundary, cap, fallback states)');
