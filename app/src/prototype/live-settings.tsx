import { useEffect, useState } from "react";
import { request, isLoopbackHost, type Agent } from "../api";
import { claudeModels } from "./model-settings";
import { Icon } from "./shared";
export type Versioned<T> = { revision: number; value: T };
export type ModelProfile = {
  provider: string;
  model: string;
  max_output_tokens: number;
  input_microusd_per_million: number;
  output_microusd_per_million: number;
};
export type GlobalSettings = {
  preferences: {
    theme: "light" | "dark";
    notifications: boolean;
    week_start: "sunday" | "monday";
    time_format: "12" | "24";
    show_completed: boolean;
  };
  models: {
    profiles: Record<"fast" | "normal" | "deep", ModelProfile>;
    agent_daily_microusd: number;
    deep_daily_microusd: number;
    task_microusd: number;
  };
};
type Skill = {
  id: string;
  name: string;
  description: string;
  files: { path: string; content: string }[];
};
export type AgentSettings = {
  soul: string;
  memory_notes: string;
  default_profile: "fast" | "normal" | "deep";
  skills: Skill[];
};
export type RuntimeStatus = {
  credential_configured: boolean;
  credential_source: string;
  live_verified: boolean;
};
export function useServerSettings<T>(token: string, path: string) {
  const [data, setData] = useState<Versioned<T>>();
  const [error, setError] = useState("");
  const [loading, setLoading] = useState(true);
  const [reload, setReload] = useState(0);
  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError("");
    request<Versioned<T>>(token, path, undefined, controller.signal)
      .then((value) => {
        if (!controller.signal.aborted) setData(value);
      })
      .catch(() => {
        if (!controller.signal.aborted)
          setError(
            "Settings could not be loaded. Retry when the gateway is available.",
          );
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [token, path, reload]);
  return {
    data,
    setData,
    error,
    loading,
    retry: () => setReload((n) => n + 1),
  };
}
function ClaudeCredential({ token }: { token: string }) {
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  async function saveCredential() {
    if (busy || !value.trim()) return;
    setBusy(true);
    setMessage("");
    const secret = value.trim();
    setValue("");
    try {
      await request(token, "/api/settings/claude-credential", {
        token: secret,
      });
      setMessage(
        "Claude API token saved securely. New chat requests use it immediately.",
      );
    } catch {
      setMessage(
        "Token save was not confirmed. Check the connection and enter the token again to replace it.",
      );
    } finally {
      setBusy(false);
    }
  }
  return (
    <div>
      <label>
        Anthropic API token
        <input
          onKeyDown={(event) => {
            if (event.key === "Enter") {
              event.preventDefault();
              void saveCredential();
            }
          }}
          type="password"
          autoComplete="new-password"
          spellCheck={false}
          value={value}
          maxLength={4096}
          disabled={busy}
          placeholder="sk-ant-…"
          onChange={(event) => setValue(event.target.value)}
        />
      </label>
      <p>
        Shared by Janet and Harley for Claude model requests. Stored encrypted
        on the backend; the saved token is never displayed. Replacing it takes
        effect for new requests.
      </p>
      <button
        type="button"
        onClick={() => void saveCredential()}
        className="primary-button"
        disabled={busy || !value.trim()}
      >
        {busy ? "Saving token…" : "Save API token"}
      </button>
      {message && <p role="status">{message}</p>}
    </div>
  );
}
export function LiveSettings({
  token,
  owner,
  global,
  onGlobal,
  onDirty,
  onChat,
  onApprovals,
  status,
}: {
  token: string;
  owner: Agent | null;
  global: Versioned<GlobalSettings> | undefined;
  onGlobal: (value: Versioned<GlobalSettings>) => void;
  onDirty: (dirty: boolean) => void;
  onChat: () => void;
  onApprovals: () => void;
  status: RuntimeStatus | undefined;
}) {
  const path = owner ? `/api/agents/${owner}/settings` : "/api/settings/global";
  const settings = useServerSettings<GlobalSettings | AgentSettings>(
    token,
    path,
  );
  const [draft, setDraft] = useState<GlobalSettings | AgentSettings>();
  const [section, setSection] = useState("general");
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");
  const [error, setError] = useState("");
  useEffect(() => {
    if (settings.data) setDraft(structuredClone(settings.data.value));
  }, [settings.data]);
  const dirty =
    !!draft &&
    !!settings.data &&
    JSON.stringify(draft) !== JSON.stringify(settings.data.value);
  useEffect(() => {
    onDirty(dirty);
    return () => onDirty(false);
  }, [dirty, onDirty]);
  useEffect(() => {
    const leave = (event: BeforeUnloadEvent) => {
      if (dirty) {
        event.preventDefault();
        event.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", leave);
    return () => window.removeEventListener("beforeunload", leave);
  }, [dirty]);
  const title = owner
    ? `${owner === "janet" ? "Janet" : "Harley"} settings`
    : "App settings";
  const choices = owner
    ? ([
        {
          id: "general",
          label: "Identity & memory",
          icon: "soul",
          description: "The guidance and notes for this agent.",
        },
        {
          id: "skills",
          label: "Skills",
          icon: "skills",
          description: "Reusable instructions for this agent’s work.",
        },
        {
          id: "credentials",
          label: "Credentials",
          icon: "lock",
          description: "Credential setup for this agent, kept outside chat.",
        },
        {
          id: "access",
          label: "Access & boundaries",
          icon: "shield",
          description: "The boundaries that keep you in control.",
        },
      ] as const)
    : ([
        {
          id: "general",
          label: "General",
          icon: "settings",
          description: "A workspace that works the way you do.",
        },
        {
          id: "models",
          label: "Claude & models",
          icon: "soul",
          description: "Choose the models behind your assistants.",
        },
        {
          id: "notifications",
          label: "Notifications",
          icon: "activity",
          description: "Choose how your workspace keeps you informed.",
        },
        {
          id: "calendar",
          label: "Calendar",
          icon: "calendar",
          description: "Your preferred view of dates and deadlines.",
        },
        {
          id: "connection",
          label: "Connection & security",
          icon: "shield",
          description: "Your local connection and security boundaries.",
        },
      ] as const);
  const current = choices.find((item) => item.id === section) ?? choices[0];
  if (settings.loading) return <p role="status">Loading {title}…</p>;
  if (settings.error)
    return (
      <div>
        <p role="alert">{settings.error}</p>
        <button className="secondary-button" onClick={settings.retry}>
          Retry settings
        </button>
      </div>
    );
  if (!draft || !settings.data) return null;
  const app = draft as GlobalSettings,
    agent = draft as AgentSettings;
  async function save(e: React.FormEvent) {
    e.preventDefault();
    if (!draft || !settings.data || busy) return;
    setBusy(true);
    setError("");
    setMessage("");
    try {
      const saved = await request<Versioned<GlobalSettings | AgentSettings>>(
        token,
        path,
        { revision: settings.data.revision, value: draft },
      );
      settings.setData(saved);
      setDraft(structuredClone(saved.value));
      if (!owner) onGlobal(saved as Versioned<GlobalSettings>);
      setMessage("Saved to runtime. Changes apply to subsequent requests.");
    } catch (e) {
      setError(
        e instanceof Error
          ? e.message
          : "Save could not be confirmed. Your draft is still here.",
      );
    } finally {
      setBusy(false);
    }
  }
  const updateAgent = (patch: Partial<AgentSettings>) =>
    setDraft({ ...agent, ...patch });
  const updatePrefs = (patch: Partial<GlobalSettings["preferences"]>) =>
    setDraft({ ...app, preferences: { ...app.preferences, ...patch } });
  const updateSkills = (skills: Skill[]) => updateAgent({ skills });
  return (
    <div className="settings-layout live-settings-layout">
      <nav
        className="settings-navigation"
        aria-label={owner ? `${owner} settings sections` : "Settings sections"}
      >
        <span className="eyebrow">
          {owner ? `${owner} ONLY` : "YOUR WORKSPACE"}
        </span>
        {choices.map(({ id, label, icon }) => (
          <button
            key={id}
            type="button"
            className={section === id ? "selected" : ""}
            aria-current={section === id ? "page" : undefined}
            onClick={() => setSection(id)}
          >
            <Icon name={icon} />
            {label}
          </button>
        ))}
        <p>
          {owner ? "Private to this agent." : "Made for you."}
          <br />
          Kept under your control.
        </p>
      </nav>
      <form className="settings-content workspace-form" onSubmit={save}>
        <header className="settings-page-heading">
          <span className="settings-heading-icon">
            <Icon name={current.icon} />
          </span>
          <div>
            <h2>{current.label}</h2>
            <p>{current.description}</p>
          </div>
          {owner && (
            <button
              type="button"
              className="secondary-button agent-return-chat"
              disabled={busy}
              onClick={onChat}
            >
              <Icon name="chat" /> Open chat
            </button>
          )}
        </header>
        <fieldset disabled={busy} className="live-settings-fields">
          {!owner && section === "general" && (
            <div className="settings-stack">
              <section className="settings-card">
                <h3>Appearance</h3>
                <p>Choose a comfortable workspace for your day.</p>
                <div
                  className="appearance-choices"
                  role="group"
                  aria-label="Appearance"
                >
                  {(["light", "dark"] as const).map((value) => (
                    <button
                      key={value}
                      type="button"
                      className={`appearance-choice ${app.preferences.theme === value ? "selected" : ""}`}
                      aria-label={`${value === "light" ? "Light" : "Dark"} theme`}
                      aria-pressed={app.preferences.theme === value}
                      onClick={() => updatePrefs({ theme: value })}
                    >
                      <span
                        className={`appearance-preview ${value}`}
                        aria-hidden="true"
                      >
                        <i />
                        <span>
                          <i />
                          <i />
                          <i />
                        </span>
                      </span>
                      <span>
                        <Icon name={value === "light" ? "sun" : "moon"} />
                        {value === "light" ? "Light" : "Dark"}
                        {app.preferences.theme === value && (
                          <Icon name="check" />
                        )}
                      </span>
                    </button>
                  ))}
                </div>
              </section>
              <section className="settings-card settings-link-card">
                <div>
                  <h3>Your AI, your choice</h3>
                  <p>
                    Configure Claude, model profiles, output limits and budgets.
                  </p>
                </div>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={() => setSection("models")}
                >
                  Manage models <Icon name="chevron" />
                </button>
              </section>
              <section className="settings-card">
                <h3>Keyboard shortcuts</h3>
                <div className="settings-row">
                  <span>Open App settings</span>
                  <kbd>⌘ / Ctrl + ,</kbd>
                </div>
                <div className="settings-row">
                  <span>Close dialogs or mobile navigation</span>
                  <kbd>Esc</kbd>
                </div>
                <div className="settings-row">
                  <span>Move between controls</span>
                  <kbd>Tab / Shift + Tab</kbd>
                </div>
              </section>
              <p className="settings-footnote">
                Choose Save settings to apply your preferences across the
                workspace.
              </p>
            </div>
          )}
          {!owner && section === "connection" && (
            <div className="settings-stack">
              <section className="settings-card">
                <h3>Workspace connection</h3>
                <p>
                  {isLoopbackHost(location.hostname)
                    ? "Connected through your local development gateway."
                    : "Connected through the paired workspace gateway."}
                </p>
                <div className="settings-row">
                  <span>Gateway</span>
                  <code>{location.origin}</code>
                </div>
                <div className="settings-row">
                  <span>Claude credential</span>
                  <strong>
                    {status
                      ? status.credential_configured
                        ? "Configured"
                        : "Not configured"
                      : "Status unavailable"}
                  </strong>
                </div>
                <div className="settings-row">
                  <span>Credential source</span>
                  <strong>
                    {status?.credential_source === "environment"
                      ? "Local environment"
                      : status?.credential_source === "secure_store"
                        ? "Encrypted credential store"
                        : status?.credential_configured
                          ? "Server configuration"
                          : "Unavailable"}
                  </strong>
                </div>
                <p>
                  Provider credentials are managed under Claude API in Models.
                  Use the dedicated token field to replace the provider
                  credential.
                </p>
              </section>
              <section className="settings-card">
                <h3>You stay in control</h3>
                <p>
                  Janet and Harley keep separate workspaces and memory. Agent
                  settings cannot grant tool access or change these boundaries.
                </p>
                <p>
                  External writes require the reviewed approval and TOTP flow.
                </p>
                <button
                  type="button"
                  className="secondary-button"
                  onClick={onApprovals}
                >
                  Open approvals <Icon name="chevron" />
                </button>
              </section>
            </div>
          )}
          {!owner && section === "notifications" && (
            <section className="settings-card">
              <h3>Notifications</h3>
              <label>
                <input
                  type="checkbox"
                  checked={app.preferences.notifications}
                  onChange={(e) =>
                    updatePrefs({ notifications: e.target.checked })
                  }
                />
                Enable workspace notifications when delivery is connected
              </label>
              <p>
                Saved preference. Notification delivery and browser push
                integration remain unfinished.
              </p>
            </section>
          )}
          {!owner && section === "calendar" && (
            <section className="settings-card">
              <h3>Calendar preferences</h3>
              <label>
                Week starts
                <select
                  value={app.preferences.week_start}
                  onChange={(e) =>
                    updatePrefs({
                      week_start: e.target.value as "sunday" | "monday",
                    })
                  }
                >
                  <option value="sunday">Sunday</option>
                  <option value="monday">Monday</option>
                </select>
              </label>
              <label>
                Time format
                <select
                  value={app.preferences.time_format}
                  onChange={(e) =>
                    updatePrefs({ time_format: e.target.value as "12" | "24" })
                  }
                >
                  <option value="12">12-hour</option>
                  <option value="24">24-hour</option>
                </select>
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={app.preferences.show_completed}
                  onChange={(e) =>
                    updatePrefs({ show_completed: e.target.checked })
                  }
                />
                Show completed deadlines
              </label>
              <p>
                America/Edmonton · External calendar sync remains unavailable.
              </p>
            </section>
          )}
          {!owner && section === "models" && (
            <>
              <section className="settings-card">
                <h3>Claude API</h3>
                <p>
                  {status?.credential_configured
                    ? `Credential configured through ${status.credential_source}.`
                    : "Claude credential not configured. Set ANTHROPIC_API_KEY in your local environment."}
                </p>
                <ClaudeCredential token={token} />
                <p>
                  Credential presence does not confirm a successful provider
                  request.
                </p>
              </section>
              <section className="settings-card">
                <h3>Model profiles</h3>
                {(["fast", "normal", "deep"] as const).map((profile) => (
                  <div key={profile} className="live-model-profile">
                    <h4>{profile}</h4>
                    <label>
                      {profile} model
                      <select
                        value={app.models.profiles[profile].model}
                        onChange={(e) =>
                          setDraft({
                            ...app,
                            models: {
                              ...app.models,
                              profiles: {
                                ...app.models.profiles,
                                [profile]: {
                                  ...app.models.profiles[profile],
                                  provider: "anthropic",
                                  model: e.target.value,
                                },
                              },
                            },
                          })
                        }
                      >
                        {![...claudeModels].some(
                          (m) => m.id === app.models.profiles[profile].model,
                        ) && (
                          <option value={app.models.profiles[profile].model}>
                            {app.models.profiles[profile].model}
                          </option>
                        )}
                        {claudeModels.map((model) => (
                          <option key={model.id} value={model.id}>
                            {model.name}
                          </option>
                        ))}
                      </select>
                    </label>
                    {(
                      [
                        ["max_output_tokens", "Output token limit"],
                        [
                          "input_microusd_per_million",
                          "Input rate (microUSD per million tokens)",
                        ],
                        [
                          "output_microusd_per_million",
                          "Output rate (microUSD per million tokens)",
                        ],
                      ] as const
                    ).map(([key, label]) => (
                      <label key={key}>
                        {profile} {label}
                        <input
                          type="number"
                          required
                          min={key === "max_output_tokens" ? 1 : 0}
                          max={key === "max_output_tokens" ? 32000 : 1000000000}
                          step={1}
                          value={
                            Number.isNaN(app.models.profiles[profile][key])
                              ? ""
                              : app.models.profiles[profile][key]
                          }
                          onChange={(e) =>
                            setDraft({
                              ...app,
                              models: {
                                ...app.models,
                                profiles: {
                                  ...app.models.profiles,
                                  [profile]: {
                                    ...app.models.profiles[profile],
                                    [key]: e.target.valueAsNumber,
                                  },
                                },
                              },
                            })
                          }
                        />
                      </label>
                    ))}
                  </div>
                ))}
                <p>
                  Rates are configured accounting estimates, not verified
                  provider billing. Changing a model does not update rates
                  automatically.
                </p>
              </section>
              <section className="settings-card">
                <h3>Usage budgets</h3>
                {(
                  [
                    ["agent_daily_microusd", "Daily budget per agent"],
                    ["deep_daily_microusd", "Daily deep budget"],
                    ["task_microusd", "Budget per task"],
                  ] as const
                ).map(([key, label]) => (
                  <label key={key}>
                    {label} (USD)
                    <input
                      type="number"
                      required
                      min={0}
                      max={10000}
                      step="0.01"
                      value={
                        Number.isNaN(app.models[key])
                          ? ""
                          : app.models[key] / 1000000
                      }
                      onChange={(e) =>
                        setDraft({
                          ...app,
                          models: {
                            ...app.models,
                            [key]: Math.round(e.target.valueAsNumber * 1000000),
                          },
                        })
                      }
                    />
                  </label>
                ))}
                <p>
                  Zero blocks new usage; limits do not silently downgrade
                  reasoning.
                </p>
              </section>
            </>
          )}
          {owner && section === "general" && (
            <section className="settings-card">
              <h3>Identity & memory</h3>
              <label>
                Default reasoning profile
                <select
                  aria-label="Default reasoning profile"
                  value={agent.default_profile}
                  onChange={(e) =>
                    updateAgent({
                      default_profile: e.target
                        .value as AgentSettings["default_profile"],
                    })
                  }
                >
                  {["fast", "normal", "deep"].map((p) => (
                    <option key={p} value={p}>
                      {p}
                    </option>
                  ))}
                </select>
              </label>
              <label>
                soul.md
                <textarea
                  aria-label="soul.md"
                  rows={14}
                  maxLength={32768}
                  value={agent.soul}
                  onChange={(e) => updateAgent({ soul: e.target.value })}
                />
              </label>
              <label>
                memory.md
                <textarea
                  aria-label="memory.md"
                  rows={10}
                  maxLength={32768}
                  value={agent.memory_notes}
                  onChange={(e) =>
                    updateAgent({ memory_notes: e.target.value })
                  }
                />
              </label>
              <p>
                Private to {owner}. Saved notes guide subsequent turns; semantic
                memory keeps its separate provenance and confidence records. Do
                not enter credentials.
              </p>
            </section>
          )}
          {owner && section === "skills" && (
            <section className="settings-card">
              <h3>Skills</h3>
              <p>
                Saved skill instructions guide this agent. Files remain text;
                scripts are never executed automatically.
              </p>
              {!agent.skills.length && <p>No skills yet.</p>}
              {agent.skills.map((skill, index) => (
                <details key={skill.id}>
                  <summary>{skill.name}</summary>
                  <label>
                    Skill name
                    <input
                      required
                      maxLength={80}
                      aria-label="Skill name"
                      value={skill.name}
                      onChange={(e) =>
                        updateSkills(
                          agent.skills.map((s, i) =>
                            i === index ? { ...s, name: e.target.value } : s,
                          ),
                        )
                      }
                    />
                  </label>
                  <label>
                    Description
                    <input
                      maxLength={400}
                      value={skill.description}
                      onChange={(e) =>
                        updateSkills(
                          agent.skills.map((s, i) =>
                            i === index
                              ? { ...s, description: e.target.value }
                              : s,
                          ),
                        )
                      }
                    />
                  </label>
                  {skill.files.map((file, fileIndex) => (
                    <div key={fileIndex}>
                      <label>
                        File path
                        <input
                          required
                          maxLength={120}
                          value={file.path}
                          onChange={(e) =>
                            updateSkills(
                              agent.skills.map((s, i) =>
                                i === index
                                  ? {
                                      ...s,
                                      files: s.files.map((f, j) =>
                                        j === fileIndex
                                          ? { ...f, path: e.target.value }
                                          : f,
                                      ),
                                    }
                                  : s,
                              ),
                            )
                          }
                        />
                      </label>
                      <label>
                        {file.path}
                        <textarea
                          rows={10}
                          maxLength={65536}
                          aria-label={file.path}
                          value={file.content}
                          onChange={(e) =>
                            updateSkills(
                              agent.skills.map((s, i) =>
                                i === index
                                  ? {
                                      ...s,
                                      files: s.files.map((f, j) =>
                                        j === fileIndex
                                          ? { ...f, content: e.target.value }
                                          : f,
                                      ),
                                    }
                                  : s,
                              ),
                            )
                          }
                        />
                      </label>
                      <button
                        type="button"
                        className="secondary-button"
                        disabled={skill.files.length === 1}
                        onClick={() =>
                          updateSkills(
                            agent.skills.map((s, i) =>
                              i === index
                                ? {
                                    ...s,
                                    files: s.files.filter(
                                      (_, j) => j !== fileIndex,
                                    ),
                                  }
                                : s,
                            ),
                          )
                        }
                      >
                        Remove file
                      </button>
                    </div>
                  ))}
                  <button
                    type="button"
                    className="secondary-button"
                    disabled={skill.files.length >= 20}
                    onClick={() =>
                      updateSkills(
                        agent.skills.map((s, i) =>
                          i === index
                            ? {
                                ...s,
                                files: [
                                  ...s.files,
                                  {
                                    path: `notes-${s.files.length}.md`,
                                    content: "",
                                  },
                                ],
                              }
                            : s,
                        ),
                      )
                    }
                  >
                    Add file
                  </button>
                  <button
                    type="button"
                    className="secondary-button"
                    onClick={() =>
                      updateSkills(agent.skills.filter((_, i) => i !== index))
                    }
                  >
                    Remove skill
                  </button>
                </details>
              ))}
              <button
                type="button"
                className="secondary-button"
                disabled={agent.skills.length >= 16}
                onClick={() =>
                  updateSkills([
                    ...agent.skills,
                    {
                      id: crypto.randomUUID(),
                      name: "New skill",
                      description: "",
                      files: [
                        { path: "SKILL.md", content: "# Instructions\n" },
                      ],
                    },
                  ])
                }
              >
                Create skill
              </button>
            </section>
          )}
          {owner && section === "credentials" && (
            <section className="settings-card">
              <h3>{owner === "janet" ? "Janet" : "Harley"}’s credentials</h3>
              <p>
                Integration credentials belong to this agent’s secret scope.
                Credential references and enrollment are not yet connected to
                this page.
              </p>
              <div className="settings-row">
                <span>Credential enrollment</span>
                <strong>Not available yet</strong>
              </div>
              <p>
                Shared Claude configuration is in App settings → Claude &
                models. Development credentials stay in the local environment.
              </p>
              <p>
                Do not put keys, tokens or passwords in chat, identity notes or
                skill files.
              </p>
            </section>
          )}
          {owner && section === "access" && (
            <section className="settings-card">
              <h3>Access & boundaries</h3>
              <p>
                These settings grant no new tools, credentials, cross-agent
                access or external writes. Policy, Guard and TOTP remain
                enforced by the runtime.
              </p>
              <p>
                Integration credentials and Vault enrollment remain unavailable
                here.
              </p>
            </section>
          )}
        </fieldset>
        <div className="settings-save-bar live-settings-save-bar">
          <div>
            <strong>
              {dirty ? "Unsaved changes" : "Settings are up to date"}
            </strong>
            <p>
              {owner
                ? "Applies only to this agent."
                : "Applies to your workspace."}
            </p>
          </div>
          <div className="task-controls">
            <button className="primary-button" disabled={!dirty || busy}>
              {busy ? "Saving…" : "Save settings"}
            </button>
            <button
              type="button"
              className="secondary-button"
              disabled={busy}
              onClick={() => {
                if (dirty && !confirm("Discard unsaved settings and reload?"))
                  return;
                setError("");
                setMessage("");
                settings.retry();
              }}
            >
              Reload settings
            </button>
          </div>
        </div>
        {dirty && <p role="status">Unsaved changes</p>}
        {message && <p role="status">{message}</p>}
        {error && <p role="alert">{error}</p>}
      </form>
    </div>
  );
}
