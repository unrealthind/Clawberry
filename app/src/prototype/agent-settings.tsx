import { useState } from "react";
import { Avatar, Icon, type Agent } from "./shared";
import { CoreFiles } from "./core-files";
import { SkillsLibrary } from "./skills";
import { Vault } from "./vault";
const sections = [
  { id: "identity", name: "Identity & memory", icon: "soul" },
  { id: "skills", name: "Skills", icon: "skills" },
  { id: "credentials", name: "Credentials", icon: "lock" },
  { id: "access", name: "Access & boundaries", icon: "shield" },
] as const;
type Section = (typeof sections)[number]["id"];
export function AgentSettings({
  owner,
  search,
  onChat,
}: {
  owner: Agent;
  search: string;
  onChat: () => void;
}) {
  const [section, setSection] = useState<Section>("identity");
  const current = sections.find((item) => item.id === section)!;
  return (
    <div
      className={`settings-layout agent-settings-layout ${section === "skills" || section === "credentials" ? "agent-settings-wide" : ""}`}
    >
      <nav
        className="settings-navigation"
        aria-label={`${owner} settings sections`}
      >
        <div className="agent-setup-owner">
          <Avatar agent={owner} small />
          <span>
            <strong>{owner}</strong>
            <small>
              {owner === "Janet" ? "Work & business" : "Personal assistant"}
            </small>
          </span>
        </div>
        {sections.map((item) => (
          <button
            key={item.id}
            className={section === item.id ? "selected" : ""}
            aria-current={section === item.id ? "page" : undefined}
            onClick={() => setSection(item.id)}
          >
            <Icon name={item.icon} />
            {item.name}
          </button>
        ))}
        <p>
          Setup and configuration
          <br />
          for {owner} only.
        </p>
      </nav>
      <div className="settings-content">
        <header className="settings-page-heading">
          <span className="settings-heading-icon">
            <Icon name={current.icon} />
          </span>
          <div>
            <h2>{current.name}</h2>
            <p>{owner}’s setup, in one place.</p>
          </div>
          <button
            className="secondary-button agent-return-chat"
            onClick={onChat}
          >
            <Icon name="chat" />
            Open chat
          </button>
        </header>
        <div hidden={section !== "identity"} className="settings-stack">
          <section className="settings-card">
            <h3>{owner}’s identity & memory</h3>
            <p>
              Edit personality and guiding principles in soul.md, and selected
              memory notes in memory.md. These files belong to {owner} only.
            </p>
            <CoreFiles owner={owner} search={search} />
          </section>
          <p className="settings-footnote">
            Saved source files persist in this browser. They are UI previews,
            not changes to runtime identity, policy, or semantic memory. There
            is no separate personality form to keep in sync.
          </p>
        </div>
        <div
          hidden={section !== "skills"}
          role="region"
          aria-label={`${owner} skill settings`}
        >
          <SkillsLibrary owner={owner} search={search} />
        </div>
        <div
          hidden={section !== "credentials"}
          role="region"
          aria-label={`${owner} credential settings`}
        >
          <Vault owner={owner} search={search} />
        </div>
        <div hidden={section !== "access"} className="settings-stack">
          <section className="settings-card">
            <h3>{owner}’s domain</h3>
            <p>
              {owner === "Janet"
                ? "Business operations, projects, client work, and work-related investigations."
                : "Personal planning, travel, household tasks, errands, and personal projects."}
            </p>
            <div className="settings-row">
              <span>Private memory & workspace</span>
              <strong>{owner} only</strong>
            </div>
            <div className="settings-row">
              <span>
                Communication with {owner === "Janet" ? "Harley" : "Janet"}
              </span>
              <strong>Explicit approval required</strong>
            </div>
            <div className="settings-row">
              <span>Temporary workers</span>
              <strong>Scoped context & capabilities</strong>
            </div>
            <div className="settings-row">
              <span>External write actions</span>
              <strong>Reviewed n8n / TOTP path</strong>
            </div>
            <p>
              These describe the platform’s required boundaries. Live
              permissions are not inspected or changed by this prototype.
            </p>
          </section>
          <section className="settings-card">
            <h3>Shared AI connection</h3>
            <p>
              Claude API credentials and Fast/Normal/Deep model profiles are
              workspace-wide and stay in main Settings. They do not grant this
              agent new tools, credentials, or access to another agent.
            </p>
          </section>
        </div>
      </div>
    </div>
  );
}
