import { FilePreview, canPreview } from "./file-preview";
import { useEffect, useRef, useState } from "react";
import {
  withRequestDeadline,
  operate,
  type Agent,
  type Attachment,
  type Task,
} from "../api";

type Decision = { id: string; content: string; created_at: string };
type Worker = {
  id: string;
  task_id: string;
  spec: { role: string; objective: string };
  status: string;
  error: string | null;
  archive_path: string | null;
  result: null | {
    summary: string;
    findings: string[];
    confidence: number;
    sources: { source_ref: string; source_type: string }[];
    artifacts: { filename: string }[];
    errors: string[];
  };
};
function download(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
export function ThreadWorkspace({
  token,
  owner,
  thread,
  tasks,
  manageTasks,
  summary,
  refresh,
  filesOnly = false,
  onSummarySaved,
  onIncludeText,
}: {
  token: string;
  owner: Agent;
  thread: string;
  tasks: Task[];
  manageTasks: () => void;
  summary: string;
  refresh: number;
  filesOnly?: boolean;
  onSummarySaved?: () => void;
  onIncludeText?: (text: string) => void;
}) {
  const [preview, setPreview] = useState<{
    id: string;
    owner: Agent;
    thread: string;
  } | null>(null);
  const [uploadUncertain, setUploadUncertain] = useState(false);
  const [summaryDraft, setSummaryDraft] = useState(summary);
  const [editingSummary, setEditingSummary] = useState(false);
  const [files, setFiles] = useState<Attachment[]>([]);
  const [decisions, setDecisions] = useState<Decision[]>([]);
  const [workers, setWorkers] = useState<Worker[]>([]);
  const [draft, setDraft] = useState("");
  const [pending, setPending] = useState<{
    id: string;
    content: string;
  } | null>(null);
  const [busy, setBusy] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [reload, setReload] = useState(0);
  const active = useRef<AbortController | null>(null);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      active.current?.abort();
    };
  }, []);
  useEffect(() => {
    const controller = new AbortController();
    setLoading(true);
    setError("");
    Promise.allSettled([
      operate<Attachment[]>(
        token,
        owner,
        "file.list",
        { thread_id: thread },
        controller.signal,
      ),
      operate<Decision[]>(
        token,
        owner,
        "thread.decisions",
        { thread_id: thread },
        controller.signal,
      ),
      operate<Worker[]>(token, owner, "agent.list", {}, controller.signal),
    ]).then(([f, d, w]) => {
      if (controller.signal.aborted) return;
      if (f.status === "fulfilled") {
        setFiles(f.value);
        setUploadUncertain(false);
      }
      if (d.status === "fulfilled") setDecisions(d.value);
      if (w.status === "fulfilled") setWorkers(w.value);
      if ([f, d, w].some((result) => result.status === "rejected"))
        setError("Some thread evidence could not be loaded. Refresh to retry.");
      setLoading(false);
    });
    return () => controller.abort();
  }, [token, owner, thread, reload, refresh]);
  async function run(
    action: (signal: AbortSignal) => Promise<void>,
    failure: string,
  ) {
    if (busy) return;
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setError("");
    try {
      await withRequestDeadline(controller.signal, action);
    } catch {
      if (mounted.current) setError(failure);
    } finally {
      if (mounted.current) setBusy(false);
    }
  }
  const linked = tasks.filter((task) => task.thread_id === thread);
  const linkedWorkers = workers.filter((worker) =>
    linked.some((task) => task.id === worker.task_id),
  );
  return (
    <section
      className="live-thread-context"
      aria-label="Focus Thread workspace"
    >
      <div className="task-controls">
        <h2>{filesOnly ? "Thread files" : "Thread workspace"}</h2>
      </div>
      <p>{summary || "No summary recorded yet."}</p>
      {!filesOnly &&
        onSummarySaved &&
        (editingSummary ? (
          <form
            className="workspace-form"
            onSubmit={(event) => {
              event.preventDefault();
              if (new TextEncoder().encode(summaryDraft).length > 2000) {
                setError("Summary must fit within 2,000 bytes.");
                return;
              }
              void run(async (signal) => {
                await operate(
                  token,
                  owner,
                  "thread.summary",
                  { thread_id: thread, summary: summaryDraft },
                  signal,
                );
                if (mounted.current) {
                  setEditingSummary(false);
                  onSummarySaved();
                }
              }, "Summary save was not confirmed. Refresh evidence before trying again.");
            }}
          >
            <label htmlFor="thread-summary">
              Thread summary
              <textarea
                id="thread-summary"
                required
                maxLength={2000}
                value={summaryDraft}
                disabled={busy}
                onChange={(e) => setSummaryDraft(e.target.value)}
              />
            </label>
            <div className="task-controls">
              <button
                className="primary-button"
                disabled={busy || !summaryDraft.trim()}
              >
                Save summary
              </button>
              <button
                type="button"
                className="secondary-button"
                disabled={busy}
                onClick={() => setEditingSummary(false)}
              >
                Cancel
              </button>
            </div>
          </form>
        ) : (
          <button
            className="secondary-button"
            disabled={busy}
            onClick={() => {
              setSummaryDraft(summary);
              setEditingSummary(true);
            }}
          >
            Edit thread summary
          </button>
        ))}
      <button
        className="secondary-button"
        disabled={busy || loading}
        onClick={() => setReload((n) => n + 1)}
      >
        Refresh evidence
      </button>
      {loading && <p role="status">Loading thread evidence…</p>}
      {error && (
        <div role="alert">
          <p>{error}</p>
          <button
            className="secondary-button"
            disabled={busy || loading}
            onClick={() => setReload((n) => n + 1)}
          >
            Retry loading
          </button>
        </div>
      )}
      <div className="task-grid">
        {!filesOnly && (
          <article className="task-card">
            <h3>Tasks</h3>
            {linked.map((task) => (
              <p key={task.id}>
                {task.title} · {task.status}
              </p>
            ))}
            {!linked.length && <p>No linked tasks yet.</p>}
            <button className="secondary-button" onClick={manageTasks}>
              Manage thread tasks
            </button>
          </article>
        )}
        <article className="task-card">
          <h3>Files</h3>
          <label>
            Upload a file (up to 4 MB)
            <input
              type="file"
              disabled={busy || loading || uploadUncertain}
              onChange={(e) => {
                const file = e.currentTarget.files?.[0];
                e.currentTarget.value = "";
                if (!file) return;
                if (!file.size || file.size > 4_000_000) {
                  setError("Choose a nonempty file up to 4 MB.");
                  return;
                }
                setUploadUncertain(true);
                void run(async (signal) => {
                  const response = await fetch(
                    `/api/agents/${owner}/threads/${encodeURIComponent(thread)}/uploads?filename=${encodeURIComponent(file.name)}`,
                    {
                      method: "POST",
                      body: file,
                      signal,
                      credentials: "omit",
                      redirect: "error",
                      cache: "no-store",
                      headers: token
                        ? { Authorization: `Bearer ${token}` }
                        : {},
                    },
                  );
                  if (!response.ok) throw new Error("Upload failed");
                  const saved: Attachment = await response.json();
                  if (mounted.current) {
                    setFiles((current) => [
                      ...current.filter((f) => f.id !== saved.id),
                      saved,
                    ]);
                    setUploadUncertain(false);
                  }
                }, "Upload was not confirmed. Refresh files before uploading again; it may already have been saved.");
              }}
            />
          </label>
          <p>
            Files are stored in this thread. Preview a text file to include its
            contents in a message; uploading alone does not send its contents to
            the model.
          </p>
          {uploadUncertain && (
            <p role="status">
              Upload needs confirmation. Refresh evidence and inspect the file
              list before uploading again.
            </p>
          )}
          {files.map((file) => (
            <div className="thread-file-row" key={file.id}>
              <button
                className="secondary-button"
                disabled={busy}
                onClick={() =>
                  void run(async (signal) => {
                    const response = await fetch(
                      `/api/agents/${owner}/files/${encodeURIComponent(file.id)}`,
                      {
                        signal,
                        credentials: "omit",
                        redirect: "error",
                        cache: "no-store",
                        headers: token
                          ? { Authorization: `Bearer ${token}` }
                          : {},
                      },
                    );
                    if (!response.ok) throw new Error("Download failed");
                    download(await response.blob(), file.filename);
                  }, "File download failed. Refresh and try again.")
                }
              >
                Download {file.filename}
              </button>
              {canPreview(file.filename) && (
                <button
                  className="secondary-button"
                  disabled={busy}
                  onClick={() =>
                    setPreview(
                      preview?.id === file.id &&
                        preview.owner === owner &&
                        preview.thread === thread
                        ? null
                        : { id: file.id, owner, thread },
                    )
                  }
                >
                  {preview?.id === file.id &&
                  preview.owner === owner &&
                  preview.thread === thread
                    ? "Close preview"
                    : `Preview ${file.filename}`}
                </button>
              )}
              <small>
                {" "}
                {file.size.toLocaleString()} bytes · {file.source}
                <details>
                  <summary>Source details</summary>
                  <p>File {file.id}</p>
                  <p>SHA-256: {file.checksum}</p>
                </details>
              </small>
              {preview?.id === file.id &&
                preview.owner === owner &&
                preview.thread === thread && (
                  <FilePreview
                    token={token}
                    owner={owner}
                    file={file}
                    onIncludeText={onIncludeText}
                  />
                )}
            </div>
          ))}
          {!files.length && !loading && <p>No files attached yet.</p>}
        </article>
        {!filesOnly && (
          <article className="task-card">
            <h3>Decisions</h3>
            <p>
              Recorded by you. Decisions do not authorize tool execution or
              external writes.
            </p>
            {decisions.map((decision) => (
              <blockquote key={decision.id}>
                <p>{decision.content}</p>
                <small>{decision.created_at}</small>
              </blockquote>
            ))}
            {!decisions.length && !loading && <p>No decisions recorded yet.</p>}
            <form
              onSubmit={(e) => {
                e.preventDefault();
                const value = pending ?? {
                  id: crypto.randomUUID(),
                  content: draft.trim(),
                };
                if (!value.content) return;
                if (new TextEncoder().encode(value.content).length > 4000) {
                  setError(
                    "Decision exceeds 4,000 bytes. Shorten the text before saving.",
                  );
                  return;
                }
                setPending(value);
                void run(async (signal) => {
                  const saved = await operate<Decision[]>(
                    token,
                    owner,
                    "thread.decision.record",
                    { thread_id: thread, ...value },
                    signal,
                  );
                  if (mounted.current) {
                    setDecisions(saved);
                    setPending(null);
                    setDraft("");
                  }
                }, "Decision save was not confirmed. Retry this same decision safely, or refresh evidence to check it.");
              }}
            >
              <label>
                Record a decision
                <textarea
                  value={draft}
                  maxLength={4000}
                  disabled={busy || loading || !!pending}
                  onChange={(e) => setDraft(e.target.value)}
                />
              </label>
              <button
                className="primary-button"
                disabled={busy || loading || (!pending && !draft.trim())}
              >
                {pending ? "Retry decision save" : "Add decision"}
              </button>
            </form>
          </article>
        )}
        {!filesOnly && (
          <article className="task-card thread-worker-results">
            <h3>Worker results</h3>
            {!linkedWorkers.length && !loading && (
              <p>No workers for this thread.</p>
            )}
            {linkedWorkers.map((worker) => (
              <details key={worker.id}>
                <summary>
                  {worker.spec.role} · {worker.status}
                </summary>
                <p>{worker.spec.objective}</p>
                <small>
                  Worker {worker.id} · Task {worker.task_id}
                </small>
                {worker.error && <p role="alert">{worker.error}</p>}
                {worker.result ? (
                  <>
                    <p>{worker.result.summary}</p>
                    <p>
                      Confidence: {Math.round(worker.result.confidence * 100)}%
                    </p>
                    <ul>
                      {worker.result.findings.map((finding, i) => (
                        <li key={i}>{finding}</li>
                      ))}
                    </ul>
                    <h4>Sources</h4>
                    <ul>
                      {worker.result.sources.map((source, i) => (
                        <li key={i}>
                          {source.source_type}: {source.source_ref}
                        </li>
                      ))}
                    </ul>
                    {worker.result.errors.map((failure, i) => (
                      <p key={i}>{failure}</p>
                    ))}
                  </>
                ) : (
                  <p>
                    No structured result yet. Refresh evidence to check
                    progress.
                  </p>
                )}
                {worker.archive_path &&
                  [
                    "summary.md",
                    "transcript.jsonl",
                    "tool_calls.jsonl",
                    "shell.log",
                    "sources.json",
                    "costs.json",
                    "memory_candidates.json",
                    ...(worker.result?.artifacts.map((a) => a.filename) ?? []),
                  ].map((filename) => (
                    <button
                      key={filename}
                      className="secondary-button"
                      disabled={busy}
                      onClick={() =>
                        void run(async (signal) => {
                          const saved = await operate<{ data_base64: string }>(
                            token,
                            owner,
                            "archive.read",
                            { worker_id: worker.id, filename },
                            signal,
                          );
                          download(
                            new Blob([
                              Uint8Array.from(atob(saved.data_base64), (c) =>
                                c.charCodeAt(0),
                              ),
                            ]),
                            filename,
                          );
                        }, "Archive download failed. Refresh evidence and try again.")
                      }
                    >
                      Download {filename}
                    </button>
                  ))}
              </details>
            ))}
          </article>
        )}
      </div>
    </section>
  );
}
