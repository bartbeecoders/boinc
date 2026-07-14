// Latest-release lookup and OS detection. Everything degrades: without the
// GitHub API (rate limit, no network) every download link falls back to the
// releases page.

import { useEffect, useState } from "react";

export const REPO = "bartbeecoders/boinc";
export const RELEASES_URL = `https://github.com/${REPO}/releases/latest`;

export function detectOs() {
  const ua = navigator.userAgent;
  if (/Windows/i.test(ua)) return "windows";
  if (/Mac OS X|Macintosh/i.test(ua)) return "mac";
  if (/Linux|X11/i.test(ua)) return "linux";
  return null;
}

export const OS_LABEL = { linux: "Linux", windows: "Windows", mac: "macOS" };

/** Extension of the primary artifact per OS. */
export const PRIMARY_ASSET = { linux: ".deb", windows: ".msi", mac: ".dmg" };

/**
 * Resolve the latest release: `{ assets: { ".deb": url, … }, tag }`.
 * Starts empty and stays empty when the API is unreachable.
 */
export function useLatestRelease() {
  const [release, setRelease] = useState({ assets: {}, tag: null });

  useEffect(() => {
    let cancelled = false;
    fetch(`https://api.github.com/repos/${REPO}/releases/latest`)
      .then((res) => (res.ok ? res.json() : null))
      .then((data) => {
        if (cancelled || !data || !data.assets) return;
        // Keys: ".deb", ".rpm" (x86_64), ".deb-aarch64", ".rpm-aarch64"
        // (arm64 — e.g. Fedora Asahi Remix), ".msi", ".dmg".
        const assets = {};
        for (const asset of data.assets) {
          const m = asset.name.match(/\.(deb|rpm|msi|dmg)$/);
          if (!m) continue;
          const arm = /(_arm64|aarch64)/.test(asset.name);
          assets[arm ? `.${m[1]}-aarch64` : `.${m[1]}`] = asset.browser_download_url;
        }
        setRelease({ assets, tag: data.tag_name ?? null });
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  return release;
}
