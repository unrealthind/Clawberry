import { useRef, useState } from "react";
import { Icon, type Agent } from "./shared";

type Injection = "Bearer token" | "API key header" | "HTTP Basic password";
type Credential = {
  id: string;
  reference: string;
  service: string;
  capability: string;
  destination: string;
  injection: Injection;
  paused: boolean;
};
const samples: Record<Agent, Credential[]> = {
  Janet: [
    {
      id: "invoice-read",
      reference: "invoice_ninja_read",
      service: "Invoice Ninja",
      capability: "invoices.read",
      destination: "https://invoices.example.test",
      injection: "Bearer token",
      paused: false,
    },
    {
      id: "n8n-read",
      reference: "n8n_read_gateway",
      service: "n8n read gateway",
      capability: "workflows.read",
      destination: "https://workflows.example.test",
      injection: "API key header",
      paused: false,
    },
    {
      id: "documents",
      reference: "document_service",
      service: "Document service",
      capability: "documents.read",
      destination: "https://documents.example.test",
      injection: "HTTP Basic password",
      paused: true,
    },
  ],
  Harley: [
    {
      id: "personal-calendar",
      reference: "personal_calendar_read",
      service: "Personal calendar",
      capability: "calendar.read",
      destination: "https://calendar.example.test",
      injection: "Bearer token",
      paused: false,
    },
    {
      id: "home-inventory",
      reference: "home_inventory",
      service: "Home inventory",
      capability: "inventory.read",
      destination: "https://home.example.test",
      injection: "HTTP Basic password",
      paused: false,
    },
  ],
};
export function Vault({ owner, search }: { owner: Agent; search: string }) {
  const [credentials, setCredentials] = useState(samples[owner]);
  const [selectedId, setSelectedId] = useState(samples[owner][0].id);
  const [name, setName] = useState("");
  const [service, setService] = useState("");
  const [capability, setCapability] = useState("");
  const [destination, setDestination] = useState("");
  const [injection, setInjection] = useState<Injection>("Bearer token");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [preview, setPreview] = useState(false);
  const dialog = useRef<HTMLDialogElement>(null);
  const selected = credentials.find((item) => item.id === selectedId)!;
  const visible = credentials.filter((item) =>
    `${item.reference} ${item.service} ${item.capability}`
      .toLowerCase()
      .includes(search.toLowerCase()),
  );
  function open() {
    setName("");
    setService("");
    setCapability("");
    setDestination("");
    setInjection("Bearer token");
    setError("");
    dialog.current?.showModal();
  }
  function close() {
    dialog.current?.close();
  }
  const header =
    selected.injection === "API key header"
      ? "X-API-Key: [injected by broker]"
      : selected.injection === "HTTP Basic password"
        ? "Authorization: Basic [injected by broker]"
        : "Authorization: Bearer [injected by broker]";
  return (
    <div className="vault-library">
      <div className="vault-preview-notice">
        <span className="vault-notice-icon">
          <Icon name="shield" />
        </span>
        <div>
          <strong>Vault design preview</strong>
          <p>
            Sample references only. Real secret entry is disabled in this
            prototype. No tokens or passwords are stored here.
          </p>
        </div>
        <span className="skill-local-tag">Not connected</span>
      </div>
      <div className="vault-toolbar">
        <span>
          {credentials.length} credential references <span>·</span> {owner} only
        </span>
        <button className="primary-button" onClick={open}>
          <Icon name="plus" />
          Add credential
        </button>
      </div>
      {notice && (
        <p className="vault-status" role="status">
          {notice}
        </p>
      )}
      <div className="vault-layout">
        <nav
          className="vault-credential-list"
          aria-label={`${owner} credential references`}
        >
          {visible.map((item) => (
            <button
              key={item.id}
              className={item.id === selectedId ? "selected-credential" : ""}
              onClick={() => {
                setSelectedId(item.id);
                setPreview(false);
                setNotice("");
              }}
            >
              <span className="vault-key-icon">
                <Icon
                  name={
                    item.injection === "HTTP Basic password" ? "lock" : "key"
                  }
                />
              </span>
              <span>
                <strong>{item.service}</strong>
                <small>{item.reference}</small>
              </span>
              <span
                className={`vault-scope-dot ${item.paused ? "paused" : ""}`}
                aria-label={
                  item.paused
                    ? "Sample access paused"
                    : "Sample access configured"
                }
              />
            </button>
          ))}
          {!visible.length && (
            <p className="skills-empty">No matching credential references.</p>
          )}
        </nav>
        <section className="vault-detail" aria-label="Credential details">
          <div className="vault-detail-heading">
            <span className="vault-key-icon">
              <Icon
                name={
                  selected.injection === "HTTP Basic password" ? "lock" : "key"
                }
              />
            </span>
            <div>
              <span className="eyebrow">
                {selected.injection === "HTTP Basic password"
                  ? "PASSWORD REFERENCE"
                  : "TOKEN REFERENCE"}
              </span>
              <h3>{selected.service}</h3>
            </div>
            <span
              className={`vault-state-tag ${selected.paused ? "paused" : ""}`}
            >
              {selected.paused ? "Paused · sample" : "Configured · sample"}
            </span>
          </div>
          <div className="vault-masked-secret">
            <div>
              <span>Secret value</span>
              <strong aria-label="Secret value is never displayed">
                •••• •••• •••• ••••
              </strong>
            </div>
            <span>
              <Icon name="lock" />
              Broker only
            </span>
          </div>
          <dl className="vault-properties">
            <div>
              <dt>Credential reference</dt>
              <dd>
                <code>{selected.reference}</code>
              </dd>
            </div>
            <div>
              <dt>Allowed agent</dt>
              <dd>
                {owner}
                <span className="vault-private-label">Private scope</span>
              </dd>
            </div>
            <div>
              <dt>Capability</dt>
              <dd>
                <code>{selected.capability}</code>
              </dd>
            </div>
            <div>
              <dt>Allowed destination</dt>
              <dd>
                <code>{selected.destination}</code>
              </dd>
            </div>
            <div>
              <dt>Middleware injection</dt>
              <dd>
                {selected.injection === "API key header"
                  ? "X-API-Key header"
                  : selected.injection === "HTTP Basic password"
                    ? "HTTP Basic authorization"
                    : "Authorization · Bearer"}
              </dd>
            </div>
          </dl>
          <div className="vault-injection-note">
            <Icon name="shield" />
            <p>
              {owner} requests the named credential. Middleware checks the
              agent, capability, and destination, then injects the secret into
              the outgoing request. Only a redacted result returns to the agent.
            </p>
          </div>
          <div className="vault-detail-actions">
            <button
              className="secondary-button"
              onClick={() => setPreview(!preview)}
              aria-expanded={preview}
            >
              {preview ? "Hide request preview" : "Preview injection"}
              <Icon name="chevron" />
            </button>
            <button
              className="secondary-button"
              onClick={() => {
                setCredentials((old) =>
                  old.map((item) =>
                    item.id === selected.id
                      ? { ...item, paused: !item.paused }
                      : item,
                  ),
                );
                setNotice(
                  `Sample access ${selected.paused ? "resumed" : "paused"} for ${selected.reference}. No live permission changed.`,
                );
              }}
            >
              {selected.paused ? "Resume sample access" : "Pause sample access"}
            </button>
          </div>
          {preview && (
            <div className="vault-request-preview">
              <div>
                <span className="eyebrow">
                  ILLUSTRATIVE REQUEST · NO NETWORK CALL
                </span>
              </div>
              <pre>{`Agent: ${owner}\nCredential: ${selected.reference}\nCapability: ${selected.capability}\nDestination: ${selected.destination}\n${header}\nStatus: ${selected.paused ? "DENIED — sample access paused" : "PREVIEW ONLY — secure broker not connected"}`}</pre>
            </div>
          )}
        </section>
      </div>
      <section className="vault-flow" aria-label="Credential injection flow">
        <div>
          <span>1</span>
          <strong>Agent requests</strong>
          <small>Named credential reference</small>
        </div>
        <Icon name="chevron" />
        <div>
          <span>2</span>
          <strong>Middleware authorizes</strong>
          <small>Agent, capability, destination</small>
        </div>
        <Icon name="chevron" />
        <div>
          <span>3</span>
          <strong>Broker injects</strong>
          <small>Secret stays outside agent context</small>
        </div>
      </section>
      <p className="vault-write-note">
        <Icon name="shield" />A credential does not authorize an external write.
        Review and TOTP still apply.
      </p>
      <dialog
        ref={dialog}
        className="prototype-dialog"
        aria-labelledby={`vault-dialog-${owner}`}
        onCancel={close}
        onClick={(event) => {
          if (event.target === event.currentTarget) close();
        }}
      >
        <div className="dialog-top">
          <span className="eyebrow">
            {owner.toUpperCase()}’S VAULT · PREVIEW
          </span>
          <button
            className="icon-button"
            aria-label="Close credential dialog"
            onClick={close}
          >
            <Icon name="close" />
          </button>
        </div>
        <h2 id={`vault-dialog-${owner}`}>Add a sample credential</h2>
        <p className="dialog-description">
          Define how middleware should use a credential. Real secret entry
          becomes available through the connected secure broker.
        </p>
        <form
          onSubmit={(event) => {
            event.preventDefault();
            const reference = name.trim();
            if (
              !/^[a-z][a-z0-9_]{2,63}$/.test(reference) ||
              credentials.some((item) => item.reference === reference)
            ) {
              setError(
                "Use a unique reference: 3–64 lowercase letters, numbers, or underscores, starting with a letter.",
              );
              return;
            }
            if (
              !service.trim() ||
              !/^[a-z][a-z0-9_.-]{1,79}$/.test(capability.trim())
            ) {
              setError(
                "Enter a service name and a capability such as projects.read.",
              );
              return;
            }
            let target: URL;
            try {
              target = new URL(destination);
              if (
                target.protocol !== "https:" ||
                target.username ||
                target.password ||
                target.search ||
                target.hash ||
                target.pathname !== "/"
              )
                throw new Error();
            } catch {
              setError(
                "Use an HTTPS origin, such as https://service.example.test, without credentials, paths, or query parameters.",
              );
              return;
            }
            if (credentials.length >= 20) {
              setError(
                "This prototype supports up to 20 sample references per agent.",
              );
              return;
            }
            const item: Credential = {
              id: crypto.randomUUID(),
              reference,
              service: service.trim(),
              capability: capability.trim(),
              destination: target.origin,
              injection,
              paused: false,
            };
            setCredentials((old) => [...old, item]);
            setSelectedId(item.id);
            setPreview(false);
            setNotice(
              "Sample credential reference added. No secret was collected or stored.",
            );
            close();
          }}
        >
          <label
            className="field-label"
            htmlFor={`credential-service-${owner}`}
          >
            Service name
          </label>
          <input
            id={`credential-service-${owner}`}
            className="thread-name-input"
            value={service}
            onChange={(event) => setService(event.target.value)}
            placeholder="For example, Project system"
            maxLength={60}
            required
          />
          <label className="field-label" htmlFor={`credential-name-${owner}`}>
            Credential reference
          </label>
          <input
            id={`credential-name-${owner}`}
            className="thread-name-input"
            value={name}
            onChange={(event) => setName(event.target.value)}
            placeholder="project_system_read"
            maxLength={64}
            autoComplete="off"
            required
          />
          <div className="vault-form-grid">
            <div>
              <label
                className="field-label"
                htmlFor={`credential-injection-${owner}`}
              >
                Credential type
              </label>
              <select
                id={`credential-injection-${owner}`}
                className="thread-name-input"
                value={injection}
                onChange={(event) =>
                  setInjection(event.target.value as Injection)
                }
              >
                <option>Bearer token</option>
                <option>API key header</option>
                <option>HTTP Basic password</option>
              </select>
            </div>
            <div>
              <label
                className="field-label"
                htmlFor={`credential-capability-${owner}`}
              >
                Allowed capability
              </label>
              <input
                id={`credential-capability-${owner}`}
                className="thread-name-input"
                value={capability}
                onChange={(event) => setCapability(event.target.value)}
                placeholder="projects.read"
                maxLength={80}
                required
              />
            </div>
          </div>
          <label className="field-label" htmlFor={`credential-target-${owner}`}>
            Allowed destination
          </label>
          <input
            id={`credential-target-${owner}`}
            className="thread-name-input"
            value={destination}
            onChange={(event) => setDestination(event.target.value)}
            placeholder="https://projects.example.test"
            maxLength={200}
            required
          />
          <label className="field-label" htmlFor={`credential-secret-${owner}`}>
            Secret value
          </label>
          <input
            id={`credential-secret-${owner}`}
            className="thread-name-input"
            type="password"
            disabled
            placeholder="Secure entry disabled in prototype"
            autoComplete="off"
          />
          <p className="sample-disclaimer">
            Do not enter real secrets here. Sample references reset on reload;
            no secret storage or injection is performed.
          </p>
          {error && (
            <p className="skills-error" role="alert">
              {error}
            </p>
          )}
          <div className="dialog-actions">
            <button type="button" className="secondary-button" onClick={close}>
              Cancel
            </button>
            <button className="primary-button">
              Create sample reference
              <Icon name="plus" />
            </button>
          </div>
        </form>
      </dialog>
    </div>
  );
}
