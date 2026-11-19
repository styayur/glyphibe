// SPDX-License-Identifier: MIT
// Original maintenance helper; upstream picomatch remains MIT and unchanged in identity.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync, mkdirSync, existsSync, readdirSync, mkdtempSync, cpSync, rmSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { tmpdir } from 'node:os';
import { spawnSync, execFileSync } from 'node:child_process';

const root = process.cwd();
const binding = JSON.parse(readFileSync(join(root, 'security/picomatch.json')));
const hash = data => createHash('sha256').update(data).digest('hex');
const marker = '  const branches = splitTopLevel(body).map(branch => branch.trim());\n';
const guard = `
  // Locally backported GHSA-fc5q-39xc-vjpf mitigation: repeated wildcard
  // bodies are literalized, retaining upstream's existing risky-pattern policy.
  const containsWildcard = (value, nested = false) => {
    let cursor = 0;
    while (cursor < value.length) {
      const token = value[cursor++];
      if (token === '\\\\') { cursor++; continue; }
      if (token === '"') {
        while (cursor < value.length) {
          const quoted = value[cursor++];
          if (quoted === '\\\\') cursor++;
          else if (quoted === '"') break;
        }
      } else if (token === '[') {
        while (cursor < value.length) {
          const member = value[cursor++];
          if (member === '\\\\') cursor++;
          else if (member === ']') break;
        }
      } else if (nested ? token === '(' || '*+?@!'.includes(token) && value[cursor] === '(' : token === '*' && value[cursor] !== '(') return true;
    }
    return false;
  };
  if (branches.some(branch => containsWildcard(branch))) return { risky: true };
  // Nested wrappers can hide nullable or ambiguous repetition from the upstream
  // recursion counter. Preserve its proven single-character flattening case;
  // literalize every other non-plain nested extglob inside an outer repeat.
  if (branches.some(branch => containsWildcard(branch, true) && normalizeSimpleBranch(branch) === undefined)) {
    const alphabet = branches.map(branch => {
      const sequence = getStarExtglobSequenceChars(branch);
      if (sequence) return sequence;
      const literal = normalizeSimpleBranch(branch);
      return literal && literal.length === 1 ? [literal] : undefined;
    });
    if (alphabet.every(chars => chars !== undefined))
      return { risky: true, safeOutput: buildCharClassStar([...new Set(alphabet.flat())]) };
    return { risky: true };
  }
`;

function locations() {
  const lock = JSON.parse(readFileSync(join(root, 'package-lock.json')));
  const paths = Object.entries(lock.packages).filter(([path]) => path.endsWith('/picomatch'));
  assert.ok(paths.length, 'locked picomatch inventory is required');
  for (const [path, pkg] of paths) {
    assert.equal(pkg.version, '4.0.7', `unreviewed version at ${path}`);
    assert.equal(pkg.dev, true, `runtime dependency at ${path} requires review`);
    assert.equal(pkg.integrity, binding.integrity, `tarball integrity at ${path}`);
  }
  // Do not let an extra installed matcher borrow a lock-bound verification.
  const installed = [];
  function walk(modules) {
    if (!existsSync(modules)) return;
    for (const entry of readdirSync(modules, { withFileTypes: true })) {
      if (!entry.isDirectory() || entry.name.startsWith('.')) continue;
      const path = join(modules, entry.name);
      if (entry.name.startsWith('@')) { walk(path); continue; }
      if (entry.name === 'picomatch') installed.push(path.replaceAll('\\', '/'));
      walk(join(path, 'node_modules'));
    }
  }
  walk(join(root, 'node_modules'));
  assert.deepEqual(installed.sort(), paths.map(([path]) => resolve(root, path).replaceAll('\\', '/')).sort());
  return paths.map(([path]) => resolve(root, path));
}

export function verify(directory, apply = false) {
  for (const [file, expected] of Object.entries(binding.upstreamSha256)) {
    const path = join(directory, file);
    const bytes = readFileSync(path);
    if (file === 'lib/parse.js') {
      if (apply && hash(bytes) === expected) {
        const source = bytes.toString('utf8');
        assert.equal(source.split(marker).length, 2, 'unique upstream mitigation point');
        writeFileSync(path, source.replace(marker, marker + guard));
      }
      assert.equal(hash(readFileSync(path)), binding.patchedParserSha256, 'unpatched/changed parser');
    } else assert.equal(hash(bytes), expected, `unreviewed installed payload: ${file}`);
  }
  assert.deepEqual(readdirSync(join(directory, 'lib')).sort(), Object.keys(binding.upstreamSha256).filter(path => path.startsWith('lib/')).map(path => path.slice(4)).sort());
  execFileSync(process.execPath, ['--input-type=commonjs', '-e', `
    const assert = require('node:assert/strict'); const pm = require(process.argv[1]);
    for (const pattern of ['+(*)','+(*a)','*(*a)','*(a*)','+(*?)','+(*a|b)','+(*(*a))','+(@(*(a)))','+(@(+(a)))','+(@(?(a)))','+(@(a|aa))','*(a@(*(a)))','+((a|aa))','+((a)+)']) {
      assert.equal(pm.isMatch('a'.repeat(96)+'!', pattern), false, pattern);
      assert.equal(pm.isMatch(pattern, pattern), true, 'literalized '+pattern);
    }
    for (const [input,pattern,result] of [
      ['src/test.ts','**/*.ts',true], ['src/test.js','**/*.ts',false],
      ['abcabc','+(abc)',true], ['foobar','*(foo|bar)',true],
      ['abab','+([ab])',true], ['aab','+(*(a)|*(b))',true],
      ['test.ts','*.{ts,tsx}',true], ['.hidden','*',false], ['.hidden','*',true]
    ].slice(0,-1)) assert.equal(pm.isMatch(input,pattern),result,pattern);
    assert.equal(pm.isMatch('.hidden','*',{dot:true}),true);
    const wrapped = ['*(a)','+(a)','?(a)','@(a|aa)','*a'];
    for (let depth=0; depth<4; depth++) {
      for (const inner of [...wrapped]) for (const wrapper of ['@','+','*','?','!']) {
        const pattern = '+('+wrapper+'('+inner+'))';
        assert.equal(pm.isMatch('a'.repeat(96)+'!', pattern), false, pattern);
      }
      wrapped.splice(0, wrapped.length, ...wrapped.map(value=>'@('+value+')'));
    }
    assert.equal(pm.makeRe('**/*.ts').test('src/test.ts'),true);
    assert.equal(pm.scan('src/**/*.ts').base,'src');
    assert.ok(pm.parse('*.ts').output);
  `, directory], { timeout: 3000, stdio: 'pipe' });
}

