import { useState } from "react";
import { Icon } from "./shared";
import {
  claudeModels,
  profiles,
  readModelDraft,
  saveModelDraft,
  validateModelDraft,
  type ModelDraft,
  type ModelId,
} from "./model-settings";
export type Preferences = {
  notifications: boolean;
  weekStart: "sunday" | "monday";
  showCompleted: boolean;
  timeFormat: "12" | "24";
};
export const defaultPreferences: Preferences = {
  notifications: true,
  weekStart: "sunday",
  showCompleted: true,
  timeFormat: "12",
};
const sections = [
  { id: "general", name: "General", icon: "settings" },
  { id: "models", name: "Claude & models", icon: "soul" },
  { id: "notifications", name: "Notifications", icon: "activity" },
  { id: "calendar", name: "Calendar", icon: "calendar" },
  { id: "connection", name: "Connection & security", icon: "shield" },
] as const;
type Section = (typeof sections)[number]["id"];
const profileDetails = {
  fast: {
    title: "Fast",
    description: "Extraction, classification, and lightweight workers",
    icon: "layers",
  },
  normal: {
    title: "Normal",
    description: "Janet, Harley, conversations, and everyday work",
    icon: "chat",
  },
  deep: {
    title: "Deep",
    description: "Complex planning, difficult debugging, and reviews",
    icon: "memory",
  },
} as const;
export function WorkspaceSettings({
  preferences,
  onChange,
  theme,
  onTheme,
  onConnect,
}: {
  preferences: Preferences;
  onChange: (value: Preferences) => void;
  theme: "light" | "dark";
  onTheme: (value: "light" | "dark") => void;
  onConnect: () => void;
}) {
  const [section, setSection] = useState<Section>("general");
  const [loaded] = useState(readModelDraft);
  const [saved, setSaved] = useState<ModelDraft>(loaded.draft);
  const [draft, setDraft] = useState<ModelDraft>(loaded.draft);
  const [status, setStatus] = useState("");
  const [error, setError] = useState("");
  const dirty = JSON.stringify(saved) !== JSON.stringify(draft);
  function startConnection() {
    if (dirty) {
      setSection("models");
      setError(
        "Save or discard your model draft before opening a connected workspace.",
      );
      return;
    }
    onConnect();
  }
  const current = sections.find((item) => item.id === section)!;
  return (
    <div className="settings-layout">
      <nav className="settings-navigation" aria-label="Settings sections">
        <span className="eyebrow">YOUR WORKSPACE</span>
        {sections.map((item) => (
          <button
            key={item.id}
            className={section === item.id ? "selected" : ""}
            aria-current={section === item.id ? "page" : undefined}
            onClick={() => setSection(item.id)}
          >
            <Icon name={item.icon} />
            {item.name}
            {item.id === "models" && dirty && (
              <span
                className="settings-unsaved-dot"
                aria-hidden="true"
                title="Unsaved model changes"
              />
            )}
          </button>
        ))}
        <p>
          Made for you.
          <br />
          Kept under your control.
        </p>
      </nav>
      <div className="settings-content">
        <header className="settings-page-heading">
          <span className="settings-heading-icon">
            <Icon name={current.icon} />
          </span>
          <div>
            <h2>{current.name}</h2>
            <p>
              {section === "models"
                ? "Choose the models behind your assistants."
                : "A workspace that works the way you do."}
            </p>
          </div>
        </header>
        {section === "general" && (
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
                    className={`appearance-choice ${theme === value ? "selected" : ""}`}
                    aria-pressed={theme === value}
                    onClick={() => {
                      onTheme(value);
                      const url = new URL(location.href);
                      url.searchParams.set("theme", value);
                      history.replaceState(null, "", url);
                    }}
                  >
                    <span className={`appearance-preview ${value}`}>
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
                      {theme === value && <Icon name="check" />}
                    </span>
                  </button>
                ))}
              </div>
            </section>
            <section className="settings-card settings-link-card">
              <div>
                <h3>Your AI, your choice</h3>
                <p>
                  Configure Claude, model profiles, output limits, and budgets.
                </p>
              </div>
              <button
                className="secondary-button"
                onClick={() => setSection("models")}
              >
                Manage models <Icon name="chevron" />
              </button>
            </section>
            <section className="settings-card">
              <h3>Keyboard shortcuts</h3>
              <div className="settings-row">
                <span>Open Settings</span>
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
              Appearance and workspace preferences apply immediately in this
              tab. Model drafts save separately in this browser.
            </p>
          </div>
        )}
        {section === "models" && (
          <form
            className="settings-stack"
            onSubmit={(e) => {
              e.preventDefault();
              setError("");
              setStatus("");
              try {
                if (loaded.error) return;
                saveModelDraft(draft);
                setSaved(structuredClone(draft));
                setStatus(
                  "Model draft saved in this browser. Runtime models have not changed.",
                );
              } catch {
                setError(
                  "Could not save. Check model choices, output limits, and budgets, or allow browser storage. Your edits are still here.",
                );
              }
            }}
          >
            <section className="settings-card claude-connection">
              <div className="claude-connection-heading">
                <span className="settings-claude-mark">C</span>
                <div>
                  <h3>Claude API</h3>
                  <p>Anthropic · Direct API</p>
                </div>
                <span className="settings-badge">Not verified</span>
              </div>
              <div className="settings-row">
                <span>Connection</span>
                <strong>Gateway not connected</strong>
              </div>
              <div className="settings-row">
                <span>API endpoint</span>
                <code>api.anthropic.com</code>
              </div>
              <div className="workspace-form">
                <label>
                  API key
                  <input
                    type="password"
                    disabled
                    placeholder="Managed by the secure backend"
                    aria-describedby="api-key-help"
                  />
                </label>
              </div>
              <p id="api-key-help">
                API-key enrollment is unavailable until the secure backend is
                connected. Don’t paste credentials into chat or personality
                notes.
              </p>
              <div className="task-controls">
                <button
                  type="button"
                  className="secondary-button"
                  onClick={startConnection}
                >
                  Connect gateway
                </button>
                <a
                  href="https://platform.claude.com/docs/en/models/overview"
                  target="_blank"
                  rel="noreferrer"
                  className="settings-text-link"
                >
                  Claude model reference <Icon name="chevron" />
                </a>
              </div>
            </section>
            <section className="settings-card">
              <div className="settings-section-title">
                <div>
                  <h3>Model profiles</h3>
                  <p>
                    Agents ask for an effort profile. You choose the model
                    behind it.
                  </p>
                </div>
                <span className="settings-badge">Local draft</span>
              </div>
              <div className="model-profile-list">
                {profiles.map((profile) => (
                  <div className="model-profile" key={profile}>
                    <div className="model-profile-title">
                      <span>
                        <Icon name={profileDetails[profile].icon} />
                      </span>
                      <div>
                        <h4>{profileDetails[profile].title}</h4>
                        <p>{profileDetails[profile].description}</p>
                      </div>
                    </div>
                    <div className="model-profile-fields workspace-form">
                      <label>
                        {profileDetails[profile].title} model
                        <select
                          aria-label={`${profileDetails[profile].title} model`}
                          aria-describedby={`model-reference-${profile}`}
                          value={draft.routes[profile].model}
                          onChange={(e) => {
                            setStatus("");
                            setDraft({
                              ...draft,
                              routes: {
                                ...draft.routes,
                                [profile]: {
                                  ...draft.routes[profile],
                                  model: e.target.value as ModelId,
                                },
                              },
                            });
                          }}
                        >
                          {claudeModels.map((model) => (
                            <option value={model.id} key={model.id}>
                              {model.name}
                            </option>
                          ))}
                        </select>
                        <small id={`model-reference-${profile}`}>
                          {draft.routes[profile].model}
                        </small>
                      </label>
                      <label>
                        {profileDetails[profile].title} output token limit
                        <input
                          type="number"
                          min={1}
                          max={32000}
                          step={1}
                          required
                          value={
                            Number.isNaN(draft.routes[profile].maxOutputTokens)
                              ? ""
                              : draft.routes[profile].maxOutputTokens
                          }
                          onChange={(e) => {
                            setStatus("");
                            setDraft({
                              ...draft,
                              routes: {
                                ...draft.routes,
                                [profile]: {
                                  ...draft.routes[profile],
                                  maxOutputTokens: e.target.valueAsNumber,
                                },
                              },
                            });
                          }}
                        />
                      </label>
                    </div>
                  </div>
                ))}
              </div>
              <p className="settings-footnote">
                Defaults follow this repository’s routing configuration. Model
                choices come from Anthropic’s reference; availability in your
                account has not been checked. Runtime output limits are capped
                at 32,000.
              </p>
            </section>
            <section className="settings-card workspace-form">
              <h3>Usage budgets</h3>
              <p>
                Draft limits in US dollars. A zero budget blocks new usage.
                Reaching a limit should surface a decision, without silently
                downgrading reasoning.
              </p>
              <div className="settings-budget-grid">
                {(
                  [
                    { key: "agentDaily", title: "Daily budget per agent" },
                    { key: "deepDaily", title: "Daily deep-reasoning budget" },
                    { key: "task", title: "Budget per task" },
                  ] as const
                ).map((item) => (
                  <label key={item.key}>
                    {item.title}
                    <div className="settings-money-input">
                      <span>US$</span>
                      <input
                        aria-label={item.title}
                        type="number"
                        min={0}
                        max={10000}
                        step="0.01"
                        required
                        value={
                          Number.isNaN(draft.budgets[item.key])
                            ? ""
                            : draft.budgets[item.key]
                        }
                        onChange={(e) => {
                          setStatus("");
                          setDraft({
                            ...draft,
                            budgets: {
                              ...draft.budgets,
                              [item.key]: e.target.valueAsNumber,
                            },
                          });
                        }}
                      />
                    </div>
                  </label>
                ))}
              </div>
            </section>
            {dirty && !validateModelDraft(draft) && (
              <p className="settings-feedback error" role="alert">
                Use whole output limits from 1 to 32,000 tokens, and budgets
                from US$0 to US$10,000 with at most two decimal places.
              </p>
            )}
            {loaded.error && (
              <p className="settings-feedback error" role="alert">
                {loaded.error}
              </p>
            )}
            {error && (
              <p className="settings-feedback error" role="alert">
                {error}
              </p>
            )}
            {status && (
              <p className="settings-feedback" role="status">
                {status}
              </p>
            )}
            <footer className="settings-save-bar">
              <div>
                <strong>
                  {dirty ? "Unsaved model changes" : "Model draft"}
                </strong>
                <p>Saving here does not change the live runtime.</p>
              </div>
              <div className="task-controls">
                <button
                  type="button"
                  className="secondary-button"
                  disabled={!dirty}
                  onClick={() => {
                    setDraft(structuredClone(saved));
                    setStatus("");
                    setError("");
                  }}
                >
                  Discard changes
                </button>
                <button
                  className="primary-button"
                  disabled={
                    !!loaded.error || !dirty || !validateModelDraft(draft)
                  }
                >
                  Save model draft
                </button>
              </div>
            </footer>
          </form>
        )}
        {section === "notifications" && (
          <section className="settings-card workspace-form">
            <h3>Workspace notifications</h3>
            <label className="settings-toggle-row">
              <span>
                <strong>Optional updates</strong>
                <small>Show confirmations and sample activity toasts.</small>
              </span>
              <input
                type="checkbox"
                checked={preferences.notifications}
                onChange={(e) =>
                  onChange({ ...preferences, notifications: e.target.checked })
                }
                aria-label="Show optional workspace notifications"
              />
            </label>
            <p>
              Errors and security notices stay visible. Browser push
              notifications, runtime alerts, and quiet hours require backend
              integration.
            </p>
          </section>
        )}
        {section === "calendar" && (
          <div className="settings-stack">
            <section className="settings-card workspace-form">
              <h3>Calendar preferences</h3>
              <label>
                Week starts on
                <select
                  value={preferences.weekStart}
                  onChange={(e) =>
                    onChange({
                      ...preferences,
                      weekStart: e.target.value as Preferences["weekStart"],
                    })
                  }
                >
                  <option value="sunday">Sunday</option>
                  <option value="monday">Monday</option>
                </select>
              </label>
              <label>
                Calendar time format
                <select
                  value={preferences.timeFormat}
                  onChange={(e) =>
                    onChange({
                      ...preferences,
                      timeFormat: e.target.value as Preferences["timeFormat"],
                    })
                  }
                >
                  <option value="12">12 hour</option>
                  <option value="24">24 hour</option>
                </select>
              </label>
              <label className="settings-toggle-row">
                <span>Show completed task deadlines</span>
                <input
                  type="checkbox"
                  checked={preferences.showCompleted}
                  onChange={(e) =>
                    onChange({
                      ...preferences,
                      showCompleted: e.target.checked,
                    })
                  }
                />
              </label>
              <div className="settings-row">
                <span>Time zone</span>
                <strong>America/Edmonton</strong>
              </div>
            </section>
            <section className="settings-card">
              <h3>Calendar connection</h3>
              <p>
                Task deadlines appear in the workspace calendar. External
                calendar synchronization and invitations need their reviewed
                integration.
              </p>
              <span className="settings-badge">External sync unavailable</span>
            </section>
          </div>
        )}
        {section === "connection" && (
          <div className="settings-stack">
            <section className="settings-card">
              <h3>Gateway connection</h3>
              <p>
                Connect your local gateway for real chat, tasks, and read-only
                activity. Local development uses --local-ui with no token entry.
                Remote connections still require pairing.
              </p>
              <button className="primary-button" onClick={startConnection}>
                Connect gateway
              </button>
            </section>
            <section className="settings-card">
              <h3>Credentials & approvals</h3>
              <div className="settings-row">
                <span>Credential enrollment</span>
                <strong>Requires secure backend</strong>
              </div>
              <div className="settings-row">
                <span>External writes</span>
                <strong>n8n + TOTP</strong>
              </div>
              <div className="settings-row">
                <span>Agent boundaries</span>
                <strong>Approval required</strong>
              </div>
              <p>
                These protections are enforced by the runtime. Model settings
                and draft budgets do not grant tool access or approval
                authority.
              </p>
            </section>
          </div>
        )}
      </div>
    </div>
  );
}
