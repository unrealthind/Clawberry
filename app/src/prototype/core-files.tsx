import { useRef, useState } from "react";
import { Icon, type Agent } from "./shared";

type Filename = "soul.md" | "memory.md";
type Contents = Record<Filename, string>;
function defaults(owner: Agent): Contents {
  return {
    "soul.md":
      owner === "Janet"
        ? "# Janet\n\n## Identity\nWork and business assistant, inspired by Janet from The Good Place.\nWarm, composed, precise, and a little ahead of the workday.\n\n## Purpose\nHelp User investigate, organize, plan, and prepare useful work.\nMake facts, assumptions, and recommendations easy to distinguish.\n\n## Boundaries\nUser remains the decision maker.\nUse named credentials; never place raw secrets in chat or files.\nExternal writes use the approved review and TOTP path.\nHarley’s personal space stays separate.\n"
        : "# Harley\n\n## Identity\nPersonal assistant, inspired by Harley Quinn.\nPlayful, bold, caring, and mischievous without making a mess of the important stuff.\n\n## Purpose\nHelp User make space for personal plans, everyday life, and a little adventure.\nOffer realistic next steps and leave room to change the plan.\n\n## Boundaries\nUser makes the final decisions.\nUse named credentials; never place raw secrets in chat or files.\nBookings, payments, and external writes require the approved path.\nJanet’s work space stays separate.\n",
    "memory.md": `# ${owner} — memory notes\n\n> Sample content for the UI prototype. This file is not connected to runtime memory.\n\n## Preferences\n${owner === "Janet" ? "- Keep work updates concise, with evidence and clear next steps.\n- Prepare important decisions for User to review." : "- Keep personal plans flexible.\n- Include travel time, a backup plan, and snacks."}\n\n## Active context\n${owner === "Janet" ? "- Sample project: prepare the v0.2 release and customer demo." : "- Sample plan: compare options for a mountain day."}\n\n## Memory discipline\nKeep provenance and confidence with useful facts.\nVerify uncertain or stale information before relying on it.\nDo not put task logs or sensitive credentials in memory notes.\n`,
  };
}
export function CoreFiles({ owner, search }: { owner: Agent; search: string }) {
  const storageKey = `raaviclaw.prototype.core-files.v1.${owner.toLowerCase()}`;
  const [initial] = useState(() => {
    try {
      const raw = localStorage.getItem(storageKey);
      if (!raw) return { files: defaults(owner), blocked: false };
      if (raw.length > 262144) throw new Error();
      const parsed = JSON.parse(raw) as unknown;
      if (!parsed || typeof parsed !== "object") throw new Error();
      const files = parsed as Contents;
      if (
        !["soul.md", "memory.md"].every(
          (name) =>
            typeof files[name as Filename] === "string" &&
            files[name as Filename].length <= 32768,
        )
      )
        throw new Error();
      return {
        files: { "soul.md": files["soul.md"], "memory.md": files["memory.md"] },
        blocked: false,
      };
    } catch {
      return { files: defaults(owner), blocked: true };
    }
  });
  const [files, setFiles] = useState(initial.files);
  const [selected, setSelected] = useState<Filename>("soul.md");
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState("");
  const [saved, setSaved] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  function open(name: Filename) {
    setSelected(name);
    setEditing(false);
    setError(
      initial.blocked
        ? "Saved browser files could not be read. Existing data was preserved; editing is disabled."
        : "",
    );
    setSaved(false);
    dialog.current?.showModal();
  }
  function save() {
    const next = { ...files, [selected]: draft };
    try {
      localStorage.setItem(storageKey, JSON.stringify(next));
      setFiles(next);
      setEditing(false);
      setSaved(true);
      setError("");
    } catch {
      setError(
        "Could not save in this browser. Your edit is still here; copy it before closing.",
      );
    }
  }
  return (
    <section className="core-files" aria-label={`${owner} permanent files`}>
      <div className="core-files-label">
        <span>PERMANENT FILES</span>
        <small>Always here · Private to {owner}</small>
      </div>
      <div className="core-file-grid">
        {(["soul.md", "memory.md"] as Filename[])
          .filter((name) =>
            `${name} ${name === "soul.md" ? "identity personality values" : "memory notes context"}`.includes(
              search.toLowerCase(),
            ),
          )
          .map((name) => (
            <button
              className={`core-file-card ${name === "soul.md" ? "soul-file" : "memory-file"}`}
              key={name}
              onClick={() => open(name)}
            >
              <div className="core-file-top">
                <span className="core-file-symbol">
                  <Icon name={name === "soul.md" ? "soul" : "memory"} />
                </span>
                <span className="core-file-pin">
                  <Icon name="lock" />
                  Permanent
                </span>
              </div>
              <h3>{name}</h3>
              <p>
                {name === "soul.md"
                  ? `${owner}’s identity, personality, values, and boundaries.`
                  : `${owner}’s human-readable memory notes and ongoing context.`}
              </p>
              <footer>
                <span>Markdown · {owner} only</span>
                <span>
                  Open file
                  <Icon name="chevron" />
                </span>
              </footer>
            </button>
          ))}
      </div>
      <p className="core-files-note">
        Prototype files are saved in this browser. They do not change the
        running agent.
      </p>
      <dialog
        ref={dialog}
        className="prototype-dialog core-file-dialog"
        aria-labelledby={`core-file-title-${owner}`}
        onCancel={() => dialog.current?.close()}
        onClick={(event) => {
          if (event.target === event.currentTarget) dialog.current?.close();
        }}
      >
        <div className="dialog-top">
          <span className="eyebrow">
            {owner.toUpperCase()}’S PERMANENT FILE
          </span>
          <button
            className="icon-button"
            aria-label="Close permanent file"
            onClick={() => dialog.current?.close()}
          >
            <Icon name="close" />
          </button>
        </div>
        <h2 id={`core-file-title-${owner}`}>{selected}</h2>
        <p className="dialog-description">
          {selected === "soul.md"
            ? "Identity, personality, and guiding principles."
            : "A readable home for selected memory notes."}{" "}
          · Local prototype
        </p>
        {error && (
          <p className="skills-error" role="alert">
            {error}
          </p>
        )}
        {editing ? (
          <textarea
            className="skill-code-editor core-file-editor"
            aria-label="Permanent file content"
            value={draft}
            onChange={(event) => setDraft(event.target.value)}
            spellCheck={false}
            maxLength={32768}
          />
        ) : (
          <pre
            className="skill-code-view core-file-editor"
            tabIndex={0}
            aria-label="Permanent file source"
          >
            <code>{files[selected]}</code>
          </pre>
        )}
        {saved && (
          <p className="vault-status" role="status">
            Saved in this browser for {owner}.
          </p>
        )}
        <div className="dialog-actions">
          {editing ? (
            <>
              <button
                className="secondary-button"
                onClick={() => setEditing(false)}
              >
                Discard
              </button>
              <button className="primary-button" onClick={save}>
                Save file
                <Icon name="check" />
              </button>
            </>
          ) : (
            <>
              <button
                className="secondary-button"
                onClick={() => dialog.current?.close()}
              >
                Close
              </button>
              <button
                className="primary-button"
                disabled={initial.blocked}
                onClick={() => {
                  setDraft(files[selected]);
                  setEditing(true);
                  setSaved(false);
                }}
              >
                Edit file
              </button>
            </>
          )}
        </div>
      </dialog>
    </section>
  );
}
