// Boinc portal: the hero demo, OS detection, and latest-release links.
// Everything degrades: without JS the page still reads and every download
// button points at the GitHub releases page.

(function () {
  "use strict";

  var REPO = "bartbeecoders/boinc";
  var RELEASES = "https://github.com/" + REPO + "/releases/latest";

  /* ---------- hero demo: convert vacation.png back and forth ---------- */

  var state = "png"; // current format of the demo file
  var card = document.getElementById("file-card");
  var fileName = document.getElementById("file-name");
  var fileBadge = document.getElementById("file-badge");
  var targetFormat = document.getElementById("target-format");
  var convertBtn = document.getElementById("convert-btn");
  var toast = document.getElementById("toast");
  var toastName = document.getElementById("toast-name");
  var toastTitle = toast.querySelector("strong");
  var hint = document.getElementById("demo-hint");
  var undoBtn = document.getElementById("undo-btn");
  var toastTimer = null;
  var reduced = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  function applyState() {
    var ext = state;
    var other = state === "png" ? "jpg" : "png";
    fileName.textContent = "vacation." + ext;
    fileBadge.textContent = ext.toUpperCase();
    targetFormat.textContent = other.toUpperCase();
  }

  function convert() {
    var from = state;
    var to = state === "png" ? "jpg" : "png";
    convertBtn.disabled = true;
    card.classList.add("converting");
    hint.style.opacity = "0";

    window.setTimeout(function () {
      state = to;
      applyState();
      card.classList.remove("converting");
      convertBtn.disabled = false;

      toastTitle.textContent = "Converted vacation." + from;
      toastName.textContent = "vacation." + to;
      undoBtn.textContent = "Convert it back";
      toast.classList.add("show");
      window.clearTimeout(toastTimer);
      toastTimer = window.setTimeout(function () {
        toast.classList.remove("show");
      }, 6000);
    }, reduced ? 0 : 700);
  }

  convertBtn.addEventListener("click", convert);
  undoBtn.addEventListener("click", function () {
    toast.classList.remove("show");
    convert();
  });

  applyState();

  /* ---------- OS detection for the primary button ---------- */

  var ua = navigator.userAgent;
  var os = /Windows/i.test(ua) ? "windows"
    : /Mac OS X|Macintosh/i.test(ua) ? "mac"
    : /Linux|X11/i.test(ua) ? "linux"
    : null;

  var osLabel = { linux: "Linux", windows: "Windows", mac: "macOS" };
  var primary = document.getElementById("primary-download");
  if (os) {
    primary.textContent = "Download for " + osLabel[os];
    document.querySelectorAll('.dl-card[data-os="' + os + '"]').forEach(function (el) {
      el.classList.add("detected");
    });
  }

  /* ---------- point buttons at the latest release assets ---------- */

  fetch("https://api.github.com/repos/" + REPO + "/releases/latest")
    .then(function (res) { return res.ok ? res.json() : null; })
    .then(function (release) {
      if (!release || !release.assets) return;

      var byExt = {};
      release.assets.forEach(function (asset) {
        var m = asset.name.match(/\.(deb|rpm|msi|dmg)$/);
        if (m) byExt["." + m[1]] = asset.browser_download_url;
      });

      document.querySelectorAll("[data-asset]").forEach(function (btn) {
        var url = byExt[btn.getAttribute("data-asset")];
        if (url) btn.href = url;
      });

      var primaryExt = { linux: ".deb", windows: ".msi", mac: ".dmg" }[os];
      if (primaryExt && byExt[primaryExt]) primary.href = byExt[primaryExt];

      var note = document.getElementById("release-note");
      if (release.tag_name) {
        note.textContent = "Latest release: " + release.tag_name + " — straight from GitHub.";
      }
    })
    .catch(function () { /* buttons keep their releases-page fallback */ });
})();
