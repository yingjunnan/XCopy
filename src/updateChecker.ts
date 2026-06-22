const RELEASES_API_URL = "https://api.github.com/repos/yingjunnan/XCopy/releases?per_page=10";

export interface GitHubReleaseAsset {
  name: string;
  browser_download_url: string;
}

export interface GitHubRelease {
  tag_name: string;
  html_url: string;
  draft: boolean;
  prerelease: boolean;
  published_at?: string | null;
  assets?: GitHubReleaseAsset[];
}

export interface UpdateCheckResult {
  currentVersion: string;
  latestVersion?: string;
  latestTag?: string;
  releaseUrl?: string;
  downloadUrl?: string;
  publishedAt?: string | null;
  hasUpdate: boolean;
}

export function versionFromTag(_tag: string): string | undefined {
  const match = _tag.match(/\bv?(\d+(?:\.\d+){1,3})(?:\b|-)/i);
  return match?.[1];
}

export function isNewerVersion(latest: string, current: string): boolean {
  const latestParts = parseVersionParts(latest);
  const currentParts = parseVersionParts(current);
  if (!latestParts || !currentParts) return false;

  for (let index = 0; index < Math.max(latestParts.length, currentParts.length); index += 1) {
    const latestPart = latestParts[index] ?? 0;
    const currentPart = currentParts[index] ?? 0;
    if (latestPart > currentPart) return true;
    if (latestPart < currentPart) return false;
  }

  return false;
}

export function updateInfoFromReleases(
  currentVersion: string,
  releases: GitHubRelease[]
): UpdateCheckResult {
  const latest = releases
    .filter((release) => !release.draft && !release.prerelease)
    .map((release) => ({
      release,
      version: versionFromTag(release.tag_name),
    }))
    .filter(
      (item): item is { release: GitHubRelease; version: string } =>
        typeof item.version === "string"
    )
    .sort((left, right) => compareVersions(right.version, left.version))[0];

  if (latest) {
    const installer = latest.release.assets?.find((asset) => {
      const name = asset.name.toLowerCase();
      return name.startsWith("xcopy_") && name.endsWith("setup.exe");
    });

    return {
      currentVersion,
      latestVersion: latest.version,
      latestTag: latest.release.tag_name,
      releaseUrl: latest.release.html_url,
      downloadUrl: installer?.browser_download_url,
      publishedAt: latest.release.published_at,
      hasUpdate: isNewerVersion(latest.version, currentVersion),
    };
  }

  return {
    currentVersion,
    hasUpdate: false,
  };
}

function parseVersionParts(version: string): number[] | undefined {
  const parts = version.split(".");
  if (parts.length < 2) return undefined;

  const parsed = parts.map((part) => {
    if (!/^\d+$/.test(part)) return Number.NaN;
    return Number(part);
  });

  return parsed.every(Number.isFinite) ? parsed : undefined;
}

function compareVersions(left: string, right: string): number {
  const leftParts = parseVersionParts(left) ?? [];
  const rightParts = parseVersionParts(right) ?? [];
  const maxLength = Math.max(leftParts.length, rightParts.length);

  for (let index = 0; index < maxLength; index += 1) {
    const leftPart = leftParts[index] ?? 0;
    const rightPart = rightParts[index] ?? 0;
    if (leftPart !== rightPart) return leftPart - rightPart;
  }

  return 0;
}

export async function checkForUpdates(
  currentVersion: string,
  fetchImpl: typeof fetch = fetch
): Promise<UpdateCheckResult> {
  const response = await fetchImpl(RELEASES_API_URL, {
    headers: {
      Accept: "application/vnd.github+json",
    },
  });

  if (!response.ok) {
    throw new Error(`GitHub releases request failed: ${response.status}`);
  }

  const releases = (await response.json()) as GitHubRelease[];
  return updateInfoFromReleases(currentVersion, releases);
}
