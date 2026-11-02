import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync, mkdtempSync, unlinkSync, rmdirSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';

test('release workflow generates a manifest from the final executable bytes', { skip: process.platform !== 'win32' }, () => {
  const workflow = readFileSync(new URL('../.github/workflows/release.yml', import.meta.url), 'utf8');
  const begin = workflow.indexOf('          $assets = @{}');
  const end = workflow.indexOf('          # 4. Generate SHA256 Checksums', begin);
  assert(begin >= 0 && end > begin);
  const actualStep = workflow.slice(begin, end).replace(/^ {10}/gm, '');
  const dir = mkdtempSync(join(tmpdir(), 'openrevo-manifest-test-'));
  const binary = join(dir, 'open-revo.exe'), installer = join(dir, 'setup.exe');
  const manifestPath = join(dir, 'latest.json');
  try {
    writeFileSync(binary, 'binary fixture'); writeFileSync(installer, 'installer fixture');
    const script = `$ErrorActionPreference = 'Stop'
$distDir = $env:OPENREVO_TEST_DIST
$destExe = Join-Path $distDir 'open-revo.exe'
$destInstaller = Join-Path $distDir 'setup.exe'
$ver = '1.2.3'
$tag = 'v1.2.3'
${actualStep}`;
    let psBin = 'pwsh';
    try { execFileSync('where.exe', ['pwsh'], { stdio: 'ignore' }); } catch { psBin = 'powershell'; }
    const runnerScript = psBin === 'powershell' ? script.replace('utf8NoBOM', 'utf8') : script;
    execFileSync(psBin, ['-NoProfile', '-NonInteractive', '-EncodedCommand', Buffer.from(runnerScript, 'utf16le').toString('base64')], {
      windowsHide: true, env: { ...process.env, OPENREVO_TEST_DIST: dir, GITHUB_REPOSITORY: 'faintonce/open-revo', GITHUB_SHA: 'test-commit' },
    });
    const manifest = JSON.parse(readFileSync(manifestPath, 'utf8').replace(/^\uFEFF/, ''));
    assert.equal(manifest.version, '1.2.3');
    assert.equal(manifest.commit, 'test-commit');
    for (const [key, file] of [['binary_only', binary], ['installer', installer]]) {
      const bytes = readFileSync(file), asset = manifest.assets[key];
      assert.equal(asset.size, bytes.length);
      assert.equal(asset.sha256, createHash('sha256').update(bytes).digest('hex'));
      assert.equal(asset.sources.github, `https://github.com/faintonce/open-revo/releases/download/v1.2.3/${asset.filename}`);
    }
  } finally {
    for (const file of [manifestPath, binary, installer]) { try { unlinkSync(file); } catch (error) { if (error.code !== 'ENOENT') throw error; } }
    rmdirSync(dir);
  }
});
