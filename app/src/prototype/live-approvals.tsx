import { useEffect, useRef, useState } from "react";
import { operate, type Agent, type Task } from "../api";
import { authorizeWrite } from "../authorization";
import { useLiveData, useLivePages } from "./live-data";

type Exchange = {
  id: string;
  requester: Agent;
  recipient: Agent;
  task_id: string;
  question: string;
  question_hash: string;
  response: string | null;
  response_hash: string | null;
  status: string;
  expires_at: number;
};
type Review = {
  id: string;
  task_id: string;
  workflow: string;
  manifest: Record<string, unknown>;
  manifest_hash: string;
  expires_at: number;
  state: string;
};
type Session = {
  expires_at: number;
  revoked: boolean;
  gateway_end_confirmed?: boolean | null;
  remaining: Record<string, number>;
  actions: {
    id: string;
    operation: string;
    status: string;
    gateway_action_id?: string;
    gateway_audit_id?: string;
    n8n_execution_id?: string;
  }[];
};
const date = (seconds: number) => new Date(seconds * 1000).toLocaleString();
function useNow() {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const timer = setInterval(() => setNow(Date.now()), 1000);
    return () => clearInterval(timer);
  }, []);
  return now;
}
export function LiveApprovals({
  token,
  owner,
}: {
  token: string;
  owner: Agent;
}) {
  const writes = useLiveData<Review[]>(token, owner, "write.reviews");
  const peers = useLiveData<Exchange[]>(token, owner, "cross.review_queue");
  return (
    <section className="live-management" aria-label="Approvals workspace">
      <div className="live-section-heading">
        <div>
          <h2>Review requests</h2>
          <p>Review the exact action or scope before giving permission.</p>
        </div>
        <button
          className="secondary-button"
          onClick={() => {
            writes.refresh();
            peers.refresh();
          }}
        >
          Refresh approvals
        </button>
      </div>
      <section aria-label="External write approvals">
        <h3>External writes</h3>
        <p>
          Single actions authorize one exact operation. Bounded sessions
          authorize only the displayed resources, duration and operation
          budgets. TOTP goes directly to the approval service.
        </p>
        {writes.loading && <p role="status">Loading write requests…</p>}
        {writes.error && <p role="alert">{writes.error}</p>}
        {writes.data?.map((review) => (
          <WriteReview
            key={`${owner}:${review.id}:${review.manifest_hash}:${review.state}`}
            token={token}
            owner={owner}
            review={review}
            refresh={writes.refresh}
          />
        ))}
        {writes.data?.length === 0 && (
          <div className="live-empty">
            <h4>No write requests</h4>
            <p>Actions that need your authorization will appear here.</p>
          </div>
        )}
      </section>
      <section aria-label="Cross-agent approvals">
        <h3>Between Janet and Harley</h3>
        <p>
          Permission applies only to the exact question or response shown. Their
          private memory stays separate.
        </p>
        <PeerRequest token={token} owner={owner} refresh={peers.refresh} />
        {peers.loading && <p role="status">Loading cross-agent requests…</p>}
        {peers.error && <p role="alert">{peers.error}</p>}
        {peers.data?.map((exchange) => (
          <PeerReview
            key={`${owner}:${exchange.id}:${exchange.status}:${exchange.question_hash}:${exchange.response_hash}`}
            token={token}
            owner={owner}
            exchange={exchange}
            refresh={peers.refresh}
          />
        ))}
        {peers.data?.length === 0 && (
          <div className="live-empty">
            <h4>No cross-agent requests</h4>
            <p>
              Requests to share a specific question or response will appear
              here.
            </p>
          </div>
        )}
      </section>
    </section>
  );
}
function WriteReview({
  token,
  owner,
  review,
  refresh,
}: {
  token: string;
  owner: Agent;
  review: Review;
  refresh: () => void;
}) {
  const now = useNow();
  const expired = review.expires_at * 1000 <= now;
  const [checked, setChecked] = useState(false),
    [code, setCode] = useState(""),
    [busy, setBusy] = useState(false),
    [error, setError] = useState(""),
    [uncertain, setUncertain] = useState(false);
  const [session, setSession] = useState<Session>();
  const statusVersion = useRef(0);
  useEffect(() => {
    if (expired) {
      setCode("");
      setChecked(false);
    }
  }, [expired]);
  const load = async () => {
    setSession(undefined);
    const version = ++statusVersion.current;
    const result = await operate<Session>(token, owner, "write.status", {
      task_id: review.task_id,
      session_id: review.id,
    });
    if (version === statusVersion.current) setSession(result);
  };
  useEffect(() => {
    setSession(undefined);
    if (review.state !== "authorized") return;
    const version = ++statusVersion.current;
    const controller = new AbortController();
    operate<Session>(
      token,
      owner,
      "write.status",
      { task_id: review.task_id, session_id: review.id },
      controller.signal,
    )
      .then((value) => {
        if (!controller.signal.aborted && version === statusVersion.current)
          setSession(value);
      })
      .catch(() => {
        if (!controller.signal.aborted && version === statusVersion.current)
          setError("Session status unavailable. Refresh before continuing.");
      });
    return () => controller.abort();
  }, [token, owner, review.id, review.task_id, review.state]);
  async function authorize(event: React.FormEvent) {
    event.preventDefault();
    if (
      !checked ||
      busy ||
      uncertain ||
      expired ||
      review.expires_at * 1000 <= Date.now() ||
      code.length !== 6
    )
      return;
    const submitted = code;
    setCode("");
    setChecked(false);
    setBusy(true);
    setError("");
    try {
      // Never route a code through operations, chat, storage or a query/mutation cache.
      await authorizeWrite(
        token,
        owner,
        review.id,
        review.manifest_hash,
        submitted,
      );
      refresh();
    } catch {
      setUncertain(true);
      setError(
        "Authorization was not confirmed. Refresh approvals before requesting another authorization; it may already have completed.",
      );
    } finally {
      setBusy(false);
    }
  }
  async function change(operation: string, fields: object = {}) {
    if (busy) return;
    setBusy(true);
    setError("");
    try {
      await operate(token, owner, operation, {
        task_id: review.task_id,
        session_id: review.id,
        ...fields,
      });
      await load();
    } catch {
      setError(
        "The result was not confirmed. Refresh session status before taking another action.",
      );
    } finally {
      setBusy(false);
    }
  }
  return (
    <article
      className="settings-card approval-review"
      aria-label={`${review.workflow} approval`}
    >
      <div className="live-section-heading">
        <h4>
          {review.manifest.mode === "single_action"
            ? "Single action"
            : "Bounded session"}{" "}
          · {review.workflow}
        </h4>
        <span className="live-status">
          {expired && review.state === "pending"
            ? "Expired"
            : review.state.replaceAll("_", " ")}
        </span>
      </div>
      <small>
        Task {review.task_id} · Review expires {date(review.expires_at)}
      </small>
      <h5>Exact action or session scope</h5>
      <pre className="live-review-payload">
        {JSON.stringify(review.manifest, null, 2)}
      </pre>
      <details>
        <summary>Review binding</summary>
        <code>{review.manifest_hash}</code>
      </details>
      {error && <p role="alert">{error}</p>}
      {session?.revoked && session.gateway_end_confirmed !== true && (
        <p role="alert">Ended locally. Gateway closure is unconfirmed.</p>
      )}
      {review.state === "pending" && !expired && (
        <form className="workspace-form" onSubmit={authorize}>
          <fieldset disabled={busy || uncertain}>
            <label>
              <input
                type="checkbox"
                checked={checked}
                onChange={(e) => setChecked(e.target.checked)}
              />
              I reviewed this exact action or session scope and budgets.
            </label>
            <label>
              TOTP authorization code
              <input
                aria-label="TOTP authorization code"
                type="password"
                inputMode="numeric"
                autoComplete="off"
                maxLength={6}
                pattern="[0-9]{6}"
                value={code}
                onChange={(e) => setCode(e.target.value.replace(/[^0-9]/g, ""))}
              />
            </label>
            <button
              className="primary-button"
              disabled={!checked || code.length !== 6 || busy || uncertain}
            >
              {busy ? "Authorizing…" : "Authorize with TOTP"}
            </button>
          </fieldset>
        </form>
      )}
      {review.state === "authorized" && (
        <>
          <div className="task-controls">
            <button
              className="secondary-button"
              disabled={busy}
              onClick={async () => {
                if (busy) return;
                setBusy(true);
                setError("");
                try {
                  await load();
                } catch {
                  setError("Session status unavailable. Refresh to try again.");
                } finally {
                  setBusy(false);
                }
              }}
            >
              Refresh session status
            </button>
            <button
              className="secondary-button"
              disabled={busy || session?.revoked}
              onClick={() => void change("write.end")}
            >
              End session
            </button>
          </div>
          {!session && !error && <p role="status">Loading session status…</p>}
          {session && (
            <>
              <p>
                <strong>
                  {session.revoked
                    ? "Ended"
                    : session.expires_at * 1000 <= now
                      ? "Expired"
                      : "Active"}
                </strong>{" "}
                · Expires {date(session.expires_at)}
              </p>
              <h5>Remaining operation budgets</h5>
              <dl className="live-budget-list">
                {Object.entries(session.remaining).map(
                  ([operation, remaining]) => (
                    <div key={operation}>
                      <dt>{operation}</dt>
                      <dd>{remaining}</dd>
                    </div>
                  ),
                )}
              </dl>
              <h5>Results</h5>
              {!session.actions.length && <p>No operations dispatched.</p>}
              {session.actions.map((action) => (
                <div className="live-session-action" key={action.id}>
                  <strong>{action.operation}</strong> · {action.status}
                  <small>
                    Action {action.id}
                    {action.gateway_audit_id &&
                      ` · Audit ${action.gateway_audit_id}`}
                    {action.n8n_execution_id &&
                      ` · n8n ${action.n8n_execution_id}`}
                  </small>
                  {action.status === "uncertain" && (
                    <>
                      <p>
                        Outcome needs reconciliation before an equivalent write
                        can be retried.
                      </p>
                      <button
                        className="secondary-button"
                        disabled={busy}
                        onClick={() =>
                          void change("write.reconcile", {
                            action_id: action.id,
                          })
                        }
                      >
                        Reconcile outcome
                      </button>
                    </>
                  )}
                </div>
              ))}
            </>
          )}
        </>
      )}
    </article>
  );
}
function PeerReview({
  token,
  owner,
  exchange,
  refresh,
}: {
  token: string;
  owner: Agent;
  exchange: Exchange;
  refresh: () => void;
}) {
  const now = useNow(),
    expired = exchange.expires_at * 1000 <= now;
  const [checked, setChecked] = useState(false),
    [busy, setBusy] = useState(false),
    [uncertain, setUncertain] = useState(false),
    [error, setError] = useState("");
  const pending =
    exchange.requester === owner &&
    ["pending", "response_pending"].includes(exchange.status);
  const revocable =
    exchange.requester === owner &&
    !["closed", "revoked", "expired"].includes(exchange.status) &&
    !expired;
  const [response, setResponse] = useState("");
  async function prepareResponse(event: React.FormEvent) {
    event.preventDefault();
    if (
      busy ||
      uncertain ||
      expired ||
      exchange.expires_at * 1000 <= Date.now() ||
      !response.trim()
    )
      return;
    if (new TextEncoder().encode(response).length > 4000) {
      setError("Response must fit within 4,000 bytes.");
      return;
    }
    setBusy(true);
    setError("");
    try {
      await operate(token, owner, "cross.respond", {
        id: exchange.id,
        response,
      });
      setResponse("");
      refresh();
    } catch {
      setUncertain(true);
      setError(
        "Response preparation was not confirmed. Refresh approvals before trying again.",
      );
    } finally {
      setBusy(false);
    }
  }
  async function decide(decision: string) {
    if (
      busy ||
      uncertain ||
      (decision !== "revoke" &&
        (!checked || expired || exchange.expires_at * 1000 <= Date.now()))
    )
      return;
    setBusy(true);
    setError("");
    setChecked(false);
    try {
      await operate(token, owner, "cross.review", {
        id: exchange.id,
        reviewed_hash:
          decision === "approve_response"
            ? exchange.response_hash
            : exchange.question_hash,
        decision,
      });
      refresh();
    } catch {
      setUncertain(true);
      setError(
        "Decision was not confirmed. Refresh approvals to check its status before trying again.",
      );
    } finally {
      setBusy(false);
    }
  }
  return (
    <article
      className="settings-card approval-review"
      aria-label="Cross-agent request"
    >
      <div className="live-section-heading">
        <h4>
          {exchange.requester} → {exchange.recipient}
        </h4>
        <span className="live-status">
          {expired && !["closed", "revoked"].includes(exchange.status)
            ? "Expired"
            : exchange.status.replaceAll("_", " ")}
        </span>
      </div>
      <small>
        Task {exchange.task_id} · Expires {date(exchange.expires_at)}
      </small>
      <h5>Exact question</h5>
      <p className="live-exact-text">{exchange.question}</p>
      {exchange.response !== null && (
        <>
          <h5>Exact response</h5>
          <p className="live-exact-text">{exchange.response}</p>
        </>
      )}
      <details>
        <summary>Review binding</summary>
        <p>
          Question: <code>{exchange.question_hash}</code>
        </p>
        {exchange.response_hash && (
          <p>
            Response: <code>{exchange.response_hash}</code>
          </p>
        )}
      </details>
      {error && <p role="alert">{error}</p>}
      {exchange.recipient === owner &&
        exchange.status === "approved" &&
        !expired && (
          <form className="workspace-form" onSubmit={prepareResponse}>
            <p>
              The exact question has been approved. Prepare only the response
              needed for this scope; User must review it separately before
              delivery.
            </p>
            <label htmlFor={`peer-response-${exchange.id}`}>
              Scoped response
              <textarea
                id={`peer-response-${exchange.id}`}
                required
                maxLength={4000}
                value={response}
                disabled={busy || uncertain}
                onChange={(e) => setResponse(e.target.value)}
              />
            </label>
            <button
              className="primary-button"
              disabled={busy || uncertain || !response.trim()}
            >
              Prepare response for review
            </button>
          </form>
        )}
      {exchange.status === "approved" && exchange.requester === owner && (
        <p>
          Question approved. Waiting for a scoped response; no private memory
          access was granted.
        </p>
      )}
      {exchange.status === "response_pending" &&
        exchange.recipient === owner && (
          <p>
            Response prepared. Waiting for User’s exact response review before
            delivery.
          </p>
        )}
      {exchange.status === "closed" && (
        <p>Reviewed response delivered. The temporary exchange is closed.</p>
      )}
      {revocable && !pending && (
        <button
          className="secondary-button"
          disabled={busy || uncertain}
          onClick={() => void decide("revoke")}
        >
          Revoke exchange
        </button>
      )}
      {pending && (
        <div className="workspace-form">
          <label>
            <input
              type="checkbox"
              checked={checked}
              disabled={busy || uncertain || expired}
              onChange={(e) => setChecked(e.target.checked)}
            />
            I reviewed this exact{" "}
            {exchange.status === "pending" ? "question" : "response"} and its
            limited scope.
          </label>
          <div className="task-controls">
            <button
              className="primary-button"
              disabled={!checked || busy || uncertain || expired}
              onClick={() =>
                void decide(
                  exchange.status === "pending"
                    ? "approve_question"
                    : "approve_response",
                )
              }
            >
              Approve {exchange.status === "pending" ? "question" : "response"}
            </button>
            <button
              className="secondary-button"
              disabled={busy || uncertain}
              onClick={() => void decide("revoke")}
            >
              Revoke exchange
            </button>
          </div>
        </div>
      )}
    </article>
  );
}

