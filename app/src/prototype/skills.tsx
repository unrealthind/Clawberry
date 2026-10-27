import { useRef, useState } from "react";
import { z } from "zod";
import { Icon, type Agent } from "./shared";

const SkillSchema = z.object({
  id: z.string().max(100),
  name: z.string().min(1).max(80),
  description: z.string().max(400),
  files: z
    .array(
      z.object({
        path: z.string().min(1).max(120),
        content: z.string().max(65536),
      }),
    )
    .min(1)
    .max(20),
});
type Skill = z.infer<typeof SkillSchema>;
const LibrarySchema = z.array(SkillSchema).max(16);
const keyFor = (owner: Agent) =>
  `raaviclaw.prototype.skills.v1.${owner.toLowerCase()}`;
const seed: Record<Agent, Skill[]> = {
  Janet: [
    {
      id: "project-brief",
      name: "Project brief",
      description:
        "Turn project updates into a clear, sourced brief with decisions and next steps.",
      files: [
        {
          path: "SKILL.md",
          content:
            "---\nname: project-brief\ndescription: Prepare a concise work project update from selected sources.\n---\n\n# Project brief\n\n## When to use\nUse when User asks for a project update or a decision brief.\n\n## Workflow\n1. Read the selected project sources.\n2. Separate confirmed facts from assumptions.\n3. Summarize progress, risks, and next steps.\n4. Include source references for important claims.\n5. Prepare drafts for review. External writes require the approved path.\n\n## Output\nA short summary, evidence, decisions needed, and suggested next steps.\n",
        },
        {
          path: "templates/brief.md",
          content:
            "# {{project}}\n\n## Current state\n\n## Evidence\n\n## Risks\n\n## Decisions for User\n\n## Next steps\n",
        },
        {
          path: "scripts/checklist.py",
          content:
            '# Sample helper. Displayed as text; this prototype never executes scripts.\n\ndef open_items(items):\n    return [item for item in items if not item.get("complete", False)]\n',
        },
      ],
    },
    {
      id: "meeting-prep",
      name: "Meeting preparation",
      description:
        "A useful agenda, the right context, and a short list of open decisions.",
      files: [
        {
          path: "SKILL.md",
          content:
            "---\nname: meeting-prep\ndescription: Prepare an agenda and context before a work meeting.\n---\n\n# Meeting preparation\n\nRead only the selected notes and tasks. Draft an agenda with goals,\nopen questions, and the decisions User needs to make.\n\nDo not send invitations or messages without the approved write flow.\n",
        },
      ],
    },
  ],
  Harley: [
    {
      id: "weekend-planner",
      name: "Weekend planner",
      description:
        "A realistic little adventure, with travel time, weather checks, and room to improvise.",
      files: [
        {
          path: "SKILL.md",
          content:
            "---\nname: weekend-planner\ndescription: Help User plan a personal weekend or short trip.\n---\n\n# Weekend planner\n\n## The mission\nMake the weekend feel like a break, not another project.\n\n## Workflow\n1. Confirm available time and personal preferences.\n2. Compare a few options with travel time and estimated costs.\n3. Flag weather and booking uncertainties.\n4. Suggest a flexible plan and one backup.\n\nPrepare booking details for review; never book or pay automatically.\nKeep work information outside this personal workspace.\n",
        },
        {
          path: "templates/packing-list.md",
          content:
            "# Adventure essentials\n\n- [ ] Water\n- [ ] Snacks (non-negotiable)\n- [ ] Layers and rain jacket\n- [ ] Charged phone\n- [ ] Offline route\n",
        },
      ],
    },
  ],
};
function load(owner: Agent): {
  skills: Skill[];
  error: string;
  blocked: boolean;
} {
  try {
    const raw = localStorage.getItem(keyFor(owner));
    if (!raw) return { skills: seed[owner], error: "", blocked: false };
    if (raw.length > 524288) throw new Error();
    return {
      skills: LibrarySchema.parse(JSON.parse(raw)),
      error: "",
      blocked: false,
    };
  } catch {
    return {
      skills: seed[owner],
      error:
        "Stored skills could not be read. Existing browser data has been preserved. This view is read-only until browser storage is available and valid.",
      blocked: true,
    };
  }
}
export function SkillsLibrary({
  owner,
  search,
}: {
  owner: Agent;
  search: string;
}) {
  const [initial] = useState(() => load(owner));
  const [skills, setSkills] = useState(initial.skills);
  const [selectedId, setSelectedId] = useState(initial.skills[0]?.id ?? "");
  const [filePath, setFilePath] = useState("SKILL.md");
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState("");
  const [error, setError] = useState(initial.error);
  const [saved, setSaved] = useState("");
  const [dialogMode, setDialogMode] = useState<"skill" | "file" | null>(null);
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const dialog = useRef<HTMLDialogElement>(null);
  const selected = skills.find((skill) => skill.id === selectedId);
  const file =
    selected?.files.find((item) => item.path === filePath) ??
    selected?.files[0];
  const dirty = editing && draft !== file?.content;
  function persist(next: Skill[]) {
    if (initial.blocked) return false;
    try {
      const parsed = LibrarySchema.parse(next);
      const raw = JSON.stringify(parsed);
      if (raw.length > 524288) throw new Error();
      localStorage.setItem(keyFor(owner), raw);
      setSkills(parsed);
      setError("");
      setSaved("Saved in this browser");
      return true;
    } catch {
      setError(
        "Could not save. Browser storage may be full, or the library exceeds its limit (16 skills, 20 files per skill, 64 KB per file). Your edit is still here; copy or download it before leaving.",
      );
      return false;
    }
  }
  function openDialog(mode: "skill" | "file") {
    setName("");
    setDescription("");
    setError("");
    setDialogMode(mode);
    dialog.current?.showModal();
  }
  function closeDialog() {
    dialog.current?.close();
    setDialogMode(null);
  }
  function download() {
    if (!file) return;
    const blob = new Blob([editing ? draft : file.content], {
      type: "text/plain;charset=utf-8",
    });
    const url = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = url;
    link.download = file.path.split("/").at(-1) ?? "SKILL.md";
    link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  }
  const visible = skills.filter((skill) =>
    `${skill.name} ${skill.description} ${skill.files.map((file) => file.path).join(" ")}`
      .toLowerCase()
      .includes(search.toLowerCase()),
  );
  return (
    <div className="skills-library">
      <div className="skills-library-toolbar">
        <span>
          <Icon name="skills" />
          {skills.length} skills <span>·</span> {owner}’s library
        </span>
        <button
          className="primary-button"
          disabled={dirty || initial.blocked}
          onClick={() => openDialog("skill")}
        >
          <Icon name="plus" />
          Create skill
        </button>
      </div>
      <p className="skills-storage-note">
        <Icon name="shield" />
        Prototype files are stored in this browser only. They are not installed
        or executed. Download files to keep a copy.
      </p>
      {error && (
        <p className="skills-error" role="alert">
          {error}
        </p>
      )}
      <div className="skills-layout">
        <nav className="skill-list" aria-label={`${owner} skills`}>
          {visible.map((skill) => (
            <button
              key={skill.id}
              className={skill.id === selectedId ? "selected-skill" : ""}
              disabled={dirty}
              onClick={() => {
                setSelectedId(skill.id);
                setFilePath("SKILL.md");
                setEditing(false);
                setSaved("");
              }}
            >
              <span className="skill-list-icon">
                <Icon name="skills" />
              </span>
              <strong>{skill.name}</strong>
              <small>{skill.description}</small>
              <span className="skill-file-count">
                {skill.files.length}{" "}
                {skill.files.length === 1 ? "file" : "files"}
              </span>
            </button>
          ))}
          {!visible.length && (
            <p className="skills-empty">
              No matching skills. Create one to get started.
            </p>
          )}
        </nav>
        {selected ? (
          <section className="skill-detail" aria-label="Skill files">
            <div className="skill-detail-heading">
              <div>
                <span className="eyebrow">{owner.toUpperCase()}’S SKILL</span>
                <h3>{selected.name}</h3>
                <p>{selected.description}</p>
              </div>
              <span className="skill-local-tag">Local prototype</span>
            </div>
            <div className="skill-source-layout">
              <div className="skill-file-tree">
                <div className="skill-files-heading">
                  FILES
                  <button
                    className="icon-button"
                    aria-label="Create skill file"
                    disabled={dirty || initial.blocked}
                    onClick={() => openDialog("file")}
                  >
                    <Icon name="plus" />
                  </button>
                </div>
                {selected.files.map((item) => (
                  <button
                    key={item.path}
                    className={file?.path === item.path ? "active-file" : ""}
                    disabled={dirty}
                    onClick={() => {
                      setFilePath(item.path);
                      setEditing(false);
                      setSaved("");
                    }}
                  >
                    <Icon name="file" />
                    <span>{item.path}</span>
                  </button>
                ))}
              </div>
              <div className="skill-source">
                <div className="skill-source-toolbar">
                  <span className="skill-current-path">
                    <Icon name="file" />
                    {file?.path}
                  </span>
                  <div>
                    {editing ? (
                      <>
                        <button
                          className="secondary-button"
                          onClick={() => {
                            setEditing(false);
                            setDraft("");
                          }}
                        >
                          Discard
                        </button>
                        <button
                          className="primary-button"
                          onClick={() => {
                            if (
                              file &&
                              persist(
                                skills.map((skill) =>
                                  skill.id === selected.id
                                    ? {
                                        ...skill,
                                        files: skill.files.map((item) =>
                                          item.path === file.path
                                            ? { ...item, content: draft }
                                            : item,
                                        ),
                                      }
                                    : skill,
                                ),
                              )
                            )
                              setEditing(false);
                          }}
                        >
                          Save file
                        </button>
                      </>
                    ) : (
                      <button
                        className="secondary-button"
                        disabled={initial.blocked}
                        onClick={() => {
                          setDraft(file?.content ?? "");
                          setEditing(true);
                          setSaved("");
                        }}
                      >
                        Edit file
                      </button>
                    )}
                    <button
                      className="icon-button"
                      aria-label="Download skill file"
                      title="Download skill file"
                      onClick={download}
                    >
                      <Icon name="download" />
                    </button>
                  </div>
                </div>
                {editing ? (
                  <textarea
                    className="skill-code-editor"
                    aria-label="Skill file content"
                    spellCheck={false}
                    value={draft}
                    onChange={(event) => setDraft(event.target.value)}
                    maxLength={65536}
                  />
                ) : (
                  <pre
                    className="skill-code-view"
                    tabIndex={0}
                    aria-label="Skill file source"
                  >
                    <code>{file?.content}</code>
                  </pre>
                )}
                <div className="skill-editor-footer">
                  <span>
                    {dirty
                      ? "Unsaved changes · Save or discard before changing files"
                      : saved || "Source file · Plain text"}
                  </span>
                  <span>
                    {
                      (editing ? draft : (file?.content ?? "")).split("\n")
                        .length
                    }{" "}
                    lines
                  </span>
                </div>
              </div>
            </div>
          </section>
        ) : (
          <div className="skills-empty">Create your first skill.</div>
        )}
      </div>
      <dialog
        ref={dialog}
        className="prototype-dialog skill-create-dialog"
        aria-labelledby="skill-dialog-title"
        onCancel={closeDialog}
        onClick={(event) => {
          if (event.target === event.currentTarget) closeDialog();
        }}
      >
        <div className="dialog-top">
          <span className="eyebrow">{owner.toUpperCase()}’S LIBRARY</span>
          <button
            className="icon-button"
            aria-label="Close skill dialog"
            onClick={closeDialog}
          >
            <Icon name="close" />
          </button>
        </div>
        <h2 id="skill-dialog-title">
          {dialogMode === "skill" ? "Create a skill" : "Create a file"}
        </h2>
        <p className="dialog-description">
          {dialogMode === "skill"
            ? "Give a reusable workflow its own instructions and supporting files."
            : "Add a script, template, or reference file. File contents stay as text in this prototype."}
        </p>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            const value = name.trim();
            if (!value) return;
            if (dialogMode === "skill") {
              const slug = value
                .toLowerCase()
                .replace(/[^a-z0-9]+/g, "-")
                .replace(/^-|-$/g, "");
              if (
                !slug ||
                skills.some(
                  (skill) => skill.name.toLowerCase() === value.toLowerCase(),
                )
              ) {
                setError(
                  "Choose a unique skill name containing letters or numbers.",
                );
                return;
              }
              const skill: Skill = {
                id: crypto.randomUUID(),
                name: value,
                description: description.trim(),
                files: [
                  {
                    path: "SKILL.md",
                    content: `---\nname: ${slug}\ndescription: ${JSON.stringify(description.trim())}\n---\n\n# ${value}\n\n## When to use\nDescribe when ${owner} should use this skill.\n\n## Workflow\n1. Define the first step.\n2. Gather evidence and prepare the result.\n3. Keep external actions within the approved write path.\n\n## Output\nDescribe the expected result.\n`,
                  },
                ],
              };
              if (persist([...skills, skill])) {
                setSelectedId(skill.id);
                setFilePath("SKILL.md");
                setEditing(false);
                closeDialog();
              }
            } else if (selected) {
              if (
                !/^[a-zA-Z0-9_-]+(?:\/[a-zA-Z0-9_-]+)*\.[a-zA-Z0-9]+$/.test(
                  value,
                ) ||
                selected.files.some(
                  (file) => file.path.toLowerCase() === value.toLowerCase(),
                )
              ) {
                setError(
                  "Use a unique relative file path, such as scripts/helper.py or templates/notes.md. Spaces and parent paths are not supported.",
                );
                return;
              }
              if (
                persist(
                  skills.map((skill) =>
                    skill.id === selected.id
                      ? {
                          ...skill,
                          files: [...skill.files, { path: value, content: "" }],
                        }
                      : skill,
                  ),
                )
              ) {
                setFilePath(value);
                setDraft("");
                setEditing(true);
                closeDialog();
              }
            }
          }}
        >
          <label className="field-label" htmlFor={`skill-name-${owner}`}>
            {dialogMode === "skill" ? "Skill name" : "File path"}
          </label>
          <input
            id={`skill-name-${owner}`}
            className="thread-name-input"
            value={name}
            onChange={(event) => setName(event.target.value)}
            maxLength={dialogMode === "skill" ? 80 : 120}
            placeholder={
              dialogMode === "skill"
                ? "For example, Video review"
                : "scripts/helper.py"
            }
            required
          />
          {dialogMode === "skill" && (
            <>
              <label
                className="field-label"
                htmlFor={`skill-description-${owner}`}
              >
                Description
              </label>
              <textarea
                id={`skill-description-${owner}`}
                className="thread-name-input"
                rows={3}
                value={description}
                onChange={(event) => setDescription(event.target.value)}
                maxLength={400}
                placeholder="What does this skill help with?"
                required
              />
            </>
          )}
          {error && (
            <p className="skills-error" role="alert">
              {error}
            </p>
          )}
          <div className="dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={closeDialog}
            >
              Cancel
            </button>
            <button className="primary-button" disabled={!name.trim()}>
              {dialogMode === "skill" ? "Create skill" : "Create file"}
              <Icon name="plus" />
            </button>
          </div>
        </form>
      </dialog>
    </div>
  );
}
