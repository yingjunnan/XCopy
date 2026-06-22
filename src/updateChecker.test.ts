import test from "node:test";
import assert from "node:assert/strict";
import {
  checkForUpdates,
  isNewerVersion,
  updateInfoFromReleases,
  versionFromTag,
  type GitHubRelease,
} from "./updateChecker.ts";

function release(
  tagName: string,
  publishedAt: string,
  options: Partial<GitHubRelease> = {}
): GitHubRelease {
  return {
    tag_name: tagName,
    html_url: `https://github.com/yingjunnan/XCopy/releases/tag/${tagName}`,
    draft: false,
    prerelease: false,
    published_at: publishedAt,
    assets: [
      {
        name: `XCopy_${tagName.replace(/^v/, "")}_x64-setup.exe`,
        browser_download_url: `https://example.com/${tagName}.exe`,
      },
    ],
    ...options,
  };
}

test("extracts semantic version from current and legacy release tags", () => {
  assert.equal(versionFromTag("v0.2.1"), "0.2.1");
  assert.equal(versionFromTag("v0.2.1-r42"), "0.2.1");
  assert.equal(versionFromTag("XCopy v1.10.0"), "1.10.0");
  assert.equal(versionFromTag("nightly-build"), undefined);
});

test("compares versions without treating build suffixes as app versions", () => {
  assert.equal(isNewerVersion("0.2.1", "0.2.0"), true);
  assert.equal(isNewerVersion("1.0.0", "0.9.9"), true);
  assert.equal(isNewerVersion("0.2.0", "0.2.0"), false);
  assert.equal(isNewerVersion("0.2.0", "0.2.1"), false);
});

test("picks the highest stable release and installer asset", () => {
  const info = updateInfoFromReleases("0.2.0", [
    release("v0.2.1-r3", "2026-06-20T08:00:00Z", { prerelease: true }),
    release("v0.2.2", "2026-06-21T08:00:00Z", { draft: true }),
    release("v0.2.1", "2026-06-19T08:00:00Z"),
  ]);

  assert.equal(info.hasUpdate, true);
  assert.equal(info.latestVersion, "0.2.1");
  assert.equal(info.latestTag, "v0.2.1");
  assert.equal(info.downloadUrl, "https://example.com/v0.2.1.exe");
});

test("throws on network failure so the caller can decide to stay silent", async () => {
  await assert.rejects(
    () =>
      checkForUpdates("0.2.0", async () => {
        throw new Error("offline");
      }),
    /offline/
  );
});