function PeerRequest({
  token,
  owner,
  refresh,
}: {
  token: string;
  owner: Agent;
  refresh: () => void;
}) {
  const tasks = useLivePages<Task>(token, owner, "task.page");
  const [open, setOpen] = useState(false),
    [task, setTask] = useState(""),
    [question, setQuestion] = useState("");
  const [busy, setBusy] = useState(false),
    [uncertain, setUncertain] = useState(false),
    [checkedStatus, setCheckedStatus] = useState(false),
    [error, setError] = useState("");
  const recipient = owner === "janet" ? "harley" : "janet";
  return (
    <details open={open} onToggle={(e) => setOpen(e.currentTarget.open)}>
      <summary>Prepare a cross-agent request</summary>
      <p>
        Ask {recipient === "janet" ? "Janet" : "Harley"} for one specific piece
        of information. Creating a request does not deliver it or grant access.
        Review the exact question below afterward.
      </p>
      {tasks.error && <p role="alert">{tasks.error}</p>}
      {tasks.loading && <p role="status">Loading owned tasks…</p>}
      {tasks.hasMore && (
        <button
          type="button"
          className="secondary-button"
          disabled={tasks.loading}
          onClick={() => void tasks.more()}
        >
          Load more request tasks
        </button>
      )}
      {!tasks.loading && !tasks.error && !tasks.items.length && (
        <p>Create a task first so this request has an owned purpose.</p>
      )}
      <form
        className="workspace-form"
        onSubmit={async (event) => {
          event.preventDefault();
          if (busy || uncertain || !task || !question.trim()) return;
          if (new TextEncoder().encode(question).length > 4000) {
            setError("Question must fit within 4,000 bytes.");
            return;
          }
          setBusy(true);
          setError("");
          try {
            await operate(token, owner, "cross.request", {
              recipient,
              task_id: task,
              question,
            });
            setQuestion("");
            setOpen(false);
            refresh();
          } catch {
            setUncertain(true);
            setError(
              "Request creation was not confirmed. Refresh approvals and inspect existing requests before preparing another.",
            );
          } finally {
            setBusy(false);
          }
        }}
      >
        <fieldset disabled={busy || uncertain || tasks.loading}>
          <label htmlFor="peer-task">
            Related task
            <select
              id="peer-task"
              required
              value={task}
              onChange={(e) => setTask(e.target.value)}
            >
              <option value="">Choose a task</option>
              {tasks.items.map((row) => (
                <option key={row.id} value={row.id}>
                  {row.title}
                </option>
              ))}
            </select>
          </label>
          <label htmlFor="peer-question">
            Exact question
            <textarea
              id="peer-question"
              required
              maxLength={4000}
              value={question}
              onChange={(e) => setQuestion(e.target.value)}
            />
          </label>
          <button
            className="primary-button"
            disabled={!task || !question.trim()}
          >
            Prepare question for review
          </button>
        </fieldset>
        {error && <p role="alert">{error}</p>}
        {uncertain && (
          <button
            type="button"
            className="secondary-button"
            onClick={() => {
              refresh();
              setCheckedStatus(true);
              setError(
                "Inspect the refreshed request list before starting a new request.",
              );
            }}
          >
            Check request status
          </button>
        )}
        {uncertain && checkedStatus && (
          <button
            type="button"
            className="secondary-button"
            onClick={() => {
              setUncertain(false);
              setCheckedStatus(false);
              setQuestion("");
              setError("");
            }}
          >
            I checked the request list; start a new draft
          </button>
        )}
      </form>
    </details>
  );
}
