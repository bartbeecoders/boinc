import Demo from "./components/Demo.jsx";
import Downloads from "./components/Downloads.jsx";
import Faq from "./components/Faq.jsx";
import LinuxSetup from "./components/LinuxSetup.jsx";
import {
  OS_LABEL,
  PRIMARY_ASSET,
  RELEASES_URL,
  detectOs,
  useLatestRelease,
} from "./lib/release.js";
import appWindow from "./assets/app-window.png";

const REPO_URL = RELEASES_URL.replace("/releases/latest", "");
const os = detectOs();

export default function App() {
  const release = useLatestRelease();
  const primaryHref =
    (os && release.assets[PRIMARY_ASSET[os]]) || RELEASES_URL;

  return (
    <>
      <nav className="nav">
        <a className="brand" href="#top">
          <span className="disc" aria-hidden="true" />boinc
        </a>
        <div className="nav-links">
          <a href="#how">How it works</a>
          <a href="#downloads">Downloads</a>
          <a href="#linux-setup">Linux setup</a>
          <a href="#faq">FAQ</a>
          <a href={REPO_URL}>GitHub</a>
        </div>
      </nav>

      <header className="hero" id="top">
        <div className="hero-copy">
          <p className="eyebrow">File conversion for the desktop</p>
          <h1>Right-click.<br />Converted.</h1>
          <p className="lede">
            Boinc adds <em>Convert&nbsp;to…</em> to your file manager's right-click menu.
            Files convert on your machine, next to the originals — nothing is uploaded
            anywhere.
          </p>
          <div className="cta-row">
            <a className="btn-primary" href={primaryHref}>
              {os ? `Download for ${OS_LABEL[os]}` : "Download Boinc"}
            </a>
            <a className="btn-quiet" href="#downloads">All platforms ↓</a>
          </div>
          <p className="fine">Free &amp; open source (MIT) · Linux, Windows, macOS</p>
        </div>

        <Demo />
      </header>

      <main>
        <section className="section" id="how">
          <h2><span className="disc small" aria-hidden="true" />How it works</h2>
          <ol className="steps">
            <li>
              <h3>Right-click a file</h3>
              <p>Boinc appears in the menu of every file it can convert — and only those.</p>
            </li>
            <li>
              <h3>Pick the target format</h3>
              <p>
                Only valid conversions are offered. A PNG shows <em>Convert to JPG</em>,
                nothing else.
              </p>
            </li>
            <li>
              <h3>Done</h3>
              <p>
                The converted copy lands next to the original. Boinc never overwrites your
                files — a second convert becomes <code>photo&nbsp;(1).jpg</code>.
              </p>
            </li>
          </ol>
        </section>

        <section className="section" id="formats">
          <h2><span className="disc small" aria-hidden="true" />What it converts</h2>
          <div className="format-grid">
            <div className="format-card">
              <div className="pair">
                <code>PNG</code><span className="arrows">⇄</span><code>JPG</code>
              </div>
              <p>
                Both directions, out of the box. Transparency is flattened onto a background
                color you choose; JPEG quality is configurable.
              </p>
            </div>
            <div className="format-card">
              <div className="pair">
                <code>PDF</code><span className="arrows">⇄</span><code>DOCX</code>
              </div>
              <p>
                Both directions, powered by <a href="https://www.libreoffice.org/">LibreOffice</a>{" "}
                when it's installed. No LibreOffice — no menu entry; Boinc doesn't offer what
                it can't do.
              </p>
            </div>
            <div className="format-card">
              <div className="pair">
                <code>PDF</code><span className="arrows">⇄</span><code>MD</code>
              </div>
              <p>
                PDF → Markdown pulls out the text layer with no extra tools needed.
                Markdown → PDF is typeset by LibreOffice, like the document pair above.
              </p>
            </div>
          </div>
          <p className="aside">
            Under the hood every conversion is a plug-in against one small engine, so new
            formats join the menu without redesigning anything.
          </p>
        </section>

        <section className="section" id="app">
          <div className="app-split">
            <div>
              <h2><span className="disc small" aria-hidden="true" />There's an app, too</h2>
              <p>
                The tray application handles everything the menu doesn't: drop in files, queue
                big batches, watch progress, pause, and get a desktop notification when each
                file is ready.
              </p>
              <ul className="plain-list">
                <li>Drag &amp; drop any convertible file</li>
                <li>Sequential queue with per-file progress</li>
                <li>Default output folder and JPEG quality in Settings</li>
                <li>Start at login, sit in the tray</li>
              </ul>
            </div>
            <figure className="shot">
              <img
                src={appWindow}
                alt="The Boinc window: a drop zone, a pending file with a Convert to JPG button, and a Settings button"
                width="520"
                height="210"
                loading="lazy"
              />
              <figcaption>The actual window — this screenshot is from the app, not a mock-up.</figcaption>
            </figure>
          </div>
        </section>

        <Downloads os={os} release={release} />
        <LinuxSetup />
        <Faq />
      </main>

      <footer className="footer">
        <span><span className="disc small" aria-hidden="true" />Boinc</span>
        <span>
          MIT License · <a href={REPO_URL}>Source on GitHub</a> · a hideterms.com project
        </span>
      </footer>
    </>
  );
}
