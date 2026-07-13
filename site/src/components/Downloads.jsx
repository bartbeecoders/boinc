import { RELEASES_URL } from "../lib/release.js";

const CARDS = [
  { os: "linux", title: "Debian / Ubuntu", ext: ".deb", note: "sudo apt install ./boinc_*.deb" },
  { os: "linux", title: "Fedora / openSUSE", ext: ".rpm", note: "sudo dnf install ./boinc-*.rpm" },
  { os: "windows", title: "Windows", ext: ".msi", note: "Run the installer — no admin needed." },
  { os: "mac", title: "macOS", ext: ".dmg", note: "Unsigned for now: right-click → Open once." },
];

export default function Downloads({ os, release }) {
  return (
    <section className="section" id="downloads">
      <h2><span className="disc small" aria-hidden="true" />Downloads</h2>
      <p className="aside">
        {release.tag
          ? `Latest release: ${release.tag} — straight from GitHub.`
          : "Latest release, straight from GitHub."}
      </p>
      <div className="dl-grid">
        {CARDS.map((card) => (
          <div key={card.title} className={`dl-card${os === card.os ? " detected" : ""}`}>
            <h3>{card.title}</h3>
            <a className="btn-secondary" href={release.assets[card.ext] ?? RELEASES_URL}>
              Download {card.ext}
            </a>
            <code>{card.note}</code>
          </div>
        ))}
      </div>
      <p className="aside">
        After installing, open Boinc once — it sets up the right-click menu for your user
        account. <code>boinc integrate uninstall</code> removes it just as cleanly.
      </p>
    </section>
  );
}
