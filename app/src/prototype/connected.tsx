import { ContextSummary } from "./context-summary";
import { LiveChat } from "./live-chat";
import { LiveApprovals } from "./live-approvals";
import { LiveMemory } from "./live-memory";
import { LiveWatches } from "./live-watches";
import { TaskEvidence } from "./task-evidence";
import { ThreadWorkspace } from "./thread-workspace";
import { useCallback, useEffect, useRef, useState } from "react";
import {
  operate,
  request,
  isLoopbackHost,
  streamChat,
  ChatProviderError,
  type Agent,
  type Task,
  type Thread,
  type Message,
  type Activity,
} from "../api";
import { Avatar, Icon } from "./shared";
import {
  LiveSettings,
  useServerSettings,
  type GlobalSettings,
  type RuntimeStatus,
  type AgentSettings,
} from "./live-settings";
import { TaskCalendar } from "./calendar";
import { TaskEditor, type WorkspaceTask } from "./tasks";

type PageResult = { items: Message[]; next_cursor: number | null };
const displayOwner = (owner: Agent) => (owner === "janet" ? "Janet" : "Harley");
function deadline(date: string) {
  if (!date) return null;
  const offset = new Intl.DateTimeFormat("en", {
    timeZone: "America/Edmonton",
    timeZoneName: "longOffset",
  })
    .formatToParts(new Date(`${date}T12:00:00Z`))
    .find((p) => p.type === "timeZoneName")!
    .value.replace("GMT", "");
  return `${date}T23:59:59${offset}`;
}
function dateOnly(value: string | null) {
  if (!value) return "";
  const parts = new Intl.DateTimeFormat("en-CA", {
    timeZone: "America/Edmonton",
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
  }).formatToParts(new Date(value));
  return ["year", "month", "day"]
    .map((type) => parts.find((p) => p.type === type)!.value)
    .join("-");
}
export function ConnectedWorkspace({
  theme: initialTheme,
  onOpenSample,
}: {
  theme: "light" | "dark";
  onOpenSample: () => void;
}) {
  const [theme, setTheme] = useState(initialTheme);
  const local =
    isLoopbackHost(window.location.hostname) &&
    import.meta.env.VITE_GATEWAY_PAIRED !== "true";
  const [token, setToken] = useState("");
  const [input, setInput] = useState("");
  const [disconnected, setDisconnected] = useState(false);
  return (
    <div className="prototype connected-workspace" data-theme={theme}>
      <a className="skip-link" href="#live-content">
        Skip to workspace
      </a>
      {disconnected || (!token && !local) ? (
        <main className="connection-form" id="live-content" tabIndex={-1}>
          <h1>Connect your workspace</h1>
          <p>
            Real chat, tasks, and read-only activity use the same-origin paired
            gateway. Vault enrollment remains unavailable.
          </p>
          <form
            className="workspace-form"
            onSubmit={(e) => {
              e.preventDefault();
              if (local) setDisconnected(false);
              else if (input.trim()) {
                setToken(input.trim());
                setInput("");
                setDisconnected(false);
              }
            }}
          >
            {!local && (
              <label>
                Paired gateway token
                <input
                  type="password"
                  autoComplete="off"
                  value={input}
                  onChange={(e) => setInput(e.target.value)}
                  required
                />
              </label>
            )}
            <button className="primary-button">Connect</button>
          </form>
          <button className="secondary-button" onClick={onOpenSample}>
            Open sample workspace
          </button>
        </main>
      ) : (
        <ConnectedSession
          token={token}
          onTheme={setTheme}
          onDisconnect={() => {
            setToken("");
            setInput("");
            setDisconnected(true);
          }}
        />
      )}
    </div>
  );
}
function ConnectedSession({
  token,
  onTheme,
  onDisconnect,
}: {
  token: string;
  onTheme: (theme: "light" | "dark") => void;
  onDisconnect: () => void;
}) {
  const [owner, setOwner] = useState<Agent>("janet");
  const [agentSetup, setAgentSetup] = useState(false);
  return (
    <LiveAgent
      key={owner}
      owner={owner}
      token={token}
      onOwner={(agent, settings = false) => {
        setAgentSetup(settings);
        setOwner(agent);
      }}
      agentSetup={agentSetup}
      onTheme={onTheme}
      onDisconnect={onDisconnect}
    />
  );
}
function LiveAgent({
  owner,
  token,
  onOwner,
  onTheme,
  agentSetup,
  onDisconnect,
}: {
  owner: Agent;
  agentSetup: boolean;
  token: string;
  onOwner: (owner: Agent, settings?: boolean) => void;
  onTheme: (theme: "light" | "dark") => void;
  onDisconnect: () => void;
}) {
  const [view, setView] = useState(agentSetup ? "Agent settings" : "Chat");
  const global = useServerSettings<GlobalSettings>(
    token,
    "/api/settings/global",
  );
  const [settingsDirty, setSettingsDirty] = useState(false);
  const [appearanceBusy, setAppearanceBusy] = useState(false);
  const [appearanceError, setAppearanceError] = useState("");
  const [runtimeStatus, setRuntimeStatus] = useState<RuntimeStatus>();
  const [configurationError, setConfigurationError] = useState("");
  const [defaultProfile, setDefaultProfile] = useState("normal");
  const dirtyChange = useCallback(
    (dirty: boolean) => setSettingsDirty(dirty),
    [],
  );
  useEffect(() => {
    if (global.data) onTheme(global.data.value.preferences.theme);
  }, [global.data, onTheme]);
  useEffect(() => {
    const controller = new AbortController();
    setConfigurationError("");
    request<RuntimeStatus>(
      token,
      "/api/runtime/status",
      undefined,
      controller.signal,
    )
      .then((value) => {
        if (!controller.signal.aborted) setRuntimeStatus(value);
      })
      .catch(() => {
        if (!controller.signal.aborted)
          setConfigurationError(
            "Runtime status could not be loaded. Open Settings or refresh to retry.",
          );
      });
    request<{ revision: number; value: AgentSettings }>(
      token,
      `/api/agents/${owner}/settings`,
      undefined,
      controller.signal,
    )
      .then((result) => {
        if (!controller.signal.aborted)
          setDefaultProfile(result.value.default_profile);
      })
      .catch(() => {
        if (!controller.signal.aborted)
          setConfigurationError(
            "Agent configuration could not be loaded. Open agent settings to retry.",
          );
      });
    return () => controller.abort();
  }, [token, owner, view]);
  function canLeave() {
    return !settingsDirty || confirm("Discard unsaved settings changes?");
  }

  const [tasks, setTasks] = useState<Task[]>([]);
  const [taskCursor, setTaskCursor] = useState<number | null>(null);
  const [threadCursor, setThreadCursor] = useState<number | null>(null);
  const [paging, setPaging] = useState(false);
  const [pageError, setPageError] = useState("");
  const pageRequest = useRef<AbortController | null>(null);
  const [threads, setThreads] = useState<Thread[]>([]);
  const [messages, setMessages] = useState<Message[]>([]);
  const [activity, setActivity] = useState<Activity>({
    events: [],
    workers: [],
  });
  const [contextError, setContextError] = useState("");
  const [contextLoading, setContextLoading] = useState(false);
  const [thread, setThread] = useState("");
  const [mobileNav, setMobileNav] = useState(false);
  const content = useRef<HTMLElement>(null);
  const [compactNavigation, setCompactNavigation] = useState(
    () => matchMedia("(max-width: 900px)").matches,
  );
  useEffect(() => {
    const media = matchMedia("(max-width: 900px)");
    const change = () => {
      setCompactNavigation(media.matches);
      if (!media.matches) setMobileNav(false);
    };
    media.addEventListener("change", change);
    return () => media.removeEventListener("change", change);
  }, []);
  useEffect(() => {
    content.current?.scrollTo({ top: 0 });
  }, [view, thread]);
  const navigation = useRef<HTMLElement>(null);
  const menuButton = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!mobileNav) return;
    const panel = navigation.current;
    const items = () =>
      Array.from(
        panel?.querySelectorAll<HTMLElement>(
          "button:not(:disabled), a[href]",
        ) ?? [],
      ).filter((item) => item.getClientRects().length);
    items()[0]?.focus();
    const trap = (event: KeyboardEvent) => {
      if (event.key !== "Tab") return;
      const controls = items();
      const first = controls[0],
        last = controls[controls.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault();
        last?.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault();
        first?.focus();
      }
    };
    panel?.addEventListener("keydown", trap);
    return () => {
      panel?.removeEventListener("keydown", trap);
      menuButton.current?.focus();
    };
  }, [mobileNav]);
  const [newThread, setNewThread] = useState(false);
  const [threadPanel, setThreadPanel] = useState<"conversation" | "workspace">(
    "conversation",
  );
  const [attachmentIntent, setAttachmentIntent] = useState(false);
  const [threadTitle, setThreadTitle] = useState("");
  const [threadError, setThreadError] = useState("");
  const [threadUncertain, setThreadUncertain] = useState(false);
  const [cursor, setCursor] = useState<number | null>(null);
  const [historyBusy, setHistoryBusy] = useState(false);
  const [historyError, setHistoryError] = useState("");
  const [needsRefresh, setNeedsRefresh] = useState(false);
  const [taskRefreshError, setTaskRefreshError] = useState(false);
  const [chatError, setChatError] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");
  const [draft, setDraft] = useState("");
  const focusAfterSend = useRef(false);
  useEffect(() => {
    if (!loading && focusAfterSend.current && view === "Chat") {
      focusAfterSend.current = false;
      if (!matchMedia("(pointer: coarse)").matches)
        document.getElementById("live-message")?.focus({ preventScroll: true });
    }
  }, [loading, view]);
  const [profile, setProfile] = useState("default");
  const [partial, setPartial] = useState("");
  const [busy, setBusy] = useState(false);
  const [reload, setReload] = useState(0);
  const [editing, setEditing] = useState<WorkspaceTask | null | undefined>(
    undefined,
  );
  const [mutationBusy, setMutationBusy] = useState(false);
  const [mutationError, setMutationError] = useState("");
  const [uncertainCreate, setUncertainCreate] = useState(false);
  const [pendingCreated, setPendingCreated] = useState<string | null>(null);
  const [taskStatus, setTaskStatus] = useState("active");
  const active = useRef<AbortController | null>(null);
  const mounted = useRef(true);
  useEffect(() => {
    mounted.current = true;
    return () => {
      mounted.current = false;
      active.current?.abort();
      pageRequest.current?.abort();
    };
  }, []);
  useEffect(() => {
    const controller = new AbortController();
    pageRequest.current?.abort();
    setPaging(false);
    setPageError("");
    setLoading(true);
    setError("");
    setMessages([]);
    setHistoryError("");
    Promise.all([
      operate<{ items: Task[]; next_cursor: number | null }>(
        token,
        owner,
        "task.page",
        {},
        controller.signal,
      ),
      operate<{ items: Thread[]; next_cursor: number | null }>(
        token,
        owner,
        "thread.page",
        {},
        controller.signal,
      ),
      operate<PageResult>(
        token,
        owner,
        "chat.page",
        { thread_id: thread || null },
        controller.signal,
      ),
    ])
      .then(async ([taskRows, threadRows, history]) => {
        if (thread && !threadRows.items.some((row) => row.id === thread)) {
          const selected = await operate<Thread>(
            token,
            owner,
            "thread.get",
            { thread_id: thread },
            controller.signal,
          );
          if (selected.id !== thread)
            throw new Error("Invalid selected thread");
          threadRows.items.push(selected);
        }
        if (controller.signal.aborted) return;
        setTasks(taskRows.items);
        setTaskRefreshError(false);
        setTaskCursor(taskRows.next_cursor);
        setThreads(threadRows.items);
        setThreadCursor(threadRows.next_cursor);
        setMessages(history.items.slice().reverse());
        setCursor(history.next_cursor);
        setNeedsRefresh(false);
        setChatError("");
        setPartial("");
      })
      .catch(() => {
        if (!controller.signal.aborted)
          setError(
            isLoopbackHost(window.location.hostname)
              ? "Could not load this local workspace. Start the loopback gateway with --local-ui on port 8080, then retry."
              : "Could not load this workspace. Check the gateway connection and paired token, then retry.",
          );
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [token, owner, thread, reload]);
  async function loadMoreRows(kind: "task" | "thread") {
    const before = kind === "task" ? taskCursor : threadCursor;
    if (paging || before === null) return;
    const controller = new AbortController();
    pageRequest.current = controller;
    setPaging(true);
    setPageError("");
    try {
      const page = await operate<{
        items: (Task | Thread)[];
        next_cursor: number | null;
      }>(
        token,
        owner,
        `${kind}.page`,
        { page: { before, limit: 50 } },
        controller.signal,
      );
      if (controller.signal.aborted) return;
      if (kind === "task") {
        setTasks((old) => [
          ...old,
          ...(page.items as Task[]).filter(
            (row) => !old.some((item) => item.id === row.id),
          ),
        ]);
        setTaskCursor(page.next_cursor);
      } else {
        setThreads((old) => [
          ...old,
          ...(page.items as Thread[]).filter(
            (row) => !old.some((item) => item.id === row.id),
          ),
        ]);
        setThreadCursor(page.next_cursor);
      }
    } catch {
      if (!controller.signal.aborted)
        setPageError("Could not load more records. Try again.");
    } finally {
      if (!controller.signal.aborted) setPaging(false);
    }
  }
  useEffect(() => {
    const controller = new AbortController();
    setContextLoading(true);
    setContextError("");
    setActivity({ events: [], workers: [] });
    operate<Activity>(token, owner, "activity.list", {}, controller.signal)
      .then((value) => {
        if (!controller.signal.aborted) setActivity(value);
      })
      .catch(() => {
        if (!controller.signal.aborted)
          setContextError(
            "Activity could not be loaded. Refresh to try again. Chat and tasks remain available.",
          );
      })
      .finally(() => {
        if (!controller.signal.aborted) setContextLoading(false);
      });
    return () => controller.abort();
  }, [token, owner, thread, reload]);
  const taskRows = tasks
    .filter(
      (task) =>
        taskStatus === "all" ||
        (taskStatus === "completed"
          ? task.status === "completed"
          : !["completed", "cancelled"].includes(task.status)),
    )
    .sort(
      (a, b) =>
        ({ urgent: 0, high: 1, normal: 2, low: 3 })[a.priority] -
        { urgent: 0, high: 1, normal: 2, low: 3 }[b.priority],
    );
  const workspaceTask = (task: Task): WorkspaceTask => ({
    id: task.id,
    title: task.title,
    description: task.description,
    dueDate: dateOnly(task.due_at),
    priority: task.priority,
    thread: task.thread_id,
    steps: ["Complete task"],
    done: [task.status === "completed"],
  });
  async function saveTask(value: WorkspaceTask) {
    if (uncertainCreate) return;
    setMutationBusy(true);
    setMutationError("");
    let created = pendingCreated;
    try {
      const threadId = value.thread;
      if (!editing && !created) {
        const task = await operate<Task>(token, owner, "task.create", {
          title: value.title,
          description: value.description || value.title,
          thread_id: threadId,
        });
        created = task.id;
        if (mounted.current) setPendingCreated(created);
      }
      const previous = tasks.find((t) => t.id === editing?.id);
      await operate(token, owner, "task.edit", {
        task_id: editing?.id ?? created,
        title: value.title,
        description: value.description || value.title,
        thread_id: threadId,
        priority: value.priority,
        due_at:
          previous && dateOnly(previous.due_at) === value.dueDate
            ? previous.due_at
            : deadline(value.dueDate),
      });
      if (mounted.current) {
        setEditing(undefined);
        setPendingCreated(null);
        setReload((n) => n + 1);
      }
    } catch {
      if (mounted.current) {
        if (!editing && !created) setUncertainCreate(true);
        setMutationError(
          created
            ? "Task created, but its details were not confirmed. Retry saving this task; a duplicate will not be created in this session."
            : "Save was not confirmed. Refresh tasks before trying again; the server may have saved the change.",
        );
      }
    } finally {
      if (mounted.current) setMutationBusy(false);
    }
  }
  async function send(e: React.FormEvent) {
    e.preventDefault();
    if (!draft.trim() || busy || needsRefresh) return;
    if (new TextEncoder().encode(draft).length > 32000) {
      setChatError(
        "Message exceeds 32,000 bytes. Shorten the text before sending.",
      );
      return;
    }
    const controller = new AbortController();
    active.current = controller;
    setBusy(true);
    setChatError("");
    setPartial("");
    setTaskRefreshError(false);
    try {
      await streamChat(
        token,
        owner,
        {
          message: draft,
          ...(profile !== "default" ? { profile } : {}),
          ...(thread ? { thread_id: thread } : {}),
        },
        controller.signal,
        (text) => {
          if (mounted.current) setPartial((p) => p + text);
        },
      );
      if (mounted.current) {
        setDraft("");
        focusAfterSend.current = true;
        setReload((n) => n + 1);
      }
    } catch (error) {
      if (mounted.current) {
        setNeedsRefresh(true);
        setChatError(
          error instanceof ChatProviderError
            ? `${error.message} Refresh history before retrying; earlier internal operations may have completed.`
            : "Response was not confirmed. Refresh history before resending; submitted work may still finish.",
        );
        // Internal actions can finish even when the final chat stream fails.
        // Refresh read-only state without replaying the submitted message.
        try {
          const rows = await operate<{
            items: Task[];
            next_cursor: number | null;
          }>(token, owner, "task.page", {}, controller.signal);
          if (mounted.current) {
            setTasks(rows.items);
            setTaskCursor(rows.next_cursor);
          }
        } catch {
          if (mounted.current) setTaskRefreshError(true);
        }
      }
    } finally {
      if (mounted.current) {
        setBusy(false);
      }
    }
  }
  async function toggleAppearance() {
    if (!global.data || appearanceBusy) return;
    setAppearanceBusy(true);
    setAppearanceError("");
    try {
      const saved = await request<NonNullable<typeof global.data>>(
        token,
        "/api/settings/global",
        {
          revision: global.data.revision,
          value: {
            ...global.data.value,
            preferences: {
              ...global.data.value.preferences,
              theme:
                global.data.value.preferences.theme === "dark"
                  ? "light"
                  : "dark",
            },
          },
        },
      );
      if (mounted.current) global.setData(saved);
    } catch {
      if (mounted.current) {
        setAppearanceError(
          "Appearance could not be saved. Settings may have changed elsewhere. Open App settings to review and try again.",
        );
        global.retry();
      }
    } finally {
      if (mounted.current) setAppearanceBusy(false);
    }
  }
  function navigate(page: string) {
    if (page === view) {
      setMobileNav(false);
      return;
    }
    if (!canLeave()) return;
    setSettingsDirty(false);
    setView(page);
    setMobileNav(false);
  }
  function openThread(id: string) {
    if (!canLeave()) return;
    setSettingsDirty(false);
    setThread(id);
    setThreadPanel("conversation");
    setDraft("");
    setView("Chat");
    setMobileNav(false);
  }
  async function createThread(e?: React.FormEvent) {
    if (!e) {
      if (newThread && threadUncertain) return;
      setAttachmentIntent(false);
      setThreadTitle("");
      setThreadError("");
      setThreadUncertain(false);
      setNewThread(true);
      return;
    }
    e.preventDefault();
    if (mutationBusy || threadUncertain) return;
    setMutationBusy(true);
    setThreadError("");
    try {
      const result = await operate<Thread>(token, owner, "thread.create", {
        title: threadTitle.trim() || "New conversation",
      });
      if (mounted.current) {
        setNewThread(false);
        setThreadTitle("");
        openThread(result.id);
        if (attachmentIntent) {
          setDraft(draft);
          setThreadPanel("workspace");
          setAttachmentIntent(false);
        }
        setReload((n) => n + 1);
      }
    } catch {
      if (mounted.current) {
        setThreadUncertain(true);
        setThreadError(
          "Creation was not confirmed. Close this form and refresh Focus Threads before trying again; the thread may already exist.",
        );
      }
    } finally {
      if (mounted.current) setMutationBusy(false);
    }
  }
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === ",") {
        e.preventDefault();
        if (!document.querySelector("dialog[open]")) navigate("Settings");
      }
      if (e.key === "Escape") {
        setMobileNav(false);
        if (!mutationBusy) setNewThread(false);
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [mutationBusy, settingsDirty, view]);
  const currentThread = threads.find((t) => t.id === thread);
  return (
    <div className="workspace-shell live-workspace-shell">
      {mobileNav && (
        <button
          className="nav-scrim"
          aria-label="Close navigation"
          onClick={() => setMobileNav(false)}
        />
      )}
      <aside
        ref={navigation}
        className={`sidebar ${mobileNav ? "nav-open" : ""}`}
        aria-label="Workspace navigation"
        role={mobileNav ? "dialog" : undefined}
        aria-modal={mobileNav || undefined}
        inert={compactNavigation && !mobileNav}
        aria-hidden={(compactNavigation && !mobileNav) || undefined}
      >
        <div className="prototype-brand">
          <img className="claw-mark" src="/brand/mark-light.svg" alt="" />
          <strong>Clawberry</strong>
          <span className="brand-tag">live</span>
        </div>
        <section className="sidebar-agents" aria-label="Connected agents">
          <div className="sidebar-agents-heading">
            <h2>AGENTS</h2>
          </div>
          <div className="agent-switcher">
            {(["janet", "harley"] as Agent[]).map((agent) => (
              <div className="agent-branch" key={agent}>
                <button
                  key={agent}
                  className={`agent-choice ${owner === agent ? "selected" : ""}`}
                  aria-label={displayOwner(agent)}
                  aria-pressed={owner === agent}
                  disabled={
                    busy || mutationBusy || historyBusy || appearanceBusy
                  }
                  onClick={() => {
                    if (agent === owner) navigate("Chat");
                    else if (canLeave()) onOwner(agent);
                  }}
                >
                  <Avatar agent={displayOwner(agent)} />
                  <span>
                    <strong>{displayOwner(agent)}</strong>
                    <small>
                      {agent === "janet" ? "Work agent" : "Personal agent"}
                    </small>
                  </span>
                </button>
                <button
                  className="agent-settings-button"
                  aria-label={`${displayOwner(agent)} settings`}
                  title={`${displayOwner(agent)} settings: identity, memory and skills`}
                  aria-pressed={agent === owner && view === "Agent settings"}
                  disabled={busy || mutationBusy || appearanceBusy}
                  onClick={() => {
                    if (!canLeave()) return;
                    if (agent !== owner) {
                      onOwner(agent, true);
                    } else navigate("Agent settings");
                  }}
                >
                  <Icon name="gear" />
                </button>
              </div>
            ))}
          </div>
        </section>
        <div className="nav-divider" />
        <nav aria-label="Connected workspace">
          {(
            [
              ["Chat", "chat"],
              ["Focus Threads", "thread"],
              ["Tasks", "tasks"],
              ["Files", "file"],
              ["Calendar", "calendar"],
              ["Activity", "activity"],
              ["Approvals", "shield"],
              ["Memory", "memory"],
            ] as const
          ).map(([page, icon]) => (
            <button
              key={page}
              className={view === page ? "active" : ""}
              aria-current={view === page ? "page" : undefined}
              disabled={busy || mutationBusy || historyBusy || appearanceBusy}
              onClick={() => navigate(page)}
            >
              <Icon name={icon} />
              {page}
            </button>
          ))}
        </nav>
        <div className="thread-nav">
          <div className="thread-label">
            FOCUS THREADS
            <button
              aria-label="New Focus Thread"
              disabled={busy || mutationBusy || loading || !!error}
              onClick={() => {
                setMobileNav(false);
                void createThread();
              }}
            >
              <Icon name="plus" />
            </button>
          </div>
          <button
            className={!thread ? "current-thread" : ""}
            disabled={busy || mutationBusy || historyBusy || appearanceBusy}
            onClick={() => openThread("")}
          >
            <Icon name="chat" />
            Main conversation
          </button>
          {threads.map((t) => (
            <button
              className={thread === t.id ? "current-thread" : ""}
              key={t.id}
              disabled={busy || mutationBusy || historyBusy || appearanceBusy}
              onClick={() => openThread(t.id)}
            >
              <span>#</span>
              {t.title}
            </button>
          ))}
        </div>
        <div className="sidebar-bottom">
          <div className="sandbox-note">
            <span className="sample-dot" />
            Connected gateway<small>Private to {displayOwner(owner)}</small>
          </div>
          <button
            className={`profile live-app-settings ${view === "Settings" ? "selected" : ""}`}
            aria-label="App settings"
            aria-current={view === "Settings" ? "page" : undefined}
            title="App settings (⌘ / Ctrl + ,)"
            onClick={() => navigate("Settings")}
            disabled={busy || mutationBusy || historyBusy || appearanceBusy}
          >
            <span className="profile-avatar">M</span>
            <span>
              <strong>App settings</strong>
              <small>User · Global preferences</small>
            </span>
            <Icon name="gear" />
          </button>
        </div>
      </aside>
      <main
        ref={content}
        className={`connected-content ${view === "Chat" ? "live-chat-page" : ""}`}
        id="live-content"
        tabIndex={-1}
      >
        <header className="connected-title">
          <button
            className="icon-button live-nav-toggle"
            ref={menuButton}
            aria-label="Open navigation"
            aria-expanded={mobileNav}
            onClick={() => setMobileNav(true)}
          >
            <Icon name="layers" />
          </button>
          {view === "Settings" ? (
            <span className="live-settings-avatar">
              <Icon name="gear" />
            </span>
          ) : (
            <Avatar agent={displayOwner(owner)} />
          )}
          <div className="live-heading">
            <h1>
              {view === "Chat"
                ? (currentThread?.title ?? displayOwner(owner))
                : view === "Settings"
                  ? "App settings"
                  : view === "Agent settings"
                    ? `${displayOwner(owner)} settings`
                    : view}
            </h1>
            <small>
              {view === "Settings"
                ? "Workspace preferences and model routing"
                : view === "Agent settings"
                  ? `${displayOwner(owner)}’s identity, memory and skills`
                  : `Live workspace · ${displayOwner(owner)}`}
            </small>
          </div>
          <div className="live-header-actions">
            {!["Settings", "Agent settings", "Approvals", "Memory"].includes(
              view,
            ) && (
              <button
                type="button"
                className="icon-button live-refresh"
                aria-label="Refresh history and data"
                title="Refresh history and data"
                disabled={busy || mutationBusy || historyBusy || loading}
                onClick={() => setReload((n) => n + 1)}
              >
                <Icon name="refresh" />
              </button>
            )}
            {view !== "Settings" && (
              <button
                type="button"
                className="icon-button"
                disabled={!global.data || appearanceBusy || settingsDirty}
                aria-busy={appearanceBusy}
                aria-label={`Switch to ${global.data?.value.preferences.theme === "dark" ? "light" : "dark"} theme`}
                title="Change workspace appearance"
                onClick={() => void toggleAppearance()}
              >
                <Icon
                  name={
                    global.data?.value.preferences.theme === "dark"
                      ? "sun"
                      : "moon"
                  }
                />
              </button>
            )}
            <button
              className="secondary-button live-disconnect"
              aria-label="Disconnect"
              title="Disconnect"
              onClick={() => {
                if (canLeave()) onDisconnect();
              }}
              disabled={busy || mutationBusy || historyBusy || appearanceBusy}
            >
              <Icon name="disconnect" />
              <span>Disconnect</span>
            </button>
          </div>
        </header>
        <div
          className={`live-page-body ${view === "Settings" || view === "Agent settings" ? "live-settings-body" : ""}`}
        >
          {appearanceError && <p role="alert">{appearanceError}</p>}
          {view === "Settings" || view === "Agent settings" ? (
            <LiveSettings
              key={view}
              token={token}
              owner={view === "Agent settings" ? owner : null}
              global={global.data}
              onGlobal={global.setData}
              onDirty={dirtyChange}
              onChat={() => navigate("Chat")}
              onApprovals={() => navigate("Approvals")}
              status={runtimeStatus}
            />
          ) : view === "Approvals" ? (
            <LiveApprovals key={owner} token={token} owner={owner} />
          ) : view === "Memory" ? (
            <LiveMemory key={owner} token={token} owner={owner} />
          ) : (
            <>
              {configurationError && <p role="status">{configurationError}</p>}
              {runtimeStatus && !runtimeStatus.credential_configured && (
                <p className="live-setup-notice" role="status">
                  Claude is not configured. Add ANTHROPIC_API_KEY to .env.local
                  and restart the local runtime. Saved tasks and settings are
                  available.
                </p>
              )}

              {newThread && (
                <form
                  className="workspace-form live-thread-form"
                  aria-label="Create Focus Thread"
                  onSubmit={createThread}
                >
                  {attachmentIntent && (
                    <p>
                      Files stay in a Focus Thread with their conversation and
                      evidence. Create a thread to upload files; your message
                      draft will come with you. You can also close this form and
                      open an existing thread.
                    </p>
                  )}
                  <label>
                    Thread title
                    <input
                      autoFocus
                      required
                      maxLength={200}
                      value={threadTitle}
                      onChange={(e) => setThreadTitle(e.target.value)}
                      disabled={mutationBusy}
                    />
                  </label>
                  {threadError && <p role="alert">{threadError}</p>}
                  <div className="task-controls">
                    <button
                      className="primary-button"
                      disabled={
                        mutationBusy || threadUncertain || !threadTitle.trim()
                      }
                    >
                      Create thread
                    </button>
                    <button
                      type="button"
                      className="secondary-button"
                      disabled={mutationBusy}
                      onClick={() => {
                        setNewThread(false);
                        setAttachmentIntent(false);
                        setReload((n) => n + 1);
                      }}
                    >
                      Close
                    </button>
                  </div>
                </form>
              )}
              {error && <p role="alert">{error}</p>}
              {contextError && <p role="status">{contextError}</p>}
              {pageError && <p role="alert">{pageError}</p>}
              {taskCursor !== null && (
                <p role="status">
                  Showing {tasks.length} loaded tasks. Older tasks and deadlines
                  may be outside this view; open Tasks and load more.
                </p>
              )}
              {threadCursor !== null && (
                <p role="status">
                  Older Focus Threads are available in Focus Threads → Load more
                  threads.
                </p>
              )}
              {view === "Tasks" && taskCursor !== null && (
                <button
                  className="secondary-button"
                  disabled={loading || paging}
                  onClick={() => void loadMoreRows("task")}
                >
                  Load more tasks
                </button>
              )}
              {view === "Focus Threads" && threadCursor !== null && (
                <button
                  className="secondary-button"
                  disabled={loading || paging}
                  onClick={() => void loadMoreRows("thread")}
                >
                  Load more threads
                </button>
              )}
              {loading ? (
                <p role="status">Loading {displayOwner(owner)}’s workspace…</p>
              ) : error ? null : (
                <>
                  {view === "Chat" && thread && (
                    <div
                      className="thread-tabs"
                      role="tablist"
                      aria-label="Focus Thread views"
                      onKeyDown={(event) => {
                        if (
                          !["ArrowLeft", "ArrowRight", "Home", "End"].includes(
                            event.key,
                          )
                        )
                          return;
                        event.preventDefault();
                        const next =
                          event.key === "Home"
                            ? "conversation"
                            : event.key === "End"
                              ? "workspace"
                              : threadPanel === "conversation"
                                ? "workspace"
                                : "conversation";
                        setThreadPanel(next);
                        document.getElementById(`thread-${next}-tab`)?.focus();
                      }}
                    >
                      <button
                        id="thread-conversation-tab"
                        role="tab"
                        aria-selected={threadPanel === "conversation"}
                        tabIndex={threadPanel === "conversation" ? 0 : -1}
                        aria-controls="thread-conversation-panel"
                        onClick={() => setThreadPanel("conversation")}
                      >
                        Conversation
                      </button>
                      <button
                        id="thread-workspace-tab"
                        role="tab"
                        aria-selected={threadPanel === "workspace"}
                        tabIndex={threadPanel === "workspace" ? 0 : -1}
                        aria-controls="thread-workspace-panel"
                        onClick={() => setThreadPanel("workspace")}
                      >
                        Files & evidence
                      </button>
                    </div>
                  )}
                  {view === "Chat" &&
                    (!thread || threadPanel === "conversation") && (
                      <div
                        className="thread-conversation-panel"
                        id="thread-conversation-panel"
                        role={thread ? "tabpanel" : undefined}
                        aria-labelledby={
                          thread ? "thread-conversation-tab" : undefined
                        }
                      >
                        <LiveChat
                          owner={owner}
                          openThread={(id) => {
                            if (threads.some((t) => t.id === id))
                              openThread(id);
                            else
                              setChatError(
                                "This Focus Thread is unavailable in this agent’s workspace. Refresh the thread list to check access.",
                              );
                          }}
                          onAttach={() => {
                            if (thread) setThreadPanel("workspace");
                            else {
                              setAttachmentIntent(true);
                              setThreadUncertain(false);
                              setThreadError("");
                              setThreadTitle("");
                              setNewThread(true);
                            }
                          }}
                          messages={messages}
                          draft={draft}
                          onDraft={setDraft}
                          busy={busy}
                          partial={partial}
                          needsRefresh={needsRefresh}
                          error={chatError}
                          onRefresh={() => setReload((n) => n + 1)}
                          onSend={send}
                          onStop={() => active.current?.abort()}
                          profile={profile}
                          onProfile={setProfile}
                          defaultProfile={defaultProfile}
                          model={
                            global.data?.value.models?.profiles?.[
                              (profile === "default"
                                ? defaultProfile
                                : profile) as "fast" | "normal" | "deep"
                            ]?.model
                          }
                          historyBusy={historyBusy}
                          historyError={historyError}
                          hasEarlier={cursor !== null}
                          onEarlier={async () => {
                            setHistoryBusy(true);
                            setHistoryError("");
                            try {
                              const previous = await operate<PageResult>(
                                token,
                                owner,
                                "chat.page",
                                {
                                  thread_id: thread || null,
                                  page: { before: cursor },
                                },
                              );
                              if (mounted.current) {
                                setMessages((old) => [
                                  ...previous.items.slice().reverse(),
                                  ...old,
                                ]);
                                setCursor(previous.next_cursor);
                              }
                            } catch {
                              if (mounted.current)
                                setHistoryError(
                                  "Could not load earlier messages. Try again.",
                                );
                            } finally {
                              if (mounted.current) setHistoryBusy(false);
                            }
                          }}
                        />
                      </div>
                    )}
                  {view === "Files" && (
                    <section aria-label="Conversation files">
                      <h2>Files</h2>
                      <p>
                        Choose a Focus Thread to upload files, download
                        attachments, and inspect source evidence.
                      </p>
                      {!threads.length && (
                        <p>Create a Focus Thread to attach your first file.</p>
                      )}
                      <div
                        className="live-file-folders"
                        aria-label="Project file folders"
                      >
                        {threads.map((item) => (
                          <button
                            key={item.id}
                            type="button"
                            className={
                              thread === item.id
                                ? "live-folder selected"
                                : "live-folder"
                            }
                            aria-pressed={thread === item.id}
                            onClick={() => setThread(item.id)}
                          >
                            <Icon name="file" />
                            <span>{item.title}</span>
                            <small>Project folder</small>
                          </button>
                        ))}
                      </div>
                      {!thread && (
                        <p>
                          Select a project folder to view its existing files or
                          upload new ones.
                        </p>
                      )}
                      {thread && (
                        <ThreadWorkspace
                          key={`${owner}:${thread}:files`}
                          token={token}
                          owner={owner}
                          thread={thread}
                          tasks={tasks}
                          summary={currentThread?.summary ?? ""}
                          refresh={reload}
                          manageTasks={() => navigate("Tasks")}
                          filesOnly
                        />
                      )}
                    </section>
                  )}
                  {view === "Focus Threads" && (
                    <section aria-label="Owned Focus Threads">
                      <h2>Your Focus Threads</h2>
                      <button
                        className="primary-button"
                        onClick={() => {
                          void createThread();
                        }}
                      >
                        New thread
                      </button>
                      {!threads.length && (
                        <p>
                          No Focus Threads yet. Create one for a project or
                          investigation.
                        </p>
                      )}
                      <div className="task-grid">
                        {threads.map((t) => (
                          <article className="task-card" key={t.id}>
                            <h3>{t.title}</h3>
                            <p>{t.summary || "No summary recorded yet."}</p>
                            <small>
                              {
                                tasks.filter((task) => task.thread_id === t.id)
                                  .length
                              }{" "}
                              loaded tasks
                            </small>
                            <button
                              className="secondary-button"
                              onClick={() => openThread(t.id)}
                            >
                              Open {t.title}
                            </button>
                          </article>
                        ))}
                      </div>
                    </section>
                  )}
                  {view === "Chat" && thread && (
                    <div
                      id="thread-workspace-panel"
                      className="thread-evidence-panel"
                      hidden={threadPanel !== "workspace"}
                      role="tabpanel"
                      aria-labelledby="thread-workspace-tab"
                    >
                      <ThreadWorkspace
                        key={`${owner}:${thread}`}
                        token={token}
                        owner={owner}
                        thread={thread}
                        tasks={tasks}
                        summary={currentThread?.summary ?? ""}
                        refresh={reload}
                        manageTasks={() => navigate("Tasks")}
                        onSummarySaved={() => setReload((n) => n + 1)}
                        onIncludeText={(text) => {
                          const next = draft ? `${draft}\n\n${text}` : text;
                          if (
                            next.length > 16000 ||
                            new TextEncoder().encode(next).length > 32000
                          ) {
                            setChatError(
                              "The file and current draft exceed the message limit. Shorten your draft or include a smaller excerpt.",
                            );
                          } else setDraft(next);
                          setThreadPanel("conversation");
                        }}
                      />
                    </div>
                  )}
                  {view === "Tasks" && (
                    <>
                      <p>
                        {displayOwner(owner)}’s tasks across all Focus Threads.
                      </p>
                      <div className="task-controls">
                        <button
                          className="primary-button"
                          onClick={() => {
                            setPendingCreated(null);
                            setUncertainCreate(false);
                            setMutationError("");
                            setEditing(null);
                          }}
                        >
                          New task
                        </button>
                        <label>
                          Show{" "}
                          <select
                            value={taskStatus}
                            onChange={(e) => setTaskStatus(e.target.value)}
                          >
                            <option value="active">Active</option>
                            <option value="completed">Completed</option>
                            <option value="all">All tasks</option>
                          </select>
                        </label>
                      </div>
                      <div className="task-grid">
                        {taskRows.map((task) => (
                          <article className="task-card" key={task.id}>
                            <h3>{task.title}</h3>
                            <p>{task.description}</p>
                            <p>
                              {task.priority} · {task.status} ·{" "}
                              {dateOnly(task.due_at) || "No due date"}
                            </p>
                            <TaskEvidence
                              key={`${owner}:${task.id}:${reload}`}
                              token={token}
                              owner={owner}
                              task={task}
                              openThread={(id) => {
                                openThread(id);
                                setThreadPanel("workspace");
                              }}
                            />
                            {!["completed", "cancelled"].includes(
                              task.status,
                            ) && (
                              <div className="task-controls">
                                <button
                                  className="secondary-button"
                                  disabled={mutationBusy}
                                  onClick={() => {
                                    setMutationError("");
                                    setEditing(workspaceTask(task));
                                  }}
                                >
                                  Edit task
                                </button>
                                <button
                                  className="secondary-button"
                                  disabled={mutationBusy}
                                  onClick={async () => {
                                    setMutationBusy(true);
                                    setError("");
                                    try {
                                      await operate(
                                        token,
                                        owner,
                                        "task.status",
                                        {
                                          task_id: task.id,
                                          status: "completed",
                                        },
                                      );
                                      if (mounted.current)
                                        setReload((n) => n + 1);
                                    } catch {
                                      if (mounted.current)
                                        setError(
                                          "Completion was not confirmed. Refresh before retrying; check for incomplete dependencies.",
                                        );
                                    } finally {
                                      if (mounted.current)
                                        setMutationBusy(false);
                                    }
                                  }}
                                >
                                  Complete task
                                </button>
                              </div>
                            )}
                          </article>
                        ))}
                      </div>
                      {!taskRows.length && (
                        <p>
                          No {taskStatus === "all" ? "" : taskStatus} tasks
                          here. Create a task or change the filter.
                        </p>
                      )}
                    </>
                  )}
                  {view === "Calendar" && (
                    <TaskCalendar
                      key={owner}
                      owner={displayOwner(owner)}
                      tasks={tasks
                        .filter((t) => !thread || t.thread_id === thread)
                        .map(workspaceTask)}
                      events={[]}
                      onChange={() => {}}
                      onOpenTasks={() => setView("Tasks")}
                      search=""
                      readOnlyEvents
                      weekStart={global.data?.value.preferences.week_start}
                      timeFormat={global.data?.value.preferences.time_format}
                      showCompleted={
                        global.data?.value.preferences.show_completed
                      }
                    />
                  )}
                  {view === "Activity" && (
                    <section
                      className="live-activity"
                      aria-label="Live activity"
                    >
                      <h2>Activity</h2>
                      <LiveWatches
                        key={owner}
                        token={token}
                        owner={owner}
                        onTasks={() => navigate("Tasks")}
                      />
                      {contextLoading && <p role="status">Loading activity…</p>}
                      {activity.events.map((event, i) => (
                        <article key={i}>
                          <h3>{event.event}</h3>
                          <p>{event.detail}</p>
                          <small>
                            {event.created_at} · {event.request_id}
                          </small>
                        </article>
                      ))}
                      {!activity.events.length && (
                        <p>No recorded activity yet.</p>
                      )}
                      <h2>Workers</h2>
                      {activity.workers.map((worker) => (
                        <p key={worker.id}>
                          {worker.role ?? worker.id} · {worker.status} · Task{" "}
                          {worker.task_id}
                        </p>
                      ))}
                    </section>
                  )}
                </>
              )}
            </>
          )}
        </div>
        {["Chat", "Focus Threads", "Tasks", "Activity"].includes(view) && (
          <aside
            className="live-insights-panel context-panel"
            aria-label="Agent dashboard and subagents"
          >
            <div className="context-heading">
              <span>AT A GLANCE</span>
            </div>
            <ContextSummary
              tasksUnavailable={loading || !!error || taskRefreshError}
              tasksPartial={taskCursor !== null}
              key={`${owner}:${reload}`}
              token={token}
              owner={owner}
              tasks={tasks}
              onApprovals={() => navigate("Approvals")}
              onTasks={() => navigate("Tasks")}
              dailyBudget={global.data?.value.models?.agent_daily_microusd}
            />
            <div className="context-footer">
              <Icon name="shield" />
              <span>
                Separate spaces. Shared purpose.
                <small>
                  {displayOwner(owner)}’s context stays with{" "}
                  {displayOwner(owner)}.
                </small>
              </span>
            </div>
          </aside>
        )}
        <TaskEditor
          key={editing?.id ?? "new"}
          task={editing}
          threads={threads.map((t) => t.id)}
          threadChoices={threads}
          defaultThread={view === "Tasks" ? null : thread || null}
          busy={mutationBusy}
          error={mutationError}
          saveBlocked={uncertainCreate}
          live
          onSave={saveTask}
          onCancel={() => {
            if (!mutationBusy) {
              setEditing(undefined);
              setPendingCreated(null);
              setReload((n) => n + 1);
            }
          }}
        />
      </main>
    </div>
  );
}
