import React, { useCallback, useEffect, useRef, useState } from "react";
import { createRoot } from "react-dom/client";
import "./prototype.css";
import { PrototypeChat } from "./chat";
import { AgentSettings } from "./agent-settings";
import { TaskEditor, type WorkspaceTask } from "./tasks";
import {
  WorkspaceSettings,
  defaultPreferences,
  type Preferences,
} from "./settings";
import { ConnectedWorkspace } from "./connected";
import { TaskCalendar, initialCalendarEvents } from "./calendar";

import { Avatar, Icon, type Agent } from "./shared";
type IconName = Parameters<typeof Icon>[0]["name"];
type Page =
  | "Chat"
  | "Focus Threads"
  | "Tasks"
  | "Approvals"
  | "Files"
  | "Agent settings"
  | "Activity"
  | "Settings";
const sampleTasks = {
  Janet: [
    {
      title: "Prepare customer demo",
      due: "Friday, Oct 23",
      dueDate: "2026-10-23",
      steps: ["Draft agenda", "Prepare demo environment", "Review with User"],
      done: [true, false, false],
    },
    {
      title: "Finalize v0.2 release plan",
      due: "Wednesday, Oct 21",
      dueDate: "2026-10-21",
      steps: ["Gather feedback", "Review release scope", "Prepare final draft"],
      done: [true, true, false],
    },
    {
      title: "Review infrastructure costs",
      due: "Thursday, Oct 22",
      dueDate: "2026-10-22",
      steps: ["Collect usage", "Compare options"],
      done: [true, false],
    },
  ],
  Harley: [
    {
      title: "Plan a mountain day",
      due: "Saturday, Oct 24",
      dueDate: "2026-10-24",
      steps: ["Shortlist trails", "Check the forecast", "Pack the essentials"],
      done: [true, false, false],
    },
    {
      title: "Book car service",
      due: "Friday, Oct 23",
      dueDate: "2026-10-23",
      steps: ["Find a time", "Review booking"],
      done: [false, false],
    },
  ],
};
const initialTasks = Object.fromEntries(
  Object.entries(sampleTasks).map(([owner, rows]) => [
    owner,
    rows.map((task, index) => ({
      ...task,
      id: `${owner}-${index}`,
      priority: "normal",
      description: "",
      thread: (owner === "Janet"
        ? ["Customer demo", "v0.2 release", "Infrastructure review"]
        : ["Mountain weekend", "Around the house"])[index],
    })),
  ]),
) as Record<Agent, WorkspaceTask[]>;
function Prototype() {
  const [connected, setConnected] = useState(
    () => new URLSearchParams(location.search).get("mode") !== "sample",
  );
  const [preferences, setPreferences] =
    useState<Preferences>(defaultPreferences);
  const [editingTask, setEditingTask] = useState<
    WorkspaceTask | null | undefined
  >(undefined);
  const [taskFilter, setTaskFilter] = useState("all");
  const [threadTab, setThreadTab] = useState<"conversation" | "workspace">(
    "conversation",
  );
  const [threadFiles, setThreadFiles] = useState<Record<string, string[]>>({
    "Janet:v0.2 release": ["v0.2 Release Plan.md"],
    "Janet:Customer demo": ["Customer Demo Agenda.md"],
    "Harley:Mountain weekend": ["Weekend shortlist.md"],
  });
  const [decisions, setDecisions] = useState<Record<string, string[]>>({});
  const [decision, setDecision] = useState("");
  const [results, setResults] = useState<Record<string, string[]>>({});
  const [agent, setAgent] = useState<Agent>("Janet");
  const [theme, setTheme] = useState<"light" | "dark">(() =>
    new URLSearchParams(window.location.search).get("theme") === "light"
      ? "light"
      : "dark",
  );
  const [page, setPage] = useState<Page>("Chat");
  const [tasks, setTasks] = useState(initialTasks);
  const [taskView, setTaskView] = useState<"tasks" | "calendar">("tasks");
  const [calendarEntries, setCalendarEntries] = useState(initialCalendarEvents);
  const [search, setSearch] = useState("");
  const [inspector, setInspector] = useState(
    () => window.matchMedia("(min-width: 1051px)").matches,
  );
  const [mobileNav, setMobileNav] = useState(false);
  const [modal, setModal] = useState<
    "draft" | "thread" | "new-agent" | "worker" | null
  >(null);
  const [approved, setApproved] = useState<Record<Agent, boolean>>({
    Janet: false,
    Harley: false,
  });
  const [threads, setThreads] = useState<Record<Agent, string[]>>({
    Janet: ["v0.2 release", "Customer demo", "Infrastructure review"],
    Harley: ["Mountain weekend", "Around the house"],
  });
  const [threadName, setThreadName] = useState("");
  const [plannedAgents, setPlannedAgents] = useState<string[]>([]);
  const [workers, setWorkers] = useState([
    {
      id: "release-research",
      owner: "Janet" as Agent,
      title: "Release researcher",
      task: "Gathering feedback for v0.2",
      progress: 65,
      thread: "v0.2 release",
    },
    {
      id: "demo-prep",
      owner: "Janet" as Agent,
      title: "Demo preparation",
      task: "Checking the sample demo checklist",
      progress: 35,
      thread: "Customer demo",
    },
    {
      id: "trail-scout",
      owner: "Harley" as Agent,
      title: "Trail scout",
      task: "Comparing weekend trail options",
      progress: 45,
      thread: "Mountain weekend",
    },
  ]);
  const [collapsedAgents, setCollapsedAgents] = useState<Agent[]>([]);
  const [selectedWorkerId, setSelectedWorkerId] = useState<string | null>(null);
  const selectedWorker = workers.find(
    (worker) => worker.id === selectedWorkerId,
  );
  const [newAgentName, setNewAgentName] = useState("");
  const [activeThread, setActiveThread] = useState<string | null>(null);
  const [files, setFiles] = useState<Record<Agent, string[]>>({
    Janet: ["v0.2 Release Plan.md", "Customer Demo Agenda.md"],
    Harley: ["Weekend shortlist.md"],
  });
  const [notice, setNotice] = useState("");
  const [events, setEvents] = useState<Record<Agent, string[]>>({
    Janet: [
      "Release plan draft prepared",
      "Customer demo agenda completed",
      "Infrastructure review started",
    ],
    Harley: ["Trail shortlist prepared", "Household checklist updated"],
  });
  const dialog = useRef<HTMLDialogElement>(null);
  const [replyingSessions, setReplyingSessions] = useState<string[]>([]);
  const handleRunningChange = useCallback(
    (session: string, running: boolean) => {
      setReplyingSessions((old) =>
        running
          ? old.includes(session)
            ? old
            : [...old, session]
          : old.includes(session)
            ? old.filter((id) => id !== session)
            : old,
      );
    },
    [],
  );
  const isReplying = replyingSessions.some((session) =>
    session.startsWith(`${agent}:`),
  );
  const activeWorker = workers.find((worker) => worker.owner === agent);
  const nextTask = tasks[agent].find((task) => task.done.some((done) => !done));
  const currentActivity = isReplying
    ? "Replying to your message…"
    : activeWorker
      ? `${activeWorker.task} · ${activeWorker.progress}%`
      : nextTask
        ? `Ready · Next: ${nextTask.title}`
        : "Ready for your next request";
  const apiUsage =
    agent === "Janet"
      ? { input: 86400, output: 12800, cost: 1.24, budget: 5 }
      : { input: 24600, output: 4200, cost: 0.38, budget: 3 };
  const tokenCount = (value: number) =>
    new Intl.NumberFormat("en", {
      notation: "compact",
      maximumFractionDigits: 1,
    }).format(value);
  const draftTitle =
    agent === "Janet" ? "v0.2 Release Plan" : "Mountain Weekend";
  useEffect(() => {
    if (modal) dialog.current?.showModal();
    else dialog.current?.close();
  }, [modal]);
  useEffect(() => {
    if (notice) {
      const id = setTimeout(() => setNotice(""), 4000);
      return () => clearTimeout(id);
    }
  }, [notice]);
  useEffect(() => {
    const handleKey = (event: KeyboardEvent) => {
      if (event.key === "Escape" && mobileNav) {
        setMobileNav(false);
        document.querySelector<HTMLButtonElement>(".mobile-menu")?.focus();
      }
      if ((event.metaKey || event.ctrlKey) && event.key === ",") {
        event.preventDefault();
        navigate("Settings");
      }
    };
    window.addEventListener("keydown", handleKey);
    return () => window.removeEventListener("keydown", handleKey);
  }, [mobileNav]);
  function navigate(next: Page) {
    setDecision("");
    setPage(next);
    setMobileNav(false);
    setActiveThread(null);
    setSearch("");
  }
  function addEvent(text: string) {
    setEvents((old) => ({ ...old, [agent]: [text, ...old[agent]] }));
  }
  function toggleStep(taskIndex: number, stepIndex: number) {
    setTasks((old) => ({
      ...old,
      [agent]: old[agent].map((task, index) =>
        index !== taskIndex
          ? task
          : {
              ...task,
              done: task.done.map((value, step) =>
                step === stepIndex ? !value : value,
              ),
            },
      ),
    }));
  }
  function reviewDone() {
    setApproved((old) => ({ ...old, [agent]: true }));
    addEvent(`${draftTitle} approved in prototype`);
    setModal(null);
    setNotice("Sample draft approved. No external action was taken.");
  }
  function openThread(title: string) {
    setThreadTab("conversation");
    setDecision("");
    setActiveThread(title);
    setPage("Chat");
    setSearch("");
    setMobileNav(false);
  }
  const filteredTasks = tasks[agent]
    .filter(
      (task) =>
        task.title.toLowerCase().includes(search.toLowerCase()) &&
        (taskFilter === "all" ||
          (taskFilter === "completed"
            ? task.done.every(Boolean)
            : task.done.some((done) => !done))),
    )
    .sort(
      (a, b) =>
        ({ urgent: 0, high: 1, normal: 2, low: 3 })[a.priority] -
        { urgent: 0, high: 1, normal: 2, low: 3 }[b.priority],
    );
  function renderTaskCard({
    index,
    compact = false,
  }: {
    index: number;
    compact?: boolean;
  }) {
    const task = tasks[agent][index];
    if (!task) return <p>No tasks yet. Create one in Tasks.</p>;
    const count = task.done.filter(Boolean).length;
    const progress = Math.round((count / task.steps.length) * 100);
    return (
      <article
        key={task.id}
        className={`task-card ${compact ? "compact" : ""}`}
      >
        <div className="task-title">
          <span
            className="progress-ring"
            style={{
              background: `conic-gradient(var(--success) ${progress}%, var(--line) 0)`,
            }}
          >
            <span>
              <Icon name="tasks" />
            </span>
          </span>
          <div>
            <h3>{task.title}</h3>
            <p>
              {task.dueDate ? `Due ${task.dueDate}` : "No due date"} ·{" "}
              {task.priority} priority
            </p>
          </div>
        </div>
        {!compact && (
          <>
            <p>{task.description}</p>
            <div className="task-controls">
              <button
                className="secondary-button"
                onClick={() => setEditingTask(task)}
              >
                Edit task
              </button>
              <button
                className="secondary-button"
                onClick={() =>
                  setTasks((old) => ({
                    ...old,
                    [agent]: old[agent].map((item) =>
                      item.id === task.id
                        ? {
                            ...item,
                            done: item.done.map(
                              () => !item.done.every(Boolean),
                            ),
                          }
                        : item,
                    ),
                  }))
                }
              >
                {task.done.every(Boolean) ? "Reopen task" : "Complete task"}
              </button>
              {task.thread && (
                <button
                  className="secondary-button"
                  onClick={() => openThread(task.thread!)}
                >
                  {task.thread}
                </button>
              )}
            </div>
          </>
        )}
        <div className="progress-track">
          <span style={{ width: `${progress}%` }} />
        </div>
        <div className="task-progress">
          <span>
            {count} of {task.steps.length} complete
          </span>
          <span>{progress}%</span>
        </div>
        <div className="checklist">
          {task.steps.map((step, stepIndex) => (
            <label key={step}>
              <input
                type="checkbox"
                checked={task.done[stepIndex]}
                onChange={() => toggleStep(index, stepIndex)}
              />
              <span>{step}</span>
            </label>
          ))}
        </div>
      </article>
    );
  }
  function DraftCard() {
    return (
      <button className="file-card" onClick={() => setModal("draft")}>
        <span className="file-icon">
          <Icon name="file" />
        </span>
        <span>
          <strong>{draftTitle}</strong>
          <small>Draft · Updated 2 hours ago</small>
        </span>
        <span className="file-open">
          Open <Icon name="chevron" />
        </span>
      </button>
    );
  }
  if (connected)
    return (
      <ConnectedWorkspace
        theme={theme}
        onOpenSample={() => {
          const url = new URL(location.href);
          url.searchParams.set("mode", "sample");
          history.replaceState(null, "", url);
          setConnected(false);
        }}
      />
    );
  return (
    <div
      className="prototype"
      data-agent={agent.toLowerCase()}
      data-theme={theme}
    >
      <a className="skip-link" href="#workspace-content">
        Skip to content
      </a>
      <TaskEditor
        key={editingTask?.id ?? "new"}
        task={editingTask}
        defaultThread={activeThread}
        threads={threads[agent]}
        onCancel={() => setEditingTask(undefined)}
        onSave={(task) => {
          setTasks((old) => ({
            ...old,
            [agent]: editingTask
              ? old[agent].map((item) => (item.id === task.id ? task : item))
              : [...old[agent], task],
          }));
          addEvent(
            `${editingTask ? "Updated" : "Created"} task: ${task.title}`,
          );
          setEditingTask(undefined);
          setNotice("Task saved. Its deadline is reflected in Calendar.");
        }}
      />
      <div className="workspace-shell">
        {mobileNav && (
          <button
            className="nav-scrim"
            aria-label="Close navigation"
            onClick={() => setMobileNav(false)}
          />
        )}
        <aside className={`sidebar ${mobileNav ? "nav-open" : ""}`}>
          <a href="/prototype.html" className="prototype-brand">
            <img className="claw-mark" src="/brand/mark-light.svg" alt="" />
            <strong>Clawberry</strong>
            <span className="brand-tag">preview</span>
          </a>
          <section className="sidebar-agents" aria-label="Agents">
            <div className="sidebar-agents-heading">
              <h2>AGENTS</h2>
              <button
                aria-label="Add agent"
                title="Add agent"
                onClick={() => setModal("new-agent")}
              >
                <Icon name="plus" />
              </button>
            </div>
            <div className="agent-switcher">
              {(["Janet", "Harley"] as Agent[]).map((item) => {
                const running = workers.filter(
                  (worker) => worker.owner === item,
                );
                const collapsed = collapsedAgents.includes(item);
                return (
                  <div
                    className="agent-branch"
                    data-agent={item.toLowerCase()}
                    key={item}
                  >
                    <button
                      className={`agent-choice ${agent === item ? "selected" : ""}`}
                      data-agent={item.toLowerCase()}
                      aria-label={`${item} ${item === "Janet" ? "Work agent" : "Personal agent"}`}
                      aria-pressed={agent === item}
                      onClick={() => {
                        setAgent(item);
                        navigate("Chat");
                      }}
                    >
                      <Avatar agent={item} />
                      <span>
                        <strong>{item}</strong>
                        <small>
                          {item === "Janet" ? "Work agent" : "Personal agent"}
                        </small>
                      </span>
                      <span className="online-dot" />
                    </button>
                    <button
                      className="agent-settings-button"
                      aria-label={`${item} settings`}
                      title={`${item} settings`}
                      onClick={() => {
                        setAgent(item);
                        navigate("Agent settings");
                      }}
                    >
                      <Icon name="settings" />
                    </button>
                    {running.length > 0 && (
                      <>
                        <button
                          className="subagent-toggle"
                          aria-expanded={!collapsed}
                          aria-controls={`workers-${item}`}
                          onClick={() =>
                            setCollapsedAgents((old) =>
                              old.includes(item)
                                ? old.filter((owner) => owner !== item)
                                : [...old, item],
                            )
                          }
                        >
                          <span className="branch-chevron">
                            <Icon name="chevron" />
                          </span>
                          <span>
                            {running.length} running{" "}
                            {running.length === 1 ? "subagent" : "subagents"}
                          </span>
                        </button>
                        <ul
                          className="subagent-tree"
                          id={`workers-${item}`}
                          hidden={collapsed}
                          aria-label={`${item} running subagents`}
                        >
                          {running.map((worker) => (
                            <li key={worker.id}>
                              <button
                                className="subagent-row"
                                title={worker.task}
                                aria-label={`${worker.title}: ${worker.task}, ${worker.progress}% complete`}
                                onClick={() => {
                                  setSelectedWorkerId(worker.id);
                                  setModal("worker");
                                  setMobileNav(false);
                                }}
                              >
                                <span className="worker-info">
                                  <strong>{worker.title}</strong>
                                </span>
                                <span className="worker-percent">
                                  {worker.progress}%
                                </span>
                              </button>
                            </li>
                          ))}
                        </ul>
                      </>
                    )}
                  </div>
                );
              })}
            </div>
            {plannedAgents.length > 0 && (
              <div className="sidebar-planned">
                {plannedAgents.map((owner) => (
                  <div className="registry-agent planned-agent" key={owner}>
                    <span className="planned-avatar">
                      {owner.charAt(0).toUpperCase()}
                    </span>
                    <span>
                      <strong>{owner}</strong>
                      <small>Planned agent</small>
                    </span>
                    <span className="planned-tag">Planned</span>
                  </div>
                ))}
              </div>
            )}
          </section>
          <div className="nav-divider" />
          <nav aria-label="Workspace">
            {(
              [
                ["Focus Threads", "thread"],
                ["Tasks", "tasks"],
                ["Approvals", "shield"],
                ["Files", "file"],
                ["Activity", "activity"],
              ] as [Page, IconName][]
            ).map(([title, icon]) => (
              <button
                key={title}
                className={page === title ? "active" : ""}
                aria-current={page === title ? "page" : undefined}
                onClick={() => navigate(title)}
              >
                <Icon name={icon} />
                {title}
                {title === "Tasks" && (
                  <span className="nav-count">{tasks[agent].length}</span>
                )}
                {title === "Approvals" && !approved[agent] && (
                  <span className="nav-count attention">1</span>
                )}
              </button>
            ))}
          </nav>
          <div className="thread-nav">
            <div className="thread-label">
              RECENT THREADS
              <button
                aria-label="New Focus Thread"
                onClick={() => setModal("thread")}
              >
                <Icon name="plus" />
              </button>
            </div>
            {threads[agent].slice(-3).map((title) => (
              <button
                className={activeThread === title ? "current-thread" : ""}
                key={title}
                onClick={() => openThread(title)}
              >
                <span>#</span>
                {title}
              </button>
            ))}
          </div>
          <div className="sidebar-bottom">
            <div className="sandbox-note">
              <span className="sample-dot" />
              Prototype workspace<small>Sample data · Yours to explore</small>
            </div>
            <button
              className="profile"
              onClick={() => {
                setMobileNav(false);
                navigate("Settings");
              }}
            >
              <span className="user-avatar">M</span>
              <span>
                <strong>User</strong>
                <small>Personal workspace</small>
              </span>
              <Icon name="settings" />
            </button>
          </div>
        </aside>
        <main className="main-workspace" id="workspace-content" tabIndex={-1}>
          <header className="workspace-header">
            <button
              className="icon-button mobile-menu"
              aria-label="Open navigation"
              onClick={() => setMobileNav(true)}
            >
              <Icon name="panel" />
            </button>
            <Avatar agent={agent} />
            <div className="agent-heading">
              <div>
                <h1>
                  {page === "Settings"
                    ? "Settings"
                    : page === "Agent settings"
                      ? `${agent} settings`
                      : agent}
                </h1>
                <span
                  className="online-label"
                  hidden={page === "Settings" || page === "Agent settings"}
                >
                  <span className="online-dot" />
                  {isReplying ? "Replying" : activeWorker ? "Working" : "Ready"}
                </span>
              </div>
              <p
                className="agent-current-activity"
                aria-live="polite"
                aria-atomic="true"
                aria-label="Current agent activity"
                title={`${page === "Settings" ? "Workspace preferences and model routing" : currentActivity} (prototype activity)`}
              >
                {page === "Settings"
                  ? "Workspace preferences and model routing"
                  : page === "Agent settings"
                    ? `${agent}’s identity, skills, credentials, and boundaries`
                    : currentActivity}
              </p>
            </div>
            <div className="header-actions">
              <label className="workspace-search" hidden={page === "Settings"}>
                <Icon name="search" />
                <input
                  aria-label="Search current view"
                  placeholder="Search this workspace…"
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                />
                <kbd>⌕</kbd>
              </label>
              <button
                className="icon-button theme-toggle"
                aria-label={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}
                title={`Switch to ${theme === "dark" ? "light" : "dark"} theme`}
                onClick={() => {
                  const next = theme === "dark" ? "light" : "dark";
                  setTheme(next);
                  const url = new URL(window.location.href);
                  url.searchParams.set("theme", next);
                  window.history.replaceState(null, "", url);
                }}
              >
                <Icon name={theme === "dark" ? "sun" : "moon"} />
              </button>
              <button
                className={`icon-button ${inspector ? "is-active" : ""}`}
                aria-label="Toggle context panel"
                disabled={
                  page === "Settings" ||
                  page === "Agent settings" ||
                  (page === "Tasks" && taskView === "calendar")
                }
                aria-expanded={inspector}
                onClick={() => setInspector(!inspector)}
              >
                <Icon name="panel" />
              </button>
            </div>
          </header>
          <div
            className={`workspace-body ${inspector && page !== "Settings" && page !== "Agent settings" && !(page === "Tasks" && taskView === "calendar") ? "show-context" : ""}`}
          >
            <div className="main-column">
              <div
                className="collection-view settings-page"
                hidden={page !== "Settings"}
                role="region"
                aria-label="Settings"
              >
                <WorkspaceSettings
                  preferences={preferences}
                  onChange={setPreferences}
                  theme={theme}
                  onTheme={setTheme}
                  onConnect={() => setConnected(true)}
                />
              </div>
              {page === "Chat" && activeThread && (
                <div className="thread-workspace-bar">
                  <strong>{activeThread}</strong>
                  <div className="task-controls">
                    <button
                      className="secondary-button"
                      aria-pressed={threadTab === "conversation"}
                      onClick={() => setThreadTab("conversation")}
                    >
                      Conversation
                    </button>
                    <button
                      className="secondary-button"
                      aria-pressed={threadTab === "workspace"}
                      onClick={() => setThreadTab("workspace")}
                    >
                      Thread workspace
                    </button>
                  </div>
                </div>
              )}
              {page === "Chat" && activeThread && threadTab === "workspace" && (
                <section
                  className="collection-view thread-workspace"
                  aria-label="Thread workspace"
                >
                  <h2>Tasks</h2>
                  <button
                    className="primary-button"
                    onClick={() => setEditingTask(null)}
                  >
                    New task
                  </button>
                  <div className="task-grid">
                    {tasks[agent]
                      .filter((task) => task.thread === activeThread)
                      .map((task) =>
                        renderTaskCard({ index: tasks[agent].indexOf(task) }),
                      )}
                  </div>
                  {!tasks[agent].some(
                    (task) => task.thread === activeThread,
                  ) && (
                    <p>
                      No tasks linked yet. Choose this thread when creating or
                      editing a task.
                    </p>
                  )}
                  <h2>Files</h2>
                  <p>Local filenames only. No file contents are uploaded.</p>
                  <ul>
                    {(threadFiles[`${agent}:${activeThread}`] ?? []).map(
                      (file, i) => (
                        <li key={`${file}-${i}`}>{file}</li>
                      ),
                    )}
                  </ul>
                  {!threadFiles[`${agent}:${activeThread}`]?.length && (
                    <p>
                      Attach a file in this thread’s conversation to list it
                      here.
                    </p>
                  )}
                  <h2>Decisions</h2>
                  <ul>
                    {(decisions[`${agent}:${activeThread}`] ?? []).map(
                      (text, i) => (
                        <li key={i}>{text} · Recorded by User</li>
                      ),
                    )}
                  </ul>
                  <form
                    onSubmit={(event) => {
                      event.preventDefault();
                      if (!decision.trim()) return;
                      setDecisions((old) => ({
                        ...old,
                        [`${agent}:${activeThread}`]: [
                          ...(old[`${agent}:${activeThread}`] ?? []),
                          decision.trim(),
                        ],
                      }));
                      setDecision("");
                    }}
                  >
                    <label className="field-label" htmlFor="thread-decision">
                      Record a decision
                    </label>
                    <input
                      id="thread-decision"
                      className="thread-name-input"
                      value={decision}
                      maxLength={2000}
                      onChange={(event) => setDecision(event.target.value)}
                      required
                    />
                    <button className="secondary-button">Add decision</button>
                  </form>
                  <h2>Subagent results</h2>
                  {(results[`${agent}:${activeThread}`] ?? []).map(
                    (result, i) => (
                      <p key={i}>{result}</p>
                    ),
                  )}
                  {workers
                    .filter(
                      (worker) =>
                        worker.owner === agent &&
                        worker.thread === activeThread,
                    )
                    .map((worker) => (
                      <button
                        key={worker.id}
                        className="secondary-button"
                        onClick={() => {
                          setSelectedWorkerId(worker.id);
                          setModal("worker");
                        }}
                      >
                        {worker.title} · Running {worker.progress}%
                      </button>
                    ))}
                  {!results[`${agent}:${activeThread}`]?.length && (
                    <p>
                      Completed sample runs will appear here with their handoff.
                    </p>
                  )}
                </section>
              )}
              <div
                className="chat-sessions"
                hidden={
                  page !== "Chat" ||
                  (!!activeThread && threadTab !== "conversation")
                }
              >
                {(["Janet", "Harley"] as Agent[]).flatMap((owner) =>
                  [null, ...threads[owner]].map((thread) => (
                    <div
                      className="chat-session"
                      key={`${owner}:${thread ?? "main"}`}
                      hidden={agent !== owner || activeThread !== thread}
                    >
                      <PrototypeChat
                        agent={owner}
                        thread={thread}
                        search={agent === owner ? search : ""}
                        onRunningChange={handleRunningChange}
                        onReview={() => setModal("draft")}
                        onAttach={(names) => {
                          if (thread)
                            setThreadFiles((old) => ({
                              ...old,
                              [`${owner}:${thread}`]: [
                                ...(old[`${owner}:${thread}`] ?? []),
                                ...names,
                              ],
                            }));
                          setFiles((old) => ({
                            ...old,
                            [owner]: [...old[owner], ...names],
                          }));
                          setNotice(
                            `${names.length} sample attachment(s) added to Files. Nothing was uploaded.`,
                          );
                        }}
                      />
                    </div>
                  )),
                )}
              </div>
              <div
                className="collection-view settings-page agent-settings-page"
                hidden={page !== "Agent settings"}
                role="region"
                aria-label={`${agent} settings`}
              >
                {(["Janet", "Harley"] as Agent[]).map((owner) => (
                  <div key={owner} hidden={owner !== agent}>
                    <AgentSettings
                      owner={owner}
                      search={owner === agent ? search : ""}
                      onChat={() => {
                        setAgent(owner);
                        navigate("Chat");
                      }}
                    />
                  </div>
                ))}
              </div>
              {page !== "Chat" &&
                page !== "Agent settings" &&
                page !== "Settings" && (
                  <div
                    className={`collection-view ${page === "Tasks" ? "tasks-collection" : ""}`}
                    role="region"
                    aria-label={page}
                  >
                    {page === "Focus Threads" && (
                      <div className="collection-actions">
                        <button
                          className="primary-button"
                          onClick={() => setModal("thread")}
                        >
                          <Icon name="plus" />
                          New thread
                        </button>
                      </div>
                    )}
                    {page === "Tasks" && (
                      <>
                        <div className="tasks-view-toolbar">
                          <button
                            className="primary-button"
                            onClick={() => setEditingTask(null)}
                          >
                            <Icon name="plus" />
                            New task
                          </button>
                          <label>
                            Show tasks{" "}
                            <select
                              aria-label="Task status filter"
                              value={taskFilter}
                              onChange={(event) =>
                                setTaskFilter(event.target.value)
                              }
                            >
                              <option value="all">All tasks</option>
                              <option value="active">Active</option>
                              <option value="completed">Completed</option>
                            </select>
                          </label>
                          <div
                            className="tasks-view-switch"
                            role="group"
                            aria-label="Task view"
                          >
                            <button
                              className={
                                taskView === "tasks" ? "selected-view" : ""
                              }
                              aria-pressed={taskView === "tasks"}
                              onClick={() => setTaskView("tasks")}
                            >
                              <Icon name="tasks" />
                              Tasks
                            </button>
                            <button
                              className={
                                taskView === "calendar" ? "selected-view" : ""
                              }
                              aria-pressed={taskView === "calendar"}
                              onClick={() => setTaskView("calendar")}
                            >
                              <Icon name="calendar" />
                              Calendar
                            </button>
                          </div>
                        </div>
                        <div
                          className="task-grid"
                          hidden={taskView !== "tasks"}
                        >
                          {filteredTasks.map((task) =>
                            renderTaskCard({
                              index: tasks[agent].indexOf(task),
                            }),
                          )}
                          {!filteredTasks.length && <p>No matching tasks.</p>}
                        </div>
                        {(["Janet", "Harley"] as Agent[]).map((owner) => (
                          <div
                            key={owner}
                            hidden={taskView !== "calendar" || owner !== agent}
                          >
                            <TaskCalendar
                              owner={owner}
                              tasks={tasks[owner]}
                              weekStart={preferences.weekStart}
                              showCompleted={preferences.showCompleted}
                              timeFormat={preferences.timeFormat}
                              events={calendarEntries[owner]}
                              onChange={(events) =>
                                setCalendarEntries((old) => ({
                                  ...old,
                                  [owner]: events,
                                }))
                              }
                              onOpenTasks={() => setTaskView("tasks")}
                              search={owner === agent ? search : ""}
                            />
                          </div>
                        ))}
                      </>
                    )}
                    {page === "Focus Threads" &&
                      !threads[agent].some((title) =>
                        title.toLowerCase().includes(search.toLowerCase()),
                      ) && (
                        <p>
                          No matching Focus Threads. Clear the search or create
                          a thread.
                        </p>
                      )}
                    {page === "Focus Threads" && (
                      <div className="thread-grid">
                        {threads[agent]
                          .filter((title) =>
                            title.toLowerCase().includes(search.toLowerCase()),
                          )
                          .map((title, i) => (
                            <button
                              className="thread-card"
                              key={title}
                              onClick={() => openThread(title)}
                            >
                              <span className="thread-card-icon">
                                <Icon name="thread" />
                              </span>
                              <span className="thread-card-status">
                                Focus thread
                              </span>
                              <h3>{title}</h3>
                              <p>
                                {i === 0
                                  ? "A clear plan and a few next steps."
                                  : "Ideas, decisions, and everything in between."}
                              </p>
                              <footer>
                                Open conversation
                                <Icon name="chevron" />
                              </footer>
                            </button>
                          ))}
                      </div>
                    )}
                    {page === "Approvals" && (
                      <div className="approval-large">
                        <span className="eyebrow">
                          {approved[agent]
                            ? "REVIEWED"
                            : "AWAITING YOUR REVIEW"}
                        </span>
                        <h3>{draftTitle}</h3>
                        <p>
                          {approved[agent]
                            ? "You approved this sample draft. No external action was taken."
                            : "The draft is ready. Take a look and decide what happens next."}
                        </p>
                        <DraftCard />
                        <button
                          className="primary-button"
                          onClick={() => setModal("draft")}
                        >
                          {approved[agent] ? "View draft" : "Review draft"}
                          <Icon name="chevron" />
                        </button>
                      </div>
                    )}
                    {page === "Files" && (
                      <>
                        <div className="file-list">
                          {files[agent]
                            .filter((file) =>
                              file.toLowerCase().includes(search.toLowerCase()),
                            )
                            .map((file, i) => (
                              <button
                                key={`${file}-${i}`}
                                onClick={() => {
                                  if (i === 0) setModal("draft");
                                  else
                                    setNotice(
                                      `${file} is a sample file. No file contents are stored.`,
                                    );
                                }}
                              >
                                <span className="file-icon">
                                  <Icon name="file" />
                                </span>
                                <span>
                                  <strong>{file}</strong>
                                  <small>
                                    {i < 2
                                      ? "Sample document"
                                      : "Local filename only"}{" "}
                                    · {agent}’s workspace
                                  </small>
                                </span>
                                <Icon name="chevron" />
                              </button>
                            ))}
                        </div>
                      </>
                    )}
                    {page === "Activity" &&
                      !events[agent].some((event) =>
                        event.toLowerCase().includes(search.toLowerCase()),
                      ) && <p>No matching activity. Try another search.</p>}
                    {page === "Activity" && (
                      <div className="activity-list">
                        {events[agent]
                          .filter((event) =>
                            event.toLowerCase().includes(search.toLowerCase()),
                          )
                          .map((event, i) => (
                            <article key={`${event}-${i}`}>
                              <span className="activity-check">
                                <Icon name="check" />
                              </span>
                              <div>
                                <h3>{event}</h3>
                                <p>
                                  {agent} ·{" "}
                                  {i === 0
                                    ? "Just now"
                                    : `${i * 24} minutes ago`}{" "}
                                  · Sample activity
                                </p>
                              </div>
                            </article>
                          ))}
                      </div>
                    )}
                  </div>
                )}
            </div>
            <aside className="context-panel">
              <div className="context-heading">
                <span>AT A GLANCE</span>
                <button
                  className="icon-button"
                  aria-label="Close context panel"
                  onClick={() => setInspector(false)}
                >
                  <Icon name="close" />
                </button>
              </div>
              <section className="context-section approval-section">
                <div className="section-heading">
                  <h2>
                    <span className="panel-icon approval-icon">
                      <Icon name="shield" />
                    </span>
                    {approved[agent] ? "Draft reviewed" : "Pending approval"}
                    <span className="panel-count">
                      {approved[agent] ? 0 : 1}
                    </span>
                  </h2>
                  <button onClick={() => navigate("Approvals")}>
                    View all
                  </button>
                </div>
                <div className="approval-card">
                  <div className="approval-document">
                    <span className="file-icon">
                      <Icon name="file" />
                    </span>
                    <div>
                      <h3>{draftTitle}</h3>
                      <p>
                        {approved[agent]
                          ? "Approved in this prototype"
                          : "Draft ready for your review"}
                      </p>
                      <small>Today, 9:10 AM · Sample</small>
                    </div>
                  </div>
                  <div className="approval-actions">
                    <button
                      className="approve-button"
                      onClick={() => setModal("draft")}
                    >
                      <Icon name="check" />
                      {approved[agent] ? "View draft" : "Review draft"}
                    </button>
                    <button
                      className="secondary-button"
                      onClick={() => {
                        addEvent(`Changes requested on ${draftTitle}`);
                        setNotice("Changes requested on the sample draft.");
                      }}
                    >
                      Request changes
                    </button>
                  </div>
                </div>
              </section>
              <section className="context-section task-section">
                <div className="section-heading">
                  <h2>
                    <span className="panel-icon task-icon">
                      <Icon name="tasks" />
                    </span>
                    Active task
                  </h2>
                  <button onClick={() => navigate("Tasks")}>View all</button>
                </div>
                {renderTaskCard({
                  index: nextTask ? tasks[agent].indexOf(nextTask) : 0,
                  compact: true,
                })}
              </section>
              <section
                className="api-usage-card"
                aria-label={`${agent} Claude API usage`}
              >
                <div className="api-usage-heading">
                  <span className="claude-monogram" aria-hidden="true">
                    C
                  </span>
                  <h2>Claude API</h2>
                  <span>Today · Sample</span>
                </div>
                <div className="api-usage-total">
                  <div>
                    <span>Estimated cost</span>
                    <strong>
                      <small>US$</small>
                      {apiUsage.cost.toFixed(2)}
                    </strong>
                  </div>
                  <div>
                    <span>Total tokens</span>
                    <strong>
                      {tokenCount(apiUsage.input + apiUsage.output)}
                    </strong>
                  </div>
                </div>
                <dl className="api-token-breakdown">
                  <div>
                    <dt>
                      <span className="input-token-dot" />
                      Input tokens
                    </dt>
                    <dd>{tokenCount(apiUsage.input)}</dd>
                  </div>
                  <div>
                    <dt>
                      <span className="output-token-dot" />
                      Output tokens
                    </dt>
                    <dd>{tokenCount(apiUsage.output)}</dd>
                  </div>
                </dl>
                <div className="api-budget-heading">
                  <span>Daily budget</span>
                  <span>
                    US${apiUsage.cost.toFixed(2)} / $
                    {apiUsage.budget.toFixed(2)}
                  </span>
                </div>
                <div
                  className="api-budget-track"
                  role="progressbar"
                  aria-label="Sample daily API budget used"
                  aria-valuenow={apiUsage.cost}
                  aria-valuemin={0}
                  aria-valuemax={apiUsage.budget}
                >
                  <span
                    style={{
                      width: `${(apiUsage.cost / apiUsage.budget) * 100}%`,
                    }}
                  />
                </div>
                <p>
                  {agent} + subagents · Illustrative usage, not live billing
                </p>
              </section>
              <div className="context-footer">
                <Icon name="shield" />
                <span>
                  Separate spaces. Shared purpose.
                  <small>
                    {agent}’s context stays with {agent}.
                  </small>
                </span>
              </div>
            </aside>
          </div>
        </main>
      </div>
      {notice &&
        (preferences.notifications ||
          /already|failed|error|external action/i.test(notice)) && (
          <div className="toast" role="status">
            <Icon name="check" />
            {notice}
            <button
              aria-label="Dismiss notification"
              onClick={() => setNotice("")}
            >
              <Icon name="close" />
            </button>
          </div>
        )}
      <dialog
        ref={dialog}
        className="prototype-dialog"
        onCancel={() => setModal(null)}
        onClick={(event) => {
          if (event.target === event.currentTarget) setModal(null);
        }}
        aria-labelledby="dialog-title"
      >
        <div className="dialog-top">
          <span className="eyebrow">
            {modal === "draft" ? "SAMPLE DOCUMENT" : "PROTOTYPE WORKSPACE"}
          </span>
          <button
            className="icon-button"
            aria-label="Close dialog"
            onClick={() => setModal(null)}
          >
            <Icon name="close" />
          </button>
        </div>
        <h2 id="dialog-title">
          {modal === "worker"
            ? (selectedWorker?.title ?? "Subagent complete")
            : modal === "new-agent"
              ? "Plan a new agent"
              : modal === "draft"
                ? draftTitle
                : modal === "thread"
                  ? "A space to focus"
                  : "Settings"}
        </h2>
        {modal === "worker" ? (
          selectedWorker ? (
            <>
              <p className="dialog-description">
                {selectedWorker.owner}’s temporary subagent · Sample run
              </p>
              <div className="worker-detail">
                <span className="worker-running-label">
                  <span className="worker-live-dot" />
                  Running
                </span>
                <h3>{selectedWorker.task}</h3>
                <div
                  className="progress-track"
                  role="progressbar"
                  aria-label="Sample subagent progress"
                  aria-valuemin={0}
                  aria-valuemax={100}
                  aria-valuenow={selectedWorker.progress}
                >
                  <span style={{ width: `${selectedWorker.progress}%` }} />
                </div>
                <p>
                  {selectedWorker.progress}% complete · Selected context only
                </p>
                <div className="worker-log">
                  <span>Task received from {selectedWorker.owner}</span>
                  <span>Reviewing sample evidence</span>
                  <span>Preparing a structured handoff</span>
                </div>
              </div>
              <p className="sample-disclaimer">
                This is a fictional run for exploring the UI. Completing it
                removes the worker from the active tree and records sample
                activity.
              </p>
              <div className="dialog-actions">
                <button
                  className="secondary-button"
                  onClick={() => setModal(null)}
                >
                  Close
                </button>
                <button
                  className="primary-button"
                  onClick={() => {
                    const worker = selectedWorker;
                    setResults((old) => ({
                      ...old,
                      [`${worker.owner}:${worker.thread}`]: [
                        ...(old[`${worker.owner}:${worker.thread}`] ?? []),
                        `${worker.title}: ${worker.task}. Sample evidence reviewed; structured handoff prepared. Fictional result, no external sources inspected.`,
                      ],
                    }));
                    setWorkers((old) =>
                      old.filter((item) => item.id !== worker.id),
                    );
                    setEvents((old) => ({
                      ...old,
                      [worker.owner]: [
                        `${worker.title} completed — sample result archived`,
                        ...old[worker.owner],
                      ],
                    }));
                    setModal(null);
                    setNotice(
                      `${worker.title} completed. Sample result added to ${worker.owner}’s Activity.`,
                    );
                  }}
                >
                  Complete sample run
                  <Icon name="check" />
                </button>
              </div>
            </>
          ) : (
            <p>This sample subagent has finished.</p>
          )
        ) : modal === "new-agent" ? (
          <form
            onSubmit={(event) => {
              event.preventDefault();
              const value = newAgentName.trim();
              if (!value) return;
              if (
                [
                  "janet",
                  "harley",
                  ...plannedAgents.map((name) => name.toLowerCase()),
                ].includes(value.toLowerCase())
              ) {
                setNotice("That agent is already in the registry.");
                return;
              }
              setPlannedAgents((old) => [...old, value]);
              setNewAgentName("");
              setModal(null);
              setNotice(
                `${value} added as a planned agent. No live agent was created.`,
              );
            }}
          >
            <p className="dialog-description">
              Add a future assistant to your sample registry. Janet and Harley
              remain your two active agents.
            </p>
            <label className="field-label" htmlFor="agent-name">
              Agent name
            </label>
            <input
              id="agent-name"
              className="thread-name-input"
              placeholder="For example, Research"
              value={newAgentName}
              onChange={(event) => setNewAgentName(event.target.value)}
              maxLength={40}
              required
            />
            <div className="dialog-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setModal(null)}
              >
                Cancel
              </button>
              <button
                className="primary-button"
                disabled={!newAgentName.trim()}
              >
                Add planned agent
                <Icon name="plus" />
              </button>
            </div>
          </form>
        ) : modal === "draft" ? (
          <>
            <p className="dialog-description">
              Prepared by {agent} · Draft for your review
            </p>
            <div className="document-preview">
              <h3>
                {agent === "Janet"
                  ? "A smaller, stronger release."
                  : "A little time outside."}
              </h3>
              <p>
                {agent === "Janet"
                  ? "Keep v0.2 focused on reliable conversations, clear approval flows, and a workspace that feels calm and useful."
                  : "An easy day in the mountains, with enough flexibility to follow the weather and take the scenic route home."}
              </p>
              <h4>Proposed plan</h4>
              <ul>
                <li>
                  {agent === "Janet"
                    ? "Finish chat and Focus Thread polish."
                    : "Leave after breakfast on Saturday."}
                </li>
                <li>
                  {agent === "Janet"
                    ? "Walk through the customer demo together."
                    : "Choose an easy trail and bring lunch."}
                </li>
                <li>
                  {agent === "Janet"
                    ? "Review the remaining decisions before release."
                    : "Keep Sunday free for a slow morning."}
                </li>
              </ul>
              <h4>Your decision</h4>
              <p>
                {agent === "Janet"
                  ? "Keep the release scope focused, and move the extra reporting views to the next iteration?"
                  : "Go for a day trip, or leave room for an overnight stay?"}
              </p>
            </div>
            <p className="sample-disclaimer">
              Prototype only. Approving changes sample state in this tab.
            </p>
            <div className="dialog-actions">
              <button
                className="secondary-button"
                onClick={() => {
                  setModal(null);
                  setNotice("Changes requested on the sample draft.");
                  addEvent(`Changes requested on ${draftTitle}`);
                }}
              >
                Request changes
              </button>
              <button
                className="primary-button"
                disabled={approved[agent]}
                onClick={reviewDone}
              >
                <Icon name="check" />
                {approved[agent] ? "Approved" : "Approve draft"}
              </button>
            </div>
          </>
        ) : modal === "thread" ? (
          <form
            onSubmit={(event) => {
              event.preventDefault();
              if (!threadName.trim()) return;
              const title = threadName.trim();
              if (threads[agent].includes(title)) {
                openThread(title);
              } else {
                setThreads((old) => ({
                  ...old,
                  [agent]: [...old[agent], title],
                }));
                openThread(title);
                addEvent(`Focus Thread created: ${title}`);
              }
              setThreadName("");
              setModal(null);
            }}
          >
            <p className="dialog-description">
              Give a project, idea, or investigation its own conversation with{" "}
              {agent}.
            </p>
            <label className="field-label" htmlFor="thread-name">
              Thread name
            </label>
            <input
              autoFocus
              id="thread-name"
              className="thread-name-input"
              placeholder="What are we working on?"
              value={threadName}
              onChange={(event) => setThreadName(event.target.value)}
              maxLength={80}
              required
            />
            <div className="dialog-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => setModal(null)}
              >
                Cancel
              </button>
              <button className="primary-button" disabled={!threadName.trim()}>
                Create thread
                <Icon name="arrow" />
              </button>
            </div>
          </form>
        ) : (
          <>
            <p className="dialog-description">
              This standalone prototype uses sample data. Conversations and
              edits last until you reload, and nothing is sent to a server.
            </p>
            <div className="document-preview">
              <h3>Try a few things</h3>
              <p>
                Switch between Janet and Harley, check off a task, review a
                draft, or create a Focus Thread. Use the panel button to bring
                your next steps alongside the conversation.
              </p>
            </div>
            <button className="primary-button" onClick={() => setModal(null)}>
              Back to the workspace
            </button>
          </>
        )}
      </dialog>
    </div>
  );
}
createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <Prototype />
  </React.StrictMode>,
);
