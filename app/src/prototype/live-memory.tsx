import { useEffect, useRef, useState, type ReactNode } from "react";
import { operate, type Agent, type Fact, type Memory } from "../api";
import { useLivePages } from "./live-data";

function MemoryDialog({
  title,
  busy,
  close,
  children,
}: {
  title: string;
  busy: boolean;
  close: () => void;
  children: ReactNode;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  useEffect(() => {
    const node = dialog.current;
    if (node && !node.open) node.showModal();
    return () => node?.close();
  }, []);
  return (
    <dialog
      ref={dialog}
      className="prototype-dialog"
      aria-label={title}
      onCancel={(event) => {
        if (busy) event.preventDefault();
        else close();
      }}
    >
      <h2>{title}</h2>
      {children}
    </dialog>
  );
}
export function LiveMemory({ token, owner }: { token: string; owner: Agent }) {
  const [query, setQuery] = useState(""),
    [source, setSource] = useState("all"),
    [minimum, setMinimum] = useState("0");
  const rows = useLivePages<Memory>(token, owner, "memory.page", {
    filter: {
      query,
      source_type: source === "all" ? null : source,
      minimum_confidence: Number(minimum),
    },
  });
  const shared = useLivePages<Memory>(token, owner, "shared.page");
  const [selected, setSelected] = useState<{
    memory: Memory;
    action:
      | "edit"
      | "delete"
      | "share"
      | "shared-edit"
      | "shared-verify"
      | "shared-stale";
  } | null>(null);
  const [error, setError] = useState(""),
    [busy, setBusy] = useState(false);
  async function manage(
    memory: Memory,
    operation: string,
    fields: object = {},
  ) {
    if (busy) return false;
    setBusy(true);
    setError("");
    try {
      await operate(token, owner, operation, { id: memory.id, ...fields });
      rows.refresh();
      shared.refresh();
      setSelected(null);
      return true;
    } catch {
      setError(
        "The change was not confirmed. Refresh memory before trying again; the server may already have saved it.",
      );
      return false;
    } finally {
      setBusy(false);
    }
  }
  return (
    <section className="live-management" aria-label="Memory workspace">
      <div className="live-section-heading">
        <div>
          <h2>Private memory</h2>
          <p>
            Private to {owner === "janet" ? "Janet" : "Harley"}. Facts retain
            their source and confidence; task logs stay separate.
          </p>
        </div>
        <button
          className="secondary-button"
          disabled={busy}
          onClick={() => {
            rows.refresh();
            shared.refresh();
            setError("");
          }}
        >
          Refresh memory
        </button>
      </div>
      <div className="workspace-form live-memory-filters">
        <label>
          Search memory
          <input
            placeholder="Search facts or sources"
            maxLength={1000}
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
        </label>
        <label>
          Filter by source
          <select value={source} onChange={(e) => setSource(e.target.value)}>
            {["all", "user", "file", "web", "api", "subagent"].map((value) => (
              <option key={value} value={value}>
                {value === "all" ? "All sources" : value}
              </option>
            ))}
          </select>
        </label>
        <label>
          Minimum confidence
          <select value={minimum} onChange={(e) => setMinimum(e.target.value)}>
            {["0", "0.5", "0.8", "0.95"].map((value) => (
              <option key={value} value={value}>
                {Math.round(Number(value) * 100)}%
              </option>
            ))}
          </select>
        </label>
      </div>
      {error && <p role="alert">{error}</p>}
      {rows.error && <p role="alert">{rows.error}</p>}
      {rows.loading && <p role="status">Loading private memory…</p>}
      {!rows.loading && !rows.error && rows.items.length === 0 && (
        <div className="live-empty">
          <h3>No matching memories</h3>
          <p>
            Reviewed facts will appear here. Try a different search or source
            filter.
          </p>
        </div>
      )}
      <div className="settings-stack">
        {rows.items.map((memory) => (
          <article className="settings-card memory-record" key={memory.id}>
            <div className="live-section-heading">
              <span className="live-status">{memory.category}</span>
              <small>Confidence {Math.round(memory.confidence * 100)}%</small>
            </div>
            <p className="live-exact-text">{memory.content}</p>
            <small>
              Source: {memory.source_type} · {memory.source_ref}
            </small>
            <p>
              <small>
                {memory.sensitivity} · {memory.visibility} ·{" "}
                {memory.stale
                  ? "Stale"
                  : memory.last_verified_at
                    ? `Verified ${memory.last_verified_at}`
                    : "Not yet verified"}
                {memory.expires_at && ` · Expires ${memory.expires_at}`}
              </small>
            </p>
            <div className="task-controls">
              {memory.sensitivity === "ordinary" && !memory.stale && (
                <button
                  className="secondary-button"
                  disabled={busy}
                  onClick={() => {
                    setError("");
                    setSelected({ memory, action: "share" });
                  }}
                >
                  Share
                </button>
              )}
              <button
                className="secondary-button"
                disabled={busy}
                onClick={() => {
                  setError("");
                  setSelected({ memory, action: "edit" });
                }}
              >
                Edit
              </button>
              <button
                className="secondary-button"
                disabled={busy}
                onClick={() => void manage(memory, "memory.verify")}
              >
                Verify
              </button>
              <button
                className="secondary-button"
                disabled={busy}
                onClick={() => void manage(memory, "memory.stale")}
              >
                Mark stale
              </button>
              <button
                className="secondary-button"
                disabled={busy}
                onClick={() => {
                  setError("");
                  setSelected({ memory, action: "delete" });
                }}
              >
                Delete
              </button>
            </div>
          </article>
        ))}
      </div>
      {rows.hasMore && (
        <button
          className="secondary-button"
          disabled={rows.loading}
          onClick={() => void rows.more()}
        >
          Load more memories
        </button>
      )}
      <h3>Explicitly shared facts</h3>
      <p>Only facts deliberately shared with both assistants appear here.</p>
      {shared.error && <p role="alert">{shared.error}</p>}
      {shared.loading && <p role="status">Loading shared facts…</p>}
      {!shared.loading && !shared.error && !shared.items.length && (
        <p>No explicitly shared facts.</p>
      )}
      <div className="settings-stack">
        {shared.items.map((memory) => (
          <article className="settings-card" key={memory.id}>
            <p className="live-exact-text">{memory.content}</p>
            <small>
              Shared by {memory.agent_owner} · {memory.source_type}:{" "}
              {memory.source_ref} · Confidence{" "}
              {Math.round(memory.confidence * 100)}%
            </small>
            <p>
              <small>
                {memory.stale
                  ? "Stale"
                  : memory.last_verified_at
                    ? `Verified ${memory.last_verified_at}`
                    : "Not yet verified"}
                {memory.expires_at && ` · Expires ${memory.expires_at}`}
              </small>
            </p>
            {memory.agent_owner === owner && (
              <div className="task-controls">
                {(
                  [
                    ["shared-edit", "Edit shared fact"],
                    ["shared-verify", "Verify shared fact"],
                    ["shared-stale", "Mark shared fact stale"],
                  ] as const
                ).map(([action, label]) => (
                  <button
                    key={action}
                    className="secondary-button"
                    disabled={busy}
                    onClick={() => {
                      setError("");
                      setSelected({ memory, action });
                    }}
                  >
                    {label}
                  </button>
                ))}
                <button
                  className="secondary-button"
                  disabled={busy}
                  onClick={() => void manage(memory, "memory.unshare")}
                >
                  Revoke shared copy
                </button>
              </div>
            )}
          </article>
        ))}
      </div>
      {shared.hasMore && (
        <button
          className="secondary-button"
          disabled={shared.loading}
          onClick={() => void shared.more()}
        >
          Load more shared facts
        </button>
      )}
      {(selected?.action === "edit" || selected?.action === "shared-edit") && (
        <MemoryEditor
          key={selected.memory.id}
          memory={selected.memory}
          busy={busy}
          error={error}
          close={() => setSelected(null)}
          shared={selected.action === "shared-edit"}
          save={(fact) =>
            manage(
              selected.memory,
              selected.action === "shared-edit" ? "shared.save" : "memory.save",
              {
                fact,
                ...(selected.action === "shared-edit"
                  ? { reviewed_hash: selected.memory.review_hash }
                  : {}),
              },
            )
          }
        />
      )}
      {selected &&
        selected.action !== "edit" &&
        selected.action !== "shared-edit" && (
          <MemoryDialog
            title={
              selected.action === "share"
                ? "Share this fact with Janet and Harley?"
                : selected.action === "shared-verify"
                  ? "Verify this shared fact?"
                  : selected.action === "shared-stale"
                    ? "Mark this shared fact stale?"
                    : "Delete this memory?"
            }
            busy={busy}
            close={() => setSelected(null)}
          >
            <p>
              {selected.action === "share"
                ? "This shares the exact fact and provenance shown below with both assistants. Their private memory databases stay separate. You can revoke the shared copy later."
                : selected.action === "shared-verify"
                  ? "Confirm you checked this exact shared fact against its source. Verification is recorded for the shared copy only."
                  : selected.action === "shared-stale"
                    ? "Mark this exact shared copy as needing review. The private source memory stays unchanged."
                    : "This removes the selected fact while retaining its source and task archive. Deletion is audited."}
            </p>
            <p className="live-exact-text">{selected.memory.content}</p>
            <small>
              {selected.memory.source_type}: {selected.memory.source_ref} ·
              Confidence {selected.memory.confidence} ·{" "}
              {selected.memory.category}
            </small>
            {error && <p role="alert">{error}</p>}
            <div className="task-controls">
              <button
                className="secondary-button"
                disabled={busy}
                onClick={() => setSelected(null)}
              >
                Cancel
              </button>
              <button
                className="primary-button"
                disabled={busy}
                onClick={() =>
                  void manage(
                    selected.memory,
                    selected.action === "share"
                      ? "memory.share"
                      : selected.action === "shared-verify"
                        ? "shared.verify"
                        : selected.action === "shared-stale"
                          ? "shared.stale"
                          : "memory.delete",
                    selected.action !== "delete"
                      ? { reviewed_hash: selected.memory.review_hash }
                      : {},
                  )
                }
              >
                {selected.action === "share"
                  ? "Share exact fact"
                  : selected.action === "shared-verify"
                    ? "Verify exact shared fact"
                    : selected.action === "shared-stale"
                      ? "Mark exact shared fact stale"
                      : "Delete memory"}
              </button>
            </div>
          </MemoryDialog>
        )}
    </section>
  );
}
function MemoryEditor({
  memory,
  busy,
  error,
  close,
  save,
  shared = false,
}: {
  memory: Memory;
  busy: boolean;
  error: string;
  close: () => void;
  save: (fact: Fact) => Promise<boolean>;
  shared?: boolean;
}) {
  const [content, setContent] = useState(memory.content),
    [confidence, setConfidence] = useState(String(memory.confidence)),
    [source, setSource] = useState(memory.source_ref),
    [expiry, setExpiry] = useState(memory.expires_at ?? "");
  return (
    <MemoryDialog
      title={shared ? "Edit shared fact" : "Edit memory"}
      busy={busy}
      close={close}
    >
      <p>
        Changing a fact clears its verification. Source and confidence remain
        explicit.
      </p>
      {shared && (
        <p>
          This updates the shared copy visible to both assistants. The private
          source memory stays unchanged.
        </p>
      )}
      <form
        className="workspace-form"
        onSubmit={(event) => {
          event.preventDefault();
          if (!busy)
            void save({
              content,
              confidence: Number(confidence),
              category: memory.category,
              source_type: memory.source_type,
              source_ref: source,
              sensitivity: memory.sensitivity,
              expires_at: expiry.trim() || null,
            });
        }}
      >
        <fieldset disabled={busy}>
          <label>
            Fact
            <textarea
              required
              maxLength={8000}
              value={content}
              onChange={(e) => setContent(e.target.value)}
            />
          </label>
          <label>
            Confidence (0–1)
            <input
              required
              type="number"
              min="0"
              max="1"
              step="0.01"
              value={confidence}
              onChange={(e) => setConfidence(e.target.value)}
            />
          </label>
          <label>
            Expiry (RFC 3339, optional)
            <input
              placeholder="2030-12-31T23:59:59Z"
              maxLength={64}
              value={expiry}
              onChange={(e) => setExpiry(e.target.value)}
            />
          </label>
          <p>
            Use an explicit time zone (Z or an offset). Leave blank for no
            expiry. Saving clears verification; review and verify the updated
            fact separately.
          </p>
          <label>
            Source reference
            <input
              required
              maxLength={2000}
              value={source}
              onChange={(e) => setSource(e.target.value)}
            />
          </label>
        </fieldset>
        {error && <p role="alert">{error}</p>}
        <div className="task-controls">
          <button
            type="button"
            className="secondary-button"
            disabled={busy}
            onClick={close}
          >
            Cancel
          </button>
          <button className="primary-button" disabled={busy}>
            Save changes
          </button>
        </div>
      </form>
    </MemoryDialog>
  );
}
