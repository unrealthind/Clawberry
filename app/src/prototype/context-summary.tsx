import { useEffect, useState } from "react";
import { Icon } from "./shared";
import type { Agent, Task } from "../api";
import { useLiveData } from "./live-data";

type Review = {
  id: string;
  workflow: string;
  state: string;
  expires_at: number;
};
type Usage = {
  input_tokens: number;
  output_tokens: number;
  cost_microusd: number;
  usage_reported: boolean;
};
export function ContextSummary({
  token,
  owner,
  tasks,
  onApprovals,
  onTasks,
  dailyBudget,
  tasksPartial = false,
  tasksUnavailable = false,
}: {
  token: string;
  owner: Agent;
  tasks: Task[];
  onApprovals: () => void;
  onTasks: () => void;
  dailyBudget?: number;
  tasksPartial?: boolean;
  tasksUnavailable?: boolean;
}) {
  const writes = useLiveData<Review[]>(token, owner, "write.reviews");
  const peers = useLiveData<
    { id: string; requester: Agent; status: string; expires_at: number }[]
  >(token, owner, "cross.review_queue");
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  const pendingWrites = (Array.isArray(writes.data) ? writes.data : []).filter(
    (r) => r.state === "pending" && r.expires_at * 1000 > now,
  );
  const pendingPeers = (Array.isArray(peers.data) ? peers.data : []).filter(
    (r) =>
      r.requester === owner &&
      ["pending", "response_pending"].includes(r.status) &&
      r.expires_at * 1000 > now,
  );
  const usage = useLiveData<Usage[]>(token, owner, "usage.list");
  const active = tasks.filter(
    (t) => !["completed", "cancelled"].includes(t.status),
  );
  const task =
    active.find((t) => ["executing", "investigating"].includes(t.status)) ??
    active[0];
  const totals = (Array.isArray(usage.data) ? usage.data : []).reduce(
    (s, r) => ({
      input: s.input + r.input_tokens,
      output: s.output + r.output_tokens,
      cost: s.cost + r.cost_microusd,
    }),
    { input: 0, output: 0, cost: 0 },
  );
  return (
    <>
      <section
        className="context-section approval-section"
        aria-label="Pending approvals summary"
      >
        <div className="section-heading">
          <h2>
            <span className="panel-icon approval-icon">
              <Icon name="shield" />
            </span>
            Pending approval
          </h2>
          <button onClick={onApprovals}>View all</button>
        </div>
        {writes.loading || peers.loading ? (
          <p role="status">Loading approvals…</p>
        ) : writes.error ||
          peers.error ||
          (writes.data !== undefined && !Array.isArray(writes.data)) ||
          (peers.data !== undefined && !Array.isArray(peers.data)) ? (
          <p role="alert">Approvals could not be loaded.</p>
        ) : (
          <>
            {pendingWrites.slice(0, 2).map((r) => (
              <div className="approval-card" key={r.id}>
                <div className="approval-document">
                  <span className="file-icon">
                    <Icon name="file" />
                  </span>
                  <div>
                    <h3>{r.workflow}</h3>
                    <p>Ready for your review</p>
                    <small>External action request</small>
                  </div>
                </div>
                <div className="approval-actions">
                  <button className="approve-button" onClick={onApprovals}>
                    <Icon name="check" />
                    Review request
                  </button>
                </div>
              </div>
            ))}
            {pendingPeers.length > 0 && (
              <div className="approval-card">
                <p>
                  {pendingPeers.length} cross-agent{" "}
                  {pendingPeers.length === 1
                    ? "request needs"
                    : "requests need"}{" "}
                  exact review.
                </p>
                <button className="secondary-button" onClick={onApprovals}>
                  Review cross-agent requests
                </button>
              </div>
            )}
            {!pendingWrites.length && !pendingPeers.length && (
              <div className="context-empty">
                <span className="context-empty-icon">
                  <Icon name="check" />
                </span>
                <div>
                  <strong>All caught up</strong>
                  <p>No requests waiting for review.</p>
                </div>
              </div>
            )}
          </>
        )}
      </section>
      <section
        className="context-section task-section"
        aria-label="Active task summary"
      >
        <div className="section-heading">
          <h2>
            <span className="panel-icon task-icon">
              <Icon name="tasks" />
            </span>
            Active task
          </h2>
          <button onClick={onTasks}>View all</button>
        </div>
        {!tasksUnavailable && tasksPartial && (
          <p>
            Summary of loaded tasks. Older tasks may need attention; open Tasks
            to load more.
          </p>
        )}
        {tasksUnavailable ? (
          <p role="status">Refresh history to load current tasks.</p>
        ) : task ? (
          <div className="task-card compact">
            <div className="task-title">
              <span className="context-task-icon">
                <Icon name="tasks" />
              </span>
              <div>
                <h3>{task.title}</h3>
                <p>
                  {task.status.replaceAll("_", " ")} · {task.priority} priority
                </p>
                {task.due_at && (
                  <small>
                    Due {new Date(task.due_at).toLocaleDateString()}
                  </small>
                )}
              </div>
            </div>
            <button className="context-task-link" onClick={onTasks}>
              Open tasks
            </button>
          </div>
        ) : (
          <div className="context-empty">
            <span className="context-empty-icon">
              <Icon name="tasks" />
            </span>
            <div>
              <strong>
                {tasksPartial ? "No active tasks loaded" : "A clear task list"}
              </strong>
              <p>Active work will appear here.</p>
            </div>
          </div>
        )}
      </section>
      <section className="api-usage-card" aria-label="Agent API usage">
        <div className="api-usage-heading">
          <span className="claude-monogram" aria-hidden="true">
            C
          </span>
          <h2>API usage</h2>
          <span>All recorded</span>
        </div>
        {usage.loading ? (
          <p role="status">Loading usage…</p>
        ) : usage.error ||
          (usage.data !== undefined && !Array.isArray(usage.data)) ? (
          <p role="alert">Usage could not be loaded.</p>
        ) : (
          <>
            <div className="api-usage-total">
              <div>
                <span>Recorded cost</span>
                <strong>
                  <small>US$</small>
                  {(totals.cost / 1_000_000).toFixed(2)}
                </strong>
              </div>
              <div>
                <span>Total tokens</span>
                <strong>
                  {(totals.input + totals.output).toLocaleString()}
                </strong>
              </div>
            </div>
            <dl className="api-token-breakdown">
              <div>
                <dt>
                  <span className="input-token-dot" />
                  Input tokens
                </dt>
                <dd>{totals.input.toLocaleString()}</dd>
              </div>
              <div>
                <dt>
                  <span className="output-token-dot" />
                  Output tokens
                </dt>
                <dd>{totals.output.toLocaleString()}</dd>
              </div>
            </dl>
            {Array.isArray(usage.data) &&
              usage.data.some((r) => !r.usage_reported) && (
                <p>Includes unconfirmed usage.</p>
              )}
            <div className="api-budget-heading">
              <span>Daily budget</span>
              <span>
                {dailyBudget === undefined
                  ? "Unavailable"
                  : `US$${(dailyBudget / 1_000_000).toFixed(2)}`}
              </span>
            </div>
            <p>
              {owner === "janet" ? "Janet" : "Harley"} + subagents · Recorded
              usage
            </p>
          </>
        )}
      </section>
    </>
  );
}
