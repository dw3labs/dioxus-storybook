// Minimal wasm section dumper — answers "how does inventory initialise on wasm?"
import fs from 'node:fs';
const b = fs.readFileSync(process.argv[2]);
const NAMES = {0:'custom',1:'type',2:'import',3:'function',4:'table',5:'memory',6:'global',
               7:'export',8:'START',9:'element',10:'code',11:'data',12:'data-count'};
let p = 8; // skip magic + version
const seen = [];
while (p < b.length) {
  const id = b[p++];
  let size = 0, shift = 0, byte;
  do { byte = b[p++]; size |= (byte & 0x7f) << shift; shift += 7; } while (byte & 0x80);
  let extra = '';
  if (id === 0) { // custom section name
    let n = 0, s2 = 0, by; const q0 = p;
    do { by = b[p++]; n |= (by & 0x7f) << s2; s2 += 7; } while (by & 0x80);
    extra = ` "${b.slice(p, p + n).toString()}"`; p = q0;
  }
  seen.push(`${String(id).padStart(2)} ${(NAMES[id]||'?').padEnd(11)} ${String(size).padStart(5)}B${extra}`);
  p += size;
}
console.log(seen.join('\n'));
console.log(seen.some(s => s.includes('START')) ? '\n=> has a START section (ctors run at instantiate)' : '\n=> NO start section');
