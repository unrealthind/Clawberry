import { useEffect, useRef, useState } from "react";
export type WorkspaceTask = {
  id: string;
  title: string;
  description: string;
  dueDate: string;
  due?: string;
  priority: "low" | "normal" | "high" | "urgent";
  thread: string | null;
  steps: string[];
  done: boolean[];
};
export function TaskEditor({
  task,
  threadChoices,
  threads,
  onSave,
  onCancel,
  busy = false,
  error = "",
  live = false,
  defaultThread = null,
  saveBlocked = false,
}: {
  task: WorkspaceTask | null | undefined;
  threads: string[];
  threadChoices?: { id: string; title: string }[];
  defaultThread?: string | null;
  saveBlocked?: boolean;
  busy?: boolean;
  error?: string;
  live?: boolean;
  onSave: (task: WorkspaceTask) => void;
  onCancel: () => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const [title, setTitle] = useState(task?.title ?? "");
  const [description, setDescription] = useState(task?.description ?? "");
  const [dueDate, setDueDate] = useState(task?.dueDate ?? "");
  const [priority, setPriority] = useState<WorkspaceTask["priority"]>(
    task?.priority ?? "normal",
  );
  const [thread, setThread] = useState(task?.thread ?? "");
  const [steps, setSteps] = useState(task?.steps.join("\n") ?? "");
  useEffect(() => {
    if (task !== undefined) {
      setTitle(task?.title ?? "");
      setDescription(task?.description ?? "");
      setDueDate(task?.dueDate ?? "");
      setPriority(task?.priority ?? "normal");
      setThread(task?.thread ?? defaultThread ?? "");
      setSteps(task?.steps.join("\n") ?? "");
      dialog.current?.showModal();
    } else dialog.current?.close();
  }, [task, defaultThread]);
  return (
    <dialog
      ref={dialog}
      className="prototype-dialog"
      aria-labelledby="task-editor-title"
      onCancel={(event) => {
        if (busy) event.preventDefault();
        else onCancel();
      }}
    >
      <h2 id="task-editor-title">{task ? "Edit task" : "New task"}</h2>
      <form
        className="workspace-form"
        onSubmit={(event) => {
          event.preventDefault();
          if (busy || saveBlocked || !title.trim()) return;
          const checklist = steps
            .split("\n")
            .map((s) => s.trim())
            .filter(Boolean);
          if (!checklist.length) checklist.push("Complete task");
          onSave({
            id: task?.id ?? crypto.randomUUID(),
            title: title.trim(),
            description: description.trim(),
            dueDate,
            priority,
            thread: thread || null,
            steps: checklist,
            done: checklist.map(
              (step) => task?.done[task.steps.indexOf(step)] ?? false,
            ),
          });
        }}
      >
        {error && <p role="alert">{error}</p>}
        <fieldset disabled={busy}>
          <label>
            Task title
            <input
              autoFocus
              value={title}
              maxLength={200}
              onChange={(e) => setTitle(e.target.value)}
              required
            />
          </label>
          <label>
            Description
            <textarea
              value={description}
              maxLength={16000}
              onChange={(e) => setDescription(e.target.value)}
            />
          </label>
          <label>
            Due date
            <input
              type="date"
              value={dueDate}
              onChange={(e) => setDueDate(e.target.value)}
            />
          </label>
          <label>
            Priority
            <select
              value={priority}
              onChange={(e) =>
                setPriority(e.target.value as WorkspaceTask["priority"])
              }
            >
              {["low", "normal", "high", "urgent"].map((p) => (
                <option key={p}>{p}</option>
              ))}
            </select>
          </label>
          <label>
            Focus Thread
            <select value={thread} onChange={(e) => setThread(e.target.value)}>
              <option value="">No thread</option>
              {(threadChoices ?? threads.map((t) => ({ id: t, title: t }))).map(
                (t) => (
                  <option key={t.id} value={t.id}>
                    {t.title}
                  </option>
                ),
              )}
            </select>
          </label>
          {!live && (
            <label>
              Checklist (one step per line)
              <textarea
                value={steps}
                maxLength={8000}
                onChange={(e) => setSteps(e.target.value)}
              />
            </label>
          )}
          <div className="dialog-actions">
            <button
              type="button"
              className="secondary-button"
              onClick={onCancel}
            >
              Cancel
            </button>
            <button
              className="primary-button"
              disabled={saveBlocked || !title.trim()}
            >
              {busy ? "Saving…" : "Save task"}
            </button>
          </div>
        </fieldset>
      </form>
    </dialog>
  );
}
