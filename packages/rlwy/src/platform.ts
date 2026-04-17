export interface Target {
  triple: string;
  exeSuffix: string;
}

export function detectTarget(
  platform: NodeJS.Platform = process.platform,
  arch: string = process.arch,
): Target {
  const key = `${platform}-${arch}`;
  switch (key) {
    case 'linux-x64':
      return { triple: 'x86_64-unknown-linux-gnu', exeSuffix: '' };
    case 'linux-arm64':
      return { triple: 'aarch64-unknown-linux-gnu', exeSuffix: '' };
    case 'darwin-x64':
      return { triple: 'x86_64-apple-darwin', exeSuffix: '' };
    case 'darwin-arm64':
      return { triple: 'aarch64-apple-darwin', exeSuffix: '' };
    case 'win32-x64':
      return { triple: 'x86_64-pc-windows-msvc', exeSuffix: '.exe' };
    default:
      throw new Error(
        `Unsupported platform: ${platform}/${arch}. ` +
          `Supported: linux-x64, linux-arm64, darwin-x64, darwin-arm64, win32-x64.`,
      );
  }
}

export function assetName(version: string, target: Target): string {
  return `rlwy-v${version}-${target.triple}${target.exeSuffix}`;
}

export function downloadUrl(
  releaseBaseUrl: string,
  version: string,
  target: Target,
): string {
  return `${releaseBaseUrl.replace(/\/+$/, '')}/v${version}/${assetName(version, target)}`;
}