function selfTest(directory) {
  const temporary = mkdtempSync(join(tmpdir(), 'picomatch-regression-'));
  try {
    cpSync(directory, temporary, { recursive: true });
    const parser = join(temporary, 'lib/parse.js');
    const patched = readFileSync(parser, 'utf8');
    assert.ok(patched.includes(guard));
    writeFileSync(parser, patched.replace(guard, ''));
    assert.throws(() => verify(temporary), /unpatched\/changed parser/);
    assert.throws(() => execFileSync(process.execPath, ['-e', "require(process.argv[1]).isMatch('a'.repeat(64)+'!', '+(*a)')", temporary], { timeout: 1500, stdio: 'pipe' }), error => error.code === 'ETIMEDOUT');
    verify(temporary, true);
    writeFileSync(parser, 'tampered');
    assert.throws(() => verify(temporary), /unpatched\/changed parser/);
  } finally { rmSync(temporary, { recursive: true, force: true }); }
}

function validateAudit(report, lock) {
  assert.ok(report.auditReportVersion === 2 && report.metadata?.vulnerabilities && !report.error, 'complete npm v2 audit required');
  const entries = report.vulnerabilities;
  assert.equal(Object.keys(entries).length, report.metadata.vulnerabilities.total, 'complete advisory inventory');
  function covered(name, visited = new Set()) {
    if (visited.has(name)) return false;
    const item = entries[name];
    assert.ok(item?.via?.length && item.nodes?.length, 'unresolved advisory dependency');
    for (const node of item.nodes) assert.equal(lock.packages[node]?.dev, true, 'runtime advisory requires separate review');
    let remediated = false;
    for (const via of item.via) {
      if (typeof via === 'string') remediated = covered(via, new Set([...visited, name])) || remediated;
      else {
        assert.ok(name === 'picomatch' && via.name === 'picomatch' && via.url === 'https://github.com/advisories/GHSA-fc5q-39xc-vjpf', 'new/unremediated advisory '+JSON.stringify(via));
        remediated = true;
      }
    }
    return remediated;
  }
  for (const name of Object.keys(entries)) assert.ok(covered(name), 'advisory has no verified remediation');
}

function audit() {
  mkdirSync('artifacts', { recursive: true });
  const result = spawnSync(process.platform === 'win32' ? 'npm.cmd' : 'npm', ['audit', '--json', '--fetch-retries=1', '--fetch-timeout=30000'], { cwd: root, encoding: 'utf8', shell: process.platform === 'win32', timeout: 120000 });
  writeFileSync('artifacts/npm-audit.json', result.stdout ?? '');
  assert.ok(!result.error && [0,1].includes(result.status), 'audit unavailable; raw output retained');
  const report = JSON.parse(result.stdout);
  validateAudit(report, JSON.parse(readFileSync('package-lock.json')));
  console.log(JSON.stringify({ status: 'VERIFIED_LOCAL_BACKPORT', rawAudit: 'artifacts/npm-audit.json', registryVulnerabilities: report.metadata.vulnerabilities, advisory: binding.advisory, installedCopies: locations().length }));
}

const mode = process.argv[2] ?? 'verify';
assert.ok(['apply','verify','test','audit'].includes(mode));
const copies = locations();
for (const directory of copies) verify(directory, mode === 'apply');
if (mode === 'test') {
  selfTest(copies[0]);
  const example = {auditReportVersion:2,metadata:{vulnerabilities:{total:1}},vulnerabilities:{picomatch:{nodes:['node_modules/picomatch'],via:[{name:'picomatch',url:'https://github.com/advisories/GHSA-fc5q-39xc-vjpf'}]}}};
  const lock = {packages:{'node_modules/picomatch':{dev:true}}};
  validateAudit(example, lock);
  const changed = structuredClone(example);
  changed.vulnerabilities.picomatch.via[0].url += '-unknown';
  assert.throws(() => validateAudit(changed, lock), /new\/unremediated/);
  assert.throws(() => validateAudit(example, {packages:{'node_modules/picomatch':{dev:false}}}), /runtime advisory/);
  assert.throws(() => validateAudit({...example,error:'audit unavailable'}, lock), /complete npm/);
}
if (mode === 'audit') audit();
console.log(`picomatch ${mode}: ${copies.length} integrity-bound installed copies passed exploit and compatibility checks`);
