import { createWriteStream, existsSync } from 'node:fs';
import { chmod, mkdir, readFile, rename, unlink } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { pipeline } from 'node:stream/promises';
import { detectTarget, downloadUrl } from './platform.js';

interface PackageManifest {
  version: string;
  rlwy?: {
    binaryVersion?: string;
    releaseBaseUrl?: string;
  };
}

async function main(): Promise<void> {
  if (process.env.RLWY_SKIP_INSTALL === '1') {
    console.log('rlwy: RLWY_SKIP_INSTALL=1 set, skipping binary download');
    return;
  }

  const here = dirname(fileURLToPath(import.meta.url));
  const pkgRoot = resolve(here, '..');
  const binDir = join(pkgRoot, 'bin');
  const manifest = JSON.parse(
    await readFile(join(pkgRoot, 'package.json'), 'utf8'),
  ) as PackageManifest;

  const version = manifest.rlwy?.binaryVersion ?? manifest.version;
  const releaseBaseUrl =
    manifest.rlwy?.releaseBaseUrl ??
    'https://github.com/rlwy-dev/rlwy/releases/download';

  let target;
  try {
    target = detectTarget();
  } catch (err) {
    console.error(`rlwy: ${(err as Error).message}`);
    console.error('rlwy: leaving install incomplete; the CLI will not run.');
    return;
  }

  const destName = process.platform === 'win32' ? 'rlwy.exe' : 'rlwy';
  const dest = join(binDir, destName);

  if (existsSync(dest) && process.env.RLWY_FORCE_INSTALL !== '1') {
    console.log(`rlwy: binary already present at ${dest}`);
    return;
  }

  const url = downloadUrl(releaseBaseUrl, version, target);
  console.log(`rlwy: downloading ${url}`);

  await mkdir(binDir, { recursive: true });
  const tmp = `${dest}.download`;

  try {
    const res = await fetch(url, {
      redirect: 'follow',
      headers: { 'user-agent': `rlwy-npm/${version}` },
    });
    if (!res.ok || !res.body) {
      throw new Error(`HTTP ${res.status} ${res.statusText}`);
    }
    await pipeline(
      res.body as unknown as NodeJS.ReadableStream,
      createWriteStream(tmp),
    );
    await rename(tmp, dest);
    if (process.platform !== 'win32') {
      await chmod(dest, 0o755);
    }
    console.log(`rlwy: installed binary → ${dest}`);
  } catch (err) {
    console.error(`rlwy: failed to install binary: ${(err as Error).message}`);
    console.error(`rlwy: tried ${url}`);
    try {
      await unlink(tmp);
    } catch {
      /* ignore */
    }
    // Don't fail the npm install entirely — user can retry.
    process.exitCode = 0;
  }
}

main().catch((err) => {
  console.error(`rlwy: install script crashed: ${(err as Error).stack ?? err}`);
  process.exitCode = 0;
});
