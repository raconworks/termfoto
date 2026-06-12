#!/usr/bin/env node
// termfoto npm install — downloads prebuilt binary from GitHub Releases

const https = require('https');
const fs = require('fs');
const path = require('path');

function getBinaryName() {
  return process.platform === 'win32' ? 'termfoto.exe' : 'termfoto';
}

function getDownloadUrl(version, binaryName) {
  return `https://github.com/PineWhisperStudio/termfoto/releases/download/v${version}/${binaryName}`;
}

function install() {
  const pkg = require('./package.json');
  const version = pkg.version;
  const binaryName = getBinaryName();
  const url = getDownloadUrl(version, binaryName);
  const dest = path.join(__dirname, binaryName);

  console.log(`termfoto: downloading v${version}...`);

  https.get(url, (res) => {
    if (res.statusCode === 302 || res.statusCode === 301) {
      https.get(res.headers.location, (redirectRes) => {
        downloadToFile(redirectRes, dest);
      }).on('error', (err) => {
        handleError(err, url);
      });
      return;
    }

    if (res.statusCode !== 200) {
      console.error(`termfoto: HTTP ${res.statusCode} — binary not found`);
      console.error(`termfoto: URL: ${url}`);
      console.error(`termfoto: install via cargo instead: cargo install termfoto`);
      process.exit(1);
    }

    downloadToFile(res, dest);
  }).on('error', (err) => {
    handleError(err, url);
  });
}

function downloadToFile(res, dest) {
  const file = fs.createWriteStream(dest, { mode: 0o755 });
  res.pipe(file);

  file.on('finish', () => {
    file.close();
    fs.chmodSync(dest, 0o755);
    console.log('termfoto: installed successfully');
  });

  file.on('error', (err) => {
    fs.unlink(dest, () => {});
    console.error(`termfoto: failed to write binary — ${err.message}`);
    process.exit(1);
  });
}

function handleError(err, url) {
  console.error(`termfoto: download failed — ${err.message}`);
  console.error(`termfoto: URL: ${url}`);
  console.error(`termfoto: install via cargo instead: cargo install termfoto`);
  process.exit(1);
}

install();
