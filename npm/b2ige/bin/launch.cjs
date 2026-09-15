const path = require('node:path');
const fs = require('node:fs');
const {createHash} = require('node:crypto');
const {spawn, spawnSync} = require('node:child_process');
const {version} = require('../package.json');
module.exports = async function(prefix) {
  const platform = `${process.platform}-${process.arch}`;
  if (!['darwin-arm64','darwin-x64','linux-x64'].includes(platform)) {
    console.error(`ERROR: Unsupported platform: ${platform}`); process.exitCode = 3; return;
  }
  let binary = path.resolve(process.env.B2IGE_BINARY || path.join(__dirname, '..', 'native', platform, 'b2ige'));
  try {
    const manifest = require('../native-manifest.json');
    if (manifest.schema_version !== '2' || manifest.version !== version) throw Error('Native manifest version mismatch.');
    if (!process.env.B2IGE_BINARY && !fs.existsSync(binary)) binary = await require('./download.cjs').install(manifest, platform, version);
    const expected = process.env.B2IGE_BINARY ? process.env.B2IGE_BINARY_SHA256 : manifest.platforms[platform]?.sha256;
    if (!expected || !/^[a-f0-9]{64}$/.test(expected)) throw Error('A trusted native SHA-256 is required.');
    if (!fs.statSync(binary).isFile()) throw Error('Native executable must be a regular file.');
    const actual = createHash('sha256').update(fs.readFileSync(binary)).digest('hex');
    if (actual !== expected) throw Error('Native checksum mismatch; executable was not launched.');
    const probe = spawnSync(binary, ['--version'], {encoding:'utf8', timeout:5000, maxBuffer:4096});
    if (probe.error || probe.status !== 0 || probe.stdout.trim() !== `verify-cli ${version}`) throw Error('Native executable version mismatch or unavailable.');
  } catch (error) {
    console.error(`ERROR: Native launcher refused: ${error.message} Use a matching native archive with a trusted SHA-256 for manual/offline installation.`);
    process.exitCode = 3; return;
  }
  const child = spawn(binary, [...prefix, ...process.argv.slice(2)], {stdio:'inherit'});
  const signals = ['SIGINT','SIGTERM','SIGHUP'];
  const handlers = signals.map(signal => { const handler = () => child.kill(signal); process.on(signal, handler); return handler; });
  child.on('error', () => {
    signals.forEach((s,i) => process.removeListener(s, handlers[i]));
    console.error('ERROR: Native binary unavailable. Install a verified native archive.'); process.exitCode = 3;
  });
  child.on('exit', (code, signal) => {
    signals.forEach((s,i) => process.removeListener(s, handlers[i]));
    if (signal) process.kill(process.pid, signal); else process.exitCode = code;
  });
};
