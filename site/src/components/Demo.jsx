// The hero demo: a recreated right-click menu that "converts" vacation.png
// back and forth, with a desktop-style toast whose undo is the reverse
// conversion.

import { useEffect, useRef, useState } from "react";

const CONVERT_MS = 700;
const TOAST_MS = 6000;

export default function Demo() {
  const [format, setFormat] = useState("png");
  const [converting, setConverting] = useState(false);
  const [hintHidden, setHintHidden] = useState(false);
  // Toast content persists while it slides out, so visibility is separate.
  const [toast, setToast] = useState({ from: "png", to: "jpg" });
  const [toastVisible, setToastVisible] = useState(false);
  const timers = useRef([]);

  useEffect(() => {
    const pending = timers.current;
    return () => pending.forEach(clearTimeout);
  }, []);

  const target = format === "png" ? "jpg" : "png";

  function convert() {
    if (converting) return;
    const from = format;
    const to = target;
    setConverting(true);
    setHintHidden(true);
    setToastVisible(false);

    const reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
    timers.current.push(
      setTimeout(
        () => {
          setFormat(to);
          setConverting(false);
          setToast({ from, to });
          setToastVisible(true);
          timers.current.push(setTimeout(() => setToastVisible(false), TOAST_MS));
        },
        reduced ? 0 : CONVERT_MS,
      ),
    );
  }

  function undo() {
    setToastVisible(false);
    convert();
  }

  return (
    <div className="demo" aria-label="Interactive demo of the Boinc context menu">
      <div className={`file-card${converting ? " converting" : ""}`}>
        <svg className="file-icon" viewBox="0 0 40 48" aria-hidden="true">
          <path className="page" d="M4 2h22l10 10v34H4z" />
          <path className="fold" d="M26 2l10 10H26z" />
          <text x="20" y="38" textAnchor="middle">{format.toUpperCase()}</text>
        </svg>
        <div>
          <div className="file-name">vacation.{format}</div>
          <div className="file-meta">2.4 MB · today 14:32</div>
        </div>
        <div className="scan" aria-hidden="true" />
      </div>

      <div className="menu" role="menu" aria-label="File context menu">
        <div className="menu-item dim" role="menuitem" aria-disabled="true">Open</div>
        <div className="menu-item dim" role="menuitem" aria-disabled="true">Open with…</div>
        <div className="menu-sep" />
        <div className="menu-item dim" role="menuitem" aria-disabled="true">Copy</div>
        <div className="menu-item dim" role="menuitem" aria-disabled="true">Rename…</div>
        <div className="menu-sep" />
        <div className="menu-item selected" aria-expanded="true">
          <span className="disc small" aria-hidden="true" />
          Boinc
          <span className="chev" aria-hidden="true">▸</span>
        </div>
        <div className="menu-item dim" role="menuitem" aria-disabled="true">Properties</div>

        <div className="submenu">
          <button
            className="menu-item action"
            type="button"
            onClick={convert}
            disabled={converting}
          >
            <span>Convert to {target.toUpperCase()}</span>
          </button>
        </div>
      </div>
      <p className="demo-hint" style={hintHidden ? { opacity: 0 } : undefined}>
        Go on — click it. That's the whole product.
      </p>

      <div className={`toast${toastVisible ? " show" : ""}`} role="status" aria-live="polite">
        <span className="disc small" aria-hidden="true" />
        <div>
          <strong>Converted vacation.{toast.from}</strong>
          <div className="toast-body">
            Saved as <code>vacation.{toast.to}</code> — next to the original.
          </div>
        </div>
        <button className="toast-undo" type="button" onClick={undo}>
          Convert it back
        </button>
      </div>
    </div>
  );
}
