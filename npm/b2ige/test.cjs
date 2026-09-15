const {test} = require('node:test');
const assert = require('node:assert/strict');
const {spawnSync, spawn} = require('node:child_process');
const {mkdtempSync, writeFileSync, rmSync, cpSync} = require('node:fs');
const {tmpdir} = require('node:os');
const path = require('node:path');
const {createHash} = require('node:crypto');
const {readFileSync, existsSync} = require('node:fs');
const versionProbe = 'if [ "$1" = "--version" ]; then echo "verify-cli 0.1.0"; exit 0; fi\n';
function trustedEnv(fixture) {
  return {...process.env,B2IGE_BINARY:fixture,B2IGE_BINARY_SHA256:createHash('sha256').update(readFileSync(fixture)).digest('hex')};
}
for (const name of ['b2ige','blindtest']) for (const code of [0,1,2,3,64]) test(`${name}: args/stdin/stdout/stderr/exit ${code}`, () => {
  const root = mkdtempSync(path.join(tmpdir(), 'wrapper-'));
  try {
    const fixture = path.join(root,'native');
    writeFileSync(fixture, `#!/bin/sh\n${versionProbe}printf '%s\\n' "$@"\ncat\nprintf stderr >&2\nexit ${code}\n`, {mode:0o700});
    const r = spawnSync(process.execPath,[path.join(__dirname,`bin/${name}.cjs`),'space arg','--literal'],{env:trustedEnv(fixture),input:'stdin',encoding:'utf8'});
    assert.equal(r.status,code); assert.equal(r.stderr,'stderr');
    assert.equal(r.stdout,`${name==='blindtest'?'blindtest\n':''}space arg\n--literal\nstdin`);
  } finally {rmSync(root,{recursive:true,force:true});}
});
test('missing native binary fails closed', () => {
  const r=spawnSync(process.execPath,[path.join(__dirname,'bin/b2ige.cjs')],{env:{...process.env,B2IGE_BINARY:'/nonexistent/b2ige'}}); assert.equal(r.status,3);
});
test('child signal is preserved', () => {
  const root=mkdtempSync(path.join(tmpdir(),'wrapper-signal-'));
  try {const f=path.join(root,'native'); writeFileSync(f,`#!/bin/sh\n${versionProbe}kill -TERM $$\n`,{mode:0o700});
    const r=spawnSync(process.execPath,[path.join(__dirname,'bin/b2ige.cjs')],{env:trustedEnv(f)}); assert.equal(r.signal,'SIGTERM');
  } finally {rmSync(root,{recursive:true,force:true});}
});
for (const mode of ['missing','mismatch','malformed','version']) test(`refuses ${mode} integrity/version before product execution`, () => {
  const root=mkdtempSync(path.join(tmpdir(),'wrapper-integrity-'));
  try {
    const f=path.join(root,'native'), marker=path.join(root,'executed');
    writeFileSync(f,`#!/bin/sh\n${mode==='version'?'':`touch '${marker}'\n`}if [ "$1" = "--version" ]; then echo 'verify-cli 9.9.9'; exit 0; fi\ntouch '${marker}'\n`,{mode:0o700});
    const env=trustedEnv(f);
    if (mode==='missing') delete env.B2IGE_BINARY_SHA256;
    if (mode==='mismatch') env.B2IGE_BINARY_SHA256='0'.repeat(64);
    if (mode==='malformed') env.B2IGE_BINARY_SHA256='not-a-checksum';
    const r=spawnSync(process.execPath,[path.join(__dirname,'bin/b2ige.cjs')],{env,encoding:'utf8'});
    assert.equal(r.status,3); assert.equal(existsSync(marker),false);
    assert.match(r.stderr,/refused/);
  } finally {rmSync(root,{recursive:true,force:true});}
});
test('bundled pin works and altered manifest version is refused', () => {
  const root=mkdtempSync(path.join(tmpdir(),'wrapper-bundled-'));
  try {
    cpSync(path.join(__dirname,'bin'),path.join(root,'bin'),{recursive:true});
    cpSync(path.join(__dirname,'package.json'),path.join(root,'package.json'));
    const platform=`${process.platform}-${process.arch}`;
    const dir=path.join(root,'native',platform);require('node:fs').mkdirSync(dir,{recursive:true});
    const f=path.join(dir,'b2ige');writeFileSync(f,`#!/bin/sh\n${versionProbe}echo ready\n`,{mode:0o700});
    const manifest={schema_version:'2',version:'0.1.0',release_repository:null,platforms:{[platform]:{sha256:trustedEnv(f).B2IGE_BINARY_SHA256}}};
    const env={...process.env};delete env.B2IGE_BINARY;delete env.B2IGE_BINARY_SHA256;
    writeFileSync(path.join(root,'native-manifest.json'),JSON.stringify(manifest));
    const run=()=>spawnSync(process.execPath,[path.join(root,'bin/b2ige.cjs')],{env,encoding:'utf8'});
    assert.equal(run().stdout,'ready\n');
    manifest.version='9.9.9';writeFileSync(path.join(root,'native-manifest.json'),JSON.stringify(manifest));
    assert.equal(run().status,3);
  } finally {rmSync(root,{recursive:true,force:true});}
});
test('parent SIGTERM is forwarded to the native child', {timeout:7000}, async () => {
  const root=mkdtempSync(path.join(tmpdir(),'wrapper-forward-'));let child;
  try {
    const f=path.join(root,'native');writeFileSync(f,`#!/bin/sh\n${versionProbe}trap 'exit 42' TERM\necho ready\nwhile :; do sleep 0.1; done\n`,{mode:0o700});
    child=spawn(process.execPath,[path.join(__dirname,'bin/b2ige.cjs')],{env:trustedEnv(f),stdio:['ignore','pipe','pipe']});
    const result=new Promise((resolve,reject)=>{child.once('exit',(code,signal)=>resolve({code,signal}));child.once('error',reject);});
    await new Promise((resolve,reject)=>{child.stdout.once('data',resolve);child.once('error',reject);child.once('exit',()=>reject(Error('Exited before ready')));});
    child.kill('SIGTERM');assert.deepEqual(await result,{code:42,signal:null});
  } finally {if(child && child.exitCode===null)child.kill('SIGKILL');rmSync(root,{recursive:true,force:true});}
});
test('relative override executes the checked file even when PATH has a namesake', () => {
  const root=mkdtempSync(path.join(tmpdir(),'wrapper-path-'));
  try {
    const f=path.join(root,'native'),shadow=path.join(root,'shadow');require('node:fs').mkdirSync(shadow);
    writeFileSync(f,`#!/bin/sh\n${versionProbe}echo checked\n`,{mode:0o700});
    writeFileSync(path.join(shadow,'native'),`#!/bin/sh\n${versionProbe}echo unchecked\n`,{mode:0o700});
    const env={...trustedEnv(f),B2IGE_BINARY:'native',PATH:`${shadow}${path.delimiter}${process.env.PATH}`};
    const r=spawnSync(process.execPath,[path.join(__dirname,'bin/b2ige.cjs')],{cwd:root,env,encoding:'utf8'});
    assert.equal(r.status,0);assert.equal(r.stdout,'checked\n');
  } finally {rmSync(root,{recursive:true,force:true});}
});
