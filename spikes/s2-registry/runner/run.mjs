// Runs a wasm32-unknown-unknown cdylib and reports the story registry state.
// Reconstructs each story id byte-by-byte out of the module, which cannot be
// satisfied unless the registry genuinely exists in memory at runtime.
import fs from 'node:fs';

const file = process.argv[2];
const label = process.argv[3] ?? file;
const buf = fs.readFileSync(file);

let mod;
try { mod = await WebAssembly.compile(buf); }
catch (e) { console.log(`${label}: COMPILE FAILED — ${e.message}`); process.exit(1); }

const imports = {};
for (const imp of WebAssembly.Module.imports(mod)) {
  imports[imp.module] ??= {};
  if (imp.kind === 'function') imports[imp.module][imp.name] = () => 0;
  else if (imp.kind === 'memory') imports[imp.module][imp.name] = new WebAssembly.Memory({ initial: 17 });
  else if (imp.kind === 'global') imports[imp.module][imp.name] = new WebAssembly.Global({ value: 'i32', mutable: false }, 0);
}

const ex = (await WebAssembly.instantiate(mod, imports)).exports;
const hasCtors = typeof ex.__wasm_call_ctors === 'function';
const before = ex.story_count?.() ?? null;
if (hasCtors) ex.__wasm_call_ctors();

const count = ex.story_count?.() ?? null;
const sum   = ex.story_checksum?.() ?? null;
const lens  = ex.story_id_len_sum?.() ?? null;

const ids = [];
if (ex.story_id_byte && count != null) {
  for (let i = 0; i < count; i++) {
    let s = '';
    for (let j = 0; j < 64; j++) {
      const b = ex.story_id_byte(i, j);
      if (b === 0 || b === 0xffffffff) break;
      s += String.fromCharCode(b);
    }
    ids.push(s);
  }
}

const EXPECT = ['core--local', 'forms-button--primary', 'forms-button--loading', 'app--local'];
const idsOk = EXPECT.every(e => ids.includes(e)) && ids.length === 4;

console.log(`--- ${label} ---`);
console.log(`  size:               ${buf.length} bytes`);
console.log(`  __wasm_call_ctors:  ${hasCtors ? 'present' : 'absent'}`);
console.log(`  count before ctors: ${before}`);
console.log(`  count:              ${count}  (expect 4)`);
console.log(`  checksum:           ${sum}  (expect 131)`);
console.log(`  id_len_sum:         ${lens}  (expect 63)`);
console.log(`  ids recovered:      ${ids.length ? ids.join(', ') : '(none)'}`);
const ok = count === 4 && sum === 131 && lens === 63 && idsOk;
console.log(`  VERDICT:            ${ok ? 'PASS' : 'FAIL'}`);
process.exit(ok ? 0 : 1);
