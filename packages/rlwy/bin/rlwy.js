#!/usr/bin/env node
import { spawn } from 'node:child_process';
import { existsSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const binaryName = process.platform === 'win32' ? 'rlwy.exe' : 'rlwy';
const binaryPath = join(here, binaryName);

if (!existsSync(binaryPath)) {
  console.error(`rlwy: binary not found at ${binaryPath}`);
  console.error('The postinstall step may have failed. Try: npm install -g rlwy --force');
  process.exit(1);
}

const child = spawn(binaryPath, process.argv.slice(2), { stdio: 'inherit' });

child.on('exit', (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
  } else {
    process.exit(code ?? 0);
  }
});

child.on('error', (err) => {
  console.error(`rlwy: failed to launch binary at ${binaryPath}`);
  console.error(err.message);
  process.exit(1);
});
