import { useEffect, useRef, useState } from "react";
import { operate, type Agent, type Task } from "../api";

type Evidence = {
  task: Task;
  events: { event: string; detail: string; created_at: string }[];
  workers: {
    id: string;
    status: string;
    error?: string;
    spec: { role: string };
    result?: null | {
      summary: string;
      findings: string[];
      confidence: number;
      sources: { source_type: string; source_ref: string }[];
    };
  }[];
  attachments: { id: string; filename: string; checksum: string }[];
};
/** Load evidence only when requested; never infer completion from assistant prose. */
export function TaskEvidence({
  token,
  owner,
  task,
  openThread,
}: {
  token: string;
  owner: Agent;
  task: Task;
  openThread: (id: string) => void;
}) {
  const [data, setData] = useState<Evidence>();
  const [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const active = useRef<AbortController | null>(null);
  useEffect(() => () => active.current?.abort(), []);
  async function load() {
    if (busy) return;
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setError("");
    try {
      const value = await operate<Evidence>(
        token,
        owner,
        "task.get",
        { task_id: task.id },
        controller.signal,
      );
      if (
        !value.task ||
        value.task.id !== task.id ||
        !Array.isArray(value.events) ||
        !Array.isArray(value.workers) ||
        !Array.isArray(value.attachments)
      )
        throw new Error("Invalid task evidence");
      if (!controller.signal.aborted) setData(value);
    } catch {
      if (!controller.signal.aborted)
        setError(
          "Task evidence could not be loaded. Refresh evidence to retry.",
        );
    } finally {
      if (!controller.signal.aborted) setBusy(false);
    }
  }
  return (
    <details
      className="task-evidence"
      onToggle={(event) => {
        if (event.currentTarget.open && !data && !busy) void load();
      }}
    >
      <summary>Task history and evidence</summary>
      {busy && <p role="status">Loading task evidence…</p>}
      {error && <p role="alert">{error}</p>}
      <button
        type="button"
        className="secondary-button"
        disabled={busy}
        onClick={() => void load()}
      >
        Refresh task evidence
      </button>
      {data && (
        <>
          <p>
            Task {data.task.id} · Recorded status:{" "}
            {data.task.status.replaceAll("_", " ")}
          </p>
          <h4>Recorded history</h4>
          {!data.events.length && <p>No recorded changes.</p>}
          {data.events.map((event, index) => (
            <section key={index}>
              <strong>{event.event.replaceAll("_", " ")}</strong>
              <small> · {event.created_at}</small>
              <pre className="live-review-payload">{event.detail}</pre>
            </section>
          ))}
          <h4>Workers and findings</h4>
          {!data.workers.length && <p>No workers attached to this task.</p>}
          {data.workers.map((worker) => (
            <section key={worker.id}>
              <h5>
                {worker.spec.role} · {worker.status}
              </h5>
              <small>Worker {worker.id}</small>
              {worker.error && <p>{worker.error}</p>}
              {worker.result && (
                <>
                  <p>{worker.result.summary}</p>
                  <p>
                    Confidence {Math.round(worker.result.confidence * 100)}%
                  </p>
                  <ul>
                    {worker.result.findings.map((finding, i) => (
                      <li key={i}>{finding}</li>
                    ))}
                  </ul>
                  {worker.result.sources.map((source, i) => (
                    <p key={i}>
                      {source.source_type}: {source.source_ref}
                    </p>
                  ))}
                </>
              )}
            </section>
          ))}
          {data.attachments.length > 0 && (
            <>
              <h4>Task files</h4>
              {data.attachments.map((file) => (
                <p key={file.id}>
                  {file.filename} · SHA-256 {file.checksum}
                </p>
              ))}
            </>
          )}
          {data.task.thread_id && (
            <button
              type="button"
              className="secondary-button"
              onClick={() => openThread(data.task.thread_id!)}
            >
              Open thread evidence
            </button>
          )}
        </>
      )}
    </details>
  );
}
