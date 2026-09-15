// Distribution only. No product verification or verdict logic belongs here.
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const https = require('node:https');
const {gunzipSync} = require('node:zlib');
const {createHash} = require('node:crypto');
const targets = {'darwin-arm64':'aarch64-apple-darwin', 'darwin-x64':'x86_64-apple-darwin', 'linux-x64':'x86_64-unknown-linux-gnu'};
const digest = data => createHash('sha256').update(data).digest('hex');
const isHash = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);

function releaseURL(manifest, platform, version) {
  if (!targets[platform]) throw Error(`Unsupported platform: ${platform}`);
  if (manifest.schema_version !== '2' || manifest.version !== version) throw Error('Native manifest version mismatch.');
  const repo = manifest.release_repository;
  if (typeof repo !== 'string' || !/^[A-Za-z0-9][A-Za-z0-9-]*\/[A-Za-z0-9][A-Za-z0-9_.-]*$/.test(repo)) throw Error('No approved GitHub release repository configured.');
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw Error('Invalid release version.');
  const pin = manifest.platforms[platform];
  if (!pin || !isHash(pin.sha256) || !isHash(pin.archive_sha256)) throw Error('Trusted archive and binary SHA-256 pins are required.');
  return `https://github.com/${repo}/releases/download/v${version}/b2ige-${version}-${targets[platform]}.tar.gz`;
}

function fetchArchive(url) {
  return new Promise((resolve, reject) => {
    let active, done = false;
    const finish = (error, data) => {
      if (done) return;
      done = true; clearTimeout(timer);
      if (error) { active?.destroy(); reject(error); } else resolve(data);
    };
    const timer = setTimeout(() => finish(Error('Download timed out.')), 60000);
    function request(address, redirects) {
      try {
        const parsed = new URL(address);
        if (parsed.protocol !== 'https:' || parsed.username || parsed.password || parsed.port ||
            !['github.com','release-assets.githubusercontent.com','objects.githubusercontent.com'].includes(parsed.hostname)) throw Error('Unapproved download/redirect host.');
        active = https.get(parsed, {headers:{'User-Agent':'b2ige-verify-launcher','Accept-Encoding':'identity'}}, response => {
          if ([301,302,303,307,308].includes(response.statusCode)) {
            response.destroy();
            if (redirects >= 3 || !response.headers.location) return finish(Error('Download redirect limit or missing location.'));
            try { request(new URL(response.headers.location, parsed).href, redirects + 1); } catch (error) { finish(error); }
            return;
          }
          if (response.statusCode !== 200) { response.destroy(); return finish(Error(`Download HTTP ${response.statusCode}.`)); }
          response.on('error', error => finish(error));
          response.on('aborted', () => finish(Error('Download aborted.')));
          const chunks = []; let size = 0;
          response.on('data', data => {
            size += data.length;
            if (size > 64 * 1024 * 1024) return finish(Error('Download exceeds 64 MiB limit.'));
            chunks.push(data);
          });
          response.on('end', () => finish(null, Buffer.concat(chunks)));
        });
        active.on('error', error => finish(error));
      } catch (error) { finish(error); }
    }
    request(url, 0);
  });
}

// Accept the packager's bounded USTAR format only; never invoke shell tar.
function unpack(archive, rootName, destination) {
  const tar = gunzipSync(archive, {maxOutputLength:512 * 1024 * 1024});
  const seen = new Set(); let end = false;
  const string = bytes => bytes.toString('utf8').replace(/\0.*$/s, '');
  const octal = bytes => {
    const value = string(bytes).trim();
    if (!/^[0-7]+$/.test(value)) throw Error('Invalid tar numeric field.');
    return parseInt(value, 8);
  };
  for (let offset = 0; offset + 512 <= tar.length;) {
    const header = tar.subarray(offset, offset + 512); offset += 512;
    if (header.every(byte => byte === 0)) {
      if (tar.length - offset < 512 || tar.subarray(offset).some(byte => byte !== 0)) throw Error('Invalid tar terminator.');
      end = true; break;
    }
    let sum = 0;
    for (let i = 0; i < 512; i++) sum += i >= 148 && i < 156 ? 32 : header[i];
    if (sum !== octal(header.subarray(148,156)) || string(header.subarray(257,263)) !== 'ustar') throw Error('Invalid USTAR header.');
    const prefix = string(header.subarray(345,500));
    const name = (prefix ? prefix + '/' : '') + string(header.subarray(0,100));
    const normalized = name.replace(/\/$/, '');
    const parts = normalized.split('/');
    const kind = header[156];
    if (!normalized || parts[0] !== rootName || parts.some(p => !p || p === '.' || p === '..') ||
        /[\\\x00-\x1f\x7f]/.test(name) || seen.has(normalized) || ![0,48,53].includes(kind) ||
        string(header.subarray(157,257))) throw Error('Unsafe, duplicate or unsupported archive member.');
    seen.add(normalized);
    const size = octal(header.subarray(124,136));
    if (offset + Math.ceil(size / 512) * 512 > tar.length || (kind === 53 && size !== 0)) throw Error('Truncated or invalid archive member.');
    const output = path.join(destination, ...parts);
    if (kind === 53) fs.mkdirSync(output, {recursive:true, mode:0o700});
    else {
      fs.mkdirSync(path.dirname(output), {recursive:true, mode:0o700});
      fs.writeFileSync(output, tar.subarray(offset, offset + size), {flag:'wx', mode:octal(header.subarray(100,108)) & 0o111 ? 0o700 : 0o600});
    }
    offset += Math.ceil(size / 512) * 512;
  }
  if (!end) throw Error('Missing tar terminator.');
}

async function install(manifest, platform, version) {
  const url = releaseURL(manifest, platform, version);
  const pin = manifest.platforms[platform];
  const rootName = `b2ige-${version}-${targets[platform]}`;
  const parent = path.join(os.homedir(), '.cache', 'b2ige-verify', version, platform);
  const destination = path.join(parent, pin.archive_sha256);
  const binary = path.join(destination, rootName, 'bin', 'b2ige');
  if (fs.existsSync(destination)) return binary; // Caller rechecks digest and version, including offline use.
  const archive = await fetchArchive(url);
  if (digest(archive) !== pin.archive_sha256) throw Error('Archive checksum mismatch; nothing extracted or executed.');
  fs.mkdirSync(parent, {recursive:true, mode:0o700});
  const stage = fs.mkdtempSync(path.join(parent, '.partial-'));
  try {
    unpack(archive, rootName, stage);
    const candidate = path.join(stage, rootName, 'bin', 'b2ige');
    if (!fs.lstatSync(candidate).isFile() || digest(fs.readFileSync(candidate)) !== pin.sha256) throw Error('Native checksum mismatch.');
    try { fs.renameSync(stage, destination); } catch (error) {
      if (!['EEXIST','ENOTEMPTY'].includes(error.code)) throw error;
    }
    return binary;
  } finally { fs.rmSync(stage, {recursive:true, force:true}); }
}
module.exports = {install, releaseURL, fetchArchive, unpack, digest, targets};
