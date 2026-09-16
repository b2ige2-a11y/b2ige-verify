const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const https = require('node:https');
const {Readable} = require('node:stream');
const {EventEmitter} = require('node:events');
const {spawn} = require('node:child_process');
const {gzipSync} = require('node:zlib');
const {releaseURL, fetchArchive, unpack, digest, targets} = require('./bin/download.cjs');
const platform = `${process.platform}-${process.arch}`;
const rootName = `b2ige-0.2.0-${targets[platform]}`;
function archive(entries) {
  const chunks = [];
  for (const {name, data = Buffer.alloc(0), type = '0'} of entries) {
    const header = Buffer.alloc(512);
    header.write(name); header.write('0000700\0',100); header.write('0000000\0',108); header.write('0000000\0',116);
    header.write(data.length.toString(8).padStart(11,'0')+'\0',124); header.write('00000000000\0',136);
    header.fill(32,148,156); header.write(type,156); header.write('ustar\0',257); header.write('00',263);
    const sum = header.reduce((a,b)=>a+b,0);
    header.write(sum.toString(8).padStart(6,'0')+'\0 ',148);
    chunks.push(header,data,Buffer.alloc((512-data.length%512)%512));
  }
  return gzipSync(Buffer.concat([...chunks,Buffer.alloc(1024)]));
}
function manifest(binary, tar) {
  return {schema_version:'2',version:'0.2.0',release_repository:'synthetic-owner/synthetic-repository',platforms:{[platform]:{sha256:digest(binary),archive_sha256:digest(tar)}}};
}
test('version/host/pin/platform configuration fails closed', () => {
  const m=manifest(Buffer.from('test'),Buffer.from('test'));
  assert.match(releaseURL(m,platform,'0.2.0'),new RegExp(`/v0.2.0/${rootName}\\.tar\\.gz$`));
  assert.throws(()=>releaseURL(m,'win32-x64','0.2.0'),/Unsupported/);
  assert.throws(()=>releaseURL({...m,version:'9.9.9'},platform,'0.2.0'),/version/);
  for (const release_repository of [null,'../escape','https://example.invalid/x','owner/repo/extra']) assert.throws(()=>releaseURL({...m,release_repository},platform,'0.2.0'));
  assert.throws(()=>releaseURL({...m,platforms:{}},platform,'0.2.0'),/pins/);
});
test('USTAR rejects escape, links, duplicate, corrupt and truncated archives', () => {
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'wrapper-tar-'));
  try {
    for (const name of ['../escape','/absolute',`${rootName}/../escape`,`${rootName}/a/./b`,`${rootName}/a\\b`]) assert.throws(()=>unpack(archive([{name}]),rootName,dir),/Unsafe/);
    for (const type of ['1','2','3','6','x']) assert.throws(()=>unpack(archive([{name:`${rootName}/link`,type}]),rootName,dir),/Unsafe/);
    assert.throws(()=>unpack(archive([{name:`${rootName}/x`},{name:`${rootName}/x`}]),rootName,dir),/duplicate/);
    assert.throws(()=>unpack(Buffer.from('broken'),rootName,dir));
    assert.throws(()=>unpack(archive([{name:`${rootName}/other`}]).subarray(0,20),rootName,dir));
  } finally {fs.rmSync(dir,{recursive:true,force:true});}
});
test('download errors, redirects and host policy', async t => {
  let responses=[];
  t.mock.method(https,'get',(_url,_options,callback)=>{
    const req=new EventEmitter();req.destroy=()=>{};
    queueMicrotask(()=>{
      const r=responses.shift();
      if (r instanceof Error) return req.emit('error',r);
      const response=Readable.from(r.body ? [Buffer.from(r.body)] : []);
      response.statusCode=r.status;response.headers=r.headers||{};callback(response);
    });
    return req;
  });
  responses=[{status:302,headers:{location:'https://release-assets.githubusercontent.com/synthetic'}},{status:200,body:'archive'}];
  assert.equal((await fetchArchive('https://github.com/synthetic')).toString(),'archive');
  responses=[{status:404}];await assert.rejects(fetchArchive('https://github.com/synthetic'),/HTTP 404/);
  responses=[new Error('offline')];await assert.rejects(fetchArchive('https://github.com/synthetic'),/offline/);
  for (const location of ['http://github.com/x','https://example.invalid/x','https://user@github.com/x']) {
    responses=[{status:302,headers:{location}}];await assert.rejects(fetchArchive('https://github.com/synthetic'),/host/);
  }
  responses=Array.from({length:4},()=>({status:302,headers:{location:'https://github.com/loop'}}));
  await assert.rejects(fetchArchive('https://github.com/synthetic'),/redirect limit/);
});
function run(args,env) {
  return new Promise((resolve,reject)=>{
    const child=spawn(process.execPath,args,{env});let stdout='',stderr='';
    child.stdout.on('data',d=>stdout+=d);child.stderr.on('data',d=>stderr+=d);
    child.on('error',reject);child.on('exit',status=>resolve({status,stdout,stderr}));
  });
}
for (const mode of ['success','archive-tamper','binary-tamper','version','unsafe','download-failure']) test(`download-to-launch flow: ${mode}`,async()=>{
  const dir=fs.mkdtempSync(path.join(os.tmpdir(),'wrapper-delivery-'));
  try {
    fs.cpSync(path.join(__dirname,'bin'),path.join(dir,'bin'),{recursive:true});
    fs.copyFileSync(path.join(__dirname,'package.json'),path.join(dir,'package.json'));
    const marker=path.join(dir,'executed');
    const binary=Buffer.from(`#!/bin/sh\nif [ "$1" = "--version" ]; then echo 'verify-cli ${mode==='version'?'9.9.9':'0.2.0'}'; exit 0; fi\ntouch '${marker}'\nprintf 'delivered:%s' "$1"\n`);
    const tar=archive([{name:mode==='unsafe'?'../escape':`${rootName}/bin/b2ige`,data:binary}]);
    const m=manifest(binary,tar);
    if (mode==='archive-tamper') m.platforms[platform].archive_sha256='0'.repeat(64);
    if (mode==='binary-tamper') m.platforms[platform].sha256='0'.repeat(64);
    fs.writeFileSync(path.join(dir,'native-manifest.json'),JSON.stringify(m));
    fs.writeFileSync(path.join(dir,'asset'),tar);
    // Process-local synthetic transport: production never accepts HTTP or a URL override.
    fs.writeFileSync(path.join(dir,'transport.cjs'),`const fs=require('node:fs'),{Readable}=require('node:stream'),{EventEmitter}=require('node:events');require('node:os').homedir=()=>__dirname;require('node:https').get=(url,opts,cb)=>{const req=new EventEmitter();req.destroy=()=>{};queueMicrotask(()=>{if(${mode==='download-failure'})return req.emit('error',Error('synthetic offline'));const r=Readable.from([fs.readFileSync(__dirname+'/asset')]);r.statusCode=200;r.headers={};cb(r)});return req;};`);
    const env={...process.env};delete env.B2IGE_BINARY;delete env.B2IGE_BINARY_SHA256;
    const args=['--require',path.join(dir,'transport.cjs'),path.join(dir,'bin/b2ige.cjs'),'hello'];
    const result=await run(args,env);
    if (mode==='success') {
      assert.equal(result.status,0,result.stderr);assert.equal(result.stdout,'delivered:hello');
      fs.unlinkSync(path.join(dir,'asset')); // Cached matching delivery works with transport unavailable.
      const offline=await run(args,env);assert.equal(offline.status,0,offline.stderr);
      const cache=path.join(dir,'.cache/b2ige-verify/0.2.0',platform,m.platforms[platform].archive_sha256,rootName,'bin/b2ige');
      fs.appendFileSync(cache,'# tampered');assert.equal((await run(args,env)).status,3);
    } else {
      assert.equal(result.status,3,result.stderr);assert.match(result.stderr,/ERROR/);assert.equal(fs.existsSync(marker),false);
    }
  } finally {fs.rmSync(dir,{recursive:true,force:true});}
});
