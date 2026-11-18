import assert from 'node:assert/strict'
import { spawnSync } from 'node:child_process'
import { chmod, copyFile, mkdir, mkdtemp, readFile, realpath, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'
import { after, test } from 'node:test'

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)))
const testRoot = await mkdtemp(join(tmpdir(), 'dsh-openpencil-run-tool-'))
const canonicalTestRoot = await realpath(testRoot)

after(async () => {
  await rm(testRoot, { recursive: true, force: true })
})

await mkdir(join(testRoot, 'scripts'), { recursive: true })
await mkdir(join(testRoot, 'node_modules', '.bin'), { recursive: true })
await mkdir(join(testRoot, 'node_modules', '@types', 'fixture'), { recursive: true })
await mkdir(join(testRoot, 'node_modules', 'typescript', 'bin'), { recursive: true })
await mkdir(join(testRoot, 'node_modules', 'tsdown', 'dist'), { recursive: true })
await copyFile(join(projectRoot, 'scripts', 'run-tool.mjs'), join(testRoot, 'scripts', 'run-tool.mjs'))

for (const tool of ['tsc', 'tsdown']) {
  const shim = join(testRoot, 'node_modules', '.bin', tool)
  await writeFile(shim, 'this shim must never be executed\n')
  await chmod(shim, 0o644)
}

await writeFile(
  join(testRoot, 'node_modules', 'typescript', 'bin', 'tsc'),
  "require('node:fs').writeFileSync(process.env.RUN_TOOL_TEST_OUTPUT, JSON.stringify(process.argv.slice(2)))\n",
)
await writeFile(
  join(testRoot, 'node_modules', 'tsdown', 'dist', 'run.mjs'),
  "import { writeFileSync } from 'node:fs'\nwriteFileSync(process.env.RUN_TOOL_TEST_OUTPUT, JSON.stringify(process.argv.slice(2)))\n",
)

test('runs the real TypeScript JavaScript entrypoint and preserves arguments', async () => {
  const output = join(testRoot, 'tsc-args.json')
  runTool('tsc', ['--pretty', 'false'], output)
  assert.deepEqual(JSON.parse(await readFile(output, 'utf8')), [
    '--pretty',
    'false',
    '--typeRoots',
    join(canonicalTestRoot, 'node_modules', '@types'),
  ])
})

test('runs the real tsdown JavaScript entrypoint without a platform shim', async () => {
  const output = join(testRoot, 'tsdown-args.json')
  runTool('tsdown', ['src/index.ts', '--format', 'cjs'], output)
  assert.deepEqual(JSON.parse(await readFile(output, 'utf8')), [
    'src/index.ts',
    '--format',
    'cjs',
  ])
})

function runTool(tool, args, output) {
  const result = spawnSync(
    process.execPath,
    [join(testRoot, 'scripts', 'run-tool.mjs'), tool, ...args],
    {
      cwd: testRoot,
      encoding: 'utf8',
      env: {
        ...process.env,
        DSH_SOURCE_ROOT: '',
        RUN_TOOL_TEST_OUTPUT: output,
        // The script also scans the invoking user's ~/.dsh/source bundles; a
        // machine with an installed DSH would otherwise leak their @types into
        // the forwarded typeRoots. Point HOME at the hermetic test tree so the
        // assertion covers exactly the roots this test provisions.
        HOME: testRoot,
      },
    },
  )
  assert.equal(result.status, 0, result.stderr || result.stdout)
}
