// Per-distro install steps and per-file-manager pointers. Two numbered
// stages because it really is a sequence: install the package, then find
// the menu where your file manager puts it.

function Cmd({ children }) {
  return (
    <pre className="cmd">
      <code>{children}</code>
    </pre>
  );
}

export default function LinuxSetup() {
  return (
    <section className="section" id="linux-setup">
      <h2><span className="disc small" aria-hidden="true" />Installing on Linux</h2>
      <p className="aside">
        Two steps everywhere: install the package for your distro, then open Boinc once —
        the first launch registers the right-click menu for your user account. No root
        hooks, no daemons; <code>boinc integrate uninstall</code> removes every trace.
      </p>

      <h3 className="setup-sub">1. Install the package for your distro</h3>

      <details>
        <summary>Debian · Ubuntu · Linux Mint · Pop!_OS (.deb)</summary>
        <p>
          Download the <code>.deb</code> above, then install it with apt — it resolves the
          GTK dependency for you:
        </p>
        <Cmd>{`sudo apt install ./boinc_*_amd64.deb
boinc-app   # first launch sets up the right-click menu`}</Cmd>
        <p>
          For PDF ↔ DOCX and Markdown → PDF conversions, also install LibreOffice:{" "}
          <code>sudo apt install libreoffice-writer</code>.
        </p>
      </details>

      <details>
        <summary>Fedora (.rpm)</summary>
        <Cmd>{`sudo dnf install ./boinc-*.x86_64.rpm
boinc-app   # first launch sets up the right-click menu`}</Cmd>
        <p>
          LibreOffice for the document conversions: <code>sudo dnf install libreoffice-writer</code>.
        </p>
      </details>

      <details>
        <summary>Fedora Asahi Remix — Apple Silicon (.rpm, aarch64)</summary>
        <Cmd>{`sudo dnf install ./boinc-*.aarch64.rpm
boinc-app   # first launch sets up the right-click menu`}</Cmd>
        <p>
          Native aarch64 build for Apple M-series Macs running{" "}
          <a href="https://asahilinux.org/fedora/">Fedora Asahi Remix</a>. The default KDE
          Plasma spin gets the Dolphin right-click menu out of the box; LibreOffice for the
          document conversions: <code>sudo dnf install libreoffice-writer</code>.
        </p>
      </details>

      <details>
        <summary>openSUSE (.rpm)</summary>
        <Cmd>{`sudo zypper install ./boinc-*.x86_64.rpm
boinc-app   # first launch sets up the right-click menu`}</Cmd>
      </details>

      <details>
        <summary>Arch · EndeavourOS · Manjaro (build from source)</summary>
        <p>No Arch package yet, but the build is two commands with rustup installed:</p>
        <Cmd>{`sudo pacman -S --needed rust gtk3 xdotool
git clone https://github.com/bartbeecoders/boinc.git && cd boinc
cargo build --release
sudo install -Dm755 -t /usr/local/bin target/release/boinc target/release/boinc-app
boinc-app   # first launch sets up the right-click menu`}</Cmd>
        <p>
          <code>gtk3</code> is for the tray icon, <code>xdotool</code> provides libxdo. For
          PDF ↔ DOCX and Markdown → PDF: <code>sudo pacman -S libreoffice-fresh</code>.
        </p>
      </details>

      <h3 className="setup-sub">2. Find the menu in your file manager</h3>

      <details>
        <summary>Nemo — Cinnamon (Linux Mint, EndeavourOS Cinnamon)</summary>
        <p>
          Right-click any convertible file: the entries appear directly in the menu, e.g.{" "}
          <em>Convert to JPG (Boinc)</em>. Nemo watches its actions folder, so they show up
          immediately — no restart needed.
        </p>
      </details>

      <details>
        <summary>Dolphin — KDE Plasma (Kubuntu, Fedora KDE, EndeavourOS KDE)</summary>
        <p>
          Right-click a file and look for the <em>Boinc</em> submenu (on some Plasma
          versions it sits under <em>Actions</em>). Only valid targets are listed — a PNG
          shows options like <em>Convert to JPG</em> and <em>Convert to SVG</em>, not
          document formats.
        </p>
      </details>

      <details>
        <summary>Nautilus / Files — GNOME (Ubuntu, Fedora Workstation)</summary>
        <p>
          Right-click a file → <em>Scripts</em> → e.g. <em>PNG to JPG (Boinc)</em>. GNOME's
          file manager can't scope entries by file type, so the scripts are listed for all
          files; unsupported ones simply report a notification. If the Scripts submenu
          doesn't appear right away, restart Nautilus:
        </p>
        <Cmd>nautilus -q</Cmd>
      </details>

      <p className="aside">
        Something missing? <code>boinc integrate status</code> lists every installed hook,
        and re-running <code>boinc integrate install</code> refreshes them — do that after
        installing LibreOffice so the PDF, DOCX, and Markdown entries appear.
      </p>
    </section>
  );
}
