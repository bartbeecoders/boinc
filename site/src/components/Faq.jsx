import { RELEASES_URL } from "../lib/release.js";

const REPO_URL = RELEASES_URL.replace("/releases/latest", "");

const QUESTIONS = [
  {
    q: "Do my files get uploaded somewhere?",
    a: (
      <p>
        No. Boinc is a desktop program; conversions run entirely on your computer. This
        website's only job is to hand you the installer.
      </p>
    ),
  },
  {
    q: "Why don't I see PDF → DOCX in my menu?",
    a: (
      <p>
        Conversions that produce PDF or DOCX use LibreOffice as their engine. Install{" "}
        <a href="https://www.libreoffice.org/">LibreOffice</a>, open Boinc once, and the
        entries appear. (PDF → Markdown needs no extra tools.)
      </p>
    ),
  },
  {
    q: "Can it overwrite my original?",
    a: (
      <p>
        Never. The original stays untouched and the copy gets a free name —{" "}
        <code>report.pdf</code> becomes <code>report.docx</code>, or{" "}
        <code>report&nbsp;(1).docx</code> if that name is taken.
      </p>
    ),
  },
  {
    q: "Can I convert many files at once?",
    a: (
      <p>
        Yes — select several files before right-clicking (Linux), drop a batch on the app
        window, or use the CLI: <code>boinc convert *.png --to jpg</code>.
      </p>
    ),
  },
  {
    q: "Is it really free?",
    a: (
      <p>
        MIT-licensed, source on <a href={REPO_URL}>GitHub</a>. No account, no tier, no "3
        conversions per day".
      </p>
    ),
  },
];

export default function Faq() {
  return (
    <section className="section" id="faq">
      <h2><span className="disc small" aria-hidden="true" />Questions, answered</h2>
      {QUESTIONS.map(({ q, a }) => (
        <details key={q}>
          <summary>{q}</summary>
          {a}
        </details>
      ))}
    </section>
  );
}
