import { useState } from "react";
import { RichCard } from "../cards";
import { operate, type Agent } from "../api";
import { useLiveData, useLivePages } from "./live-data";
type Watch = {
  enabled: boolean;
  interval_minutes: number;
  quiet_start_utc: number;
  quiet_end_utc: number;
  daily_runs: number;
  daily_budget_microusd: number;
  investigation_budget_microusd: number;
  stale_after_hours: number;
};
type Notice = {
  id: string;
  title: string;
  summary: string;
  status: string;
  task_id: string;
  created_at: number;
  read: boolean;
  run_id?: string;
  worker_id?: string | null;
  budget_microusd?: number;
  evidence?: { source_type: string; source_ref: string; content: string }[];
};
export function LiveWatches({
  token,
  owner,
  onTasks,
}: {
  token: string;
  owner: Agent;
  onTasks?: () => void;
}) {
  const settings = useLiveData<Watch>(token, owner, "watch.settings"),
    notices = useLivePages<Notice>(token, owner, "notification.list");
  const [draft, setDraft] = useState<Watch>(),
    [busy, setBusy] = useState(false),
    [error, setError] = useState("");
  const value =
    draft ??
    (typeof settings.data?.enabled === "boolean" ? settings.data : undefined);
  const fields: [Exclude<keyof Watch, "enabled">, string, number, number][] = [
    ["interval_minutes", "Check every (minutes)", 5, 1440],
    ["quiet_start_utc", "Quiet hours start (UTC hour)", 0, 23],
    ["quiet_end_utc", "Quiet hours end (UTC hour)", 0, 23],
    ["daily_runs", "Maximum investigations per UTC day", 1, 48],
    [
      "daily_budget_microusd",
      "Daily reservation budget (microUSD)",
      1,
      100000000,
    ],
    [
      "investigation_budget_microusd",
      "Budget per investigation (microUSD)",
      1,
      100000000,
    ],
    ["stale_after_hours", "Work becomes stale after (hours)", 1, 8760],
  ];
  return (
    <section
      className="settings-card live-watch-settings"
      aria-label="Watch recommendations"
    >
      <div className="live-section-heading">
        <h3>Watch recommendations</h3>
        <button
          className="secondary-button"
          onClick={() => {
            notices.refresh();
            settings.refresh();
          }}
        >
          Refresh recommendations
        </button>
      </div>
      {(settings.error || notices.error || error) && (
        <p role="alert">{error || settings.error || notices.error}</p>
      )}
      {(settings.loading || notices.loading) && (
        <p role="status">Loading watch state…</p>
      )}
      {notices.items.map((notice) => (
        <article className="live-session-action" key={notice.id}>
          <RichCard
            value={{
              version: 1,
              type: "recommendation",
              title: notice.title,
              summary: notice.summary,
              status: notice.status,
            }}
            openThread={() => {}}
          />
          <small>
            {new Date(notice.created_at * 1000).toLocaleString()} · Task{" "}
            {notice.task_id}
          </small>
          <details className="notice-evidence">
            <summary>Evidence and investigation</summary>
            <p>These are the recorded sources used for this recommendation.</p>
            {notice.run_id && <p>Investigation {notice.run_id}</p>}
            {notice.worker_id && <p>Worker {notice.worker_id}</p>}
            {notice.budget_microusd !== undefined && (
              <p>
                Reserved budget: US$
                {(notice.budget_microusd / 1_000_000).toFixed(2)}. Actual model
                cost is recorded separately.
              </p>
            )}
            {notice.evidence?.map((source, i) => (
              <section key={i} aria-label={`Evidence source ${i + 1}`}>
                <h5>
                  {source.source_type}: {source.source_ref}
                </h5>
                <pre className="live-review-payload">
                  {formatEvidence(source.content)}
                </pre>
              </section>
            ))}
            {!notice.evidence?.length && (
              <p>No source details available for this recommendation.</p>
            )}
          </details>
          {onTasks && (
            <button className="secondary-button" onClick={onTasks}>
              View related tasks
            </button>
          )}
          {!notice.read && (
            <button
              className="secondary-button"
              disabled={busy}
              onClick={async () => {
                setBusy(true);
                setError("");
                try {
                  await operate(token, owner, "notification.read", {
                    id: notice.id,
                  });
                  notices.refresh();
                } catch {
                  setError(
                    "Review was not confirmed. Refresh recommendations before trying again.",
                  );
                } finally {
                  setBusy(false);
                }
              }}
            >
              Mark reviewed
            </button>
          )}
        </article>
      ))}
      {!notices.loading && !notices.error && !notices.items.length && (
        <p>No watch recommendations yet.</p>
      )}
      {notices.hasMore && (
        <button
          className="secondary-button"
          disabled={notices.loading}
          onClick={() => void notices.more()}
        >
          Load more recommendations
        </button>
      )}
      <details>
        <summary>Watch schedule for {owner}</summary>
        <p>
          Investigates owned due or stale tasks and failed read workflows.
          Returns recommendations for review. Equal quiet-hour values disable
          quiet hours. One USD equals 1,000,000 microUSD.
        </p>
        {value && (
          <form
            className="workspace-form"
            onSubmit={async (event) => {
              event.preventDefault();
              if (busy) return;
              setBusy(true);
              setError("");
              try {
                await operate(token, owner, "watch.configure", {
                  settings: value,
                });
                setDraft(undefined);
                settings.refresh();
              } catch {
                setError(
                  "Schedule save was not confirmed. Refresh watch settings to check the saved state.",
                );
              } finally {
                setBusy(false);
              }
            }}
          >
            <fieldset disabled={busy}>
              <label>
                <input
                  type="checkbox"
                  checked={value.enabled}
                  onChange={(e) =>
                    setDraft({ ...value, enabled: e.target.checked })
                  }
                />
                Enable {owner}’s watch
              </label>
              <div className="live-memory-filters">
                {fields.map(([key, label, min, max]) => (
                  <label key={key}>
                    {label}
                    <input
                      required
                      type="number"
                      min={min}
                      max={max}
                      step="1"
                      value={value[key]}
                      onChange={(e) =>
                        setDraft({ ...value, [key]: Number(e.target.value) })
                      }
                    />
                  </label>
                ))}
              </div>
            </fieldset>
            <button className="primary-button" disabled={busy}>
              Save schedule
            </button>
          </form>
        )}
      </details>
    </section>
  );
}

function formatEvidence(content: string) {
  try {
    return JSON.stringify(JSON.parse(content), null, 2);
  } catch {
    return content;
  }
}
