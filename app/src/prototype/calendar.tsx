import { useRef, useState } from "react";
import { Icon, type Agent } from "./shared";

export type CalendarEvent = {
  id: string;
  title: string;
  date: string;
  time: string;
  duration: number;
  creator: "User" | Agent;
  notes: string;
};
export const initialCalendarEvents: Record<Agent, CalendarEvent[]> = {
  Janet: [
    {
      id: "work-checkin",
      title: "Weekly check-in",
      date: "2026-10-16",
      time: "09:30",
      duration: 30,
      creator: "User",
      notes: "Review priorities and decide what needs attention next.",
    },
    {
      id: "release-review",
      title: "Release scope review",
      date: "2026-10-20",
      time: "10:00",
      duration: 45,
      creator: "Janet",
      notes:
        "Sample event prepared by Janet. Review the remaining release decisions together.",
    },
    {
      id: "demo-rehearsal",
      title: "Demo rehearsal",
      date: "2026-10-22",
      time: "14:00",
      duration: 45,
      creator: "User",
      notes: "Walk through the customer demo with Janet.",
    },
  ],
  Harley: [
    {
      id: "weekend-plan",
      title: "Plan the weekend",
      date: "2026-10-16",
      time: "17:30",
      duration: 30,
      creator: "User",
      notes:
        "Pick a route, check the forecast, and leave room for a backup plan.",
    },
    {
      id: "packing",
      title: "Adventure prep",
      date: "2026-10-23",
      time: "18:00",
      duration: 30,
      creator: "Harley",
      notes:
        "Sample reminder: water, layers, offline map, and the all-important snacks.",
    },
  ],
};
const TODAY = "2026-10-16";
const dateKey = (date: Date) => date.toISOString().slice(0, 10);
const parseDate = (date: string) => new Date(`${date}T12:00:00Z`);
function formatDate(date: string, options: Intl.DateTimeFormatOptions) {
  return new Intl.DateTimeFormat("en-CA", {
    ...options,
    timeZone: "UTC",
  }).format(parseDate(date));
}
function timeLabel(time: string) {
  const [hour, minute] = time.split(":").map(Number);
  return `${hour % 12 || 12}:${String(minute).padStart(2, "0")} ${hour >= 12 ? "PM" : "AM"}`;
}
type CalendarTask = { title: string; dueDate: string; done: boolean[] };
type Item = {
  id: string;
  title: string;
  date: string;
  time: string;
  creator: "User" | Agent;
  task?: CalendarTask;
  event?: CalendarEvent;
};
export function TaskCalendar({
  owner,
  tasks,
  events,
  onChange,
  onOpenTasks,
  search,
  readOnlyEvents = false,
  weekStart = "sunday",
  showCompleted = true,
  timeFormat = "12",
}: {
  owner: Agent;
  tasks: CalendarTask[];
  events: CalendarEvent[];
  onChange: (events: CalendarEvent[]) => void;
  onOpenTasks: () => void;
  search: string;
  readOnlyEvents?: boolean;
  weekStart?: "sunday" | "monday";
  showCompleted?: boolean;
  timeFormat?: "12" | "24";
}) {
  const today = readOnlyEvents
    ? new Intl.DateTimeFormat("en-CA", {
        timeZone: "America/Edmonton",
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
      }).format(new Date())
    : TODAY;
  const [month, setMonth] = useState(`${today.slice(0, 7)}-01`);
  const [selectedDate, setSelectedDate] = useState(today);
  const [filter, setFilter] = useState("all");
  const [editingId, setEditingId] = useState<string | null>(null);
  const [taskDetail, setTaskDetail] = useState<CalendarTask | null>(null);
  const [title, setTitle] = useState("");
  const [date, setDate] = useState(today);
  const [time, setTime] = useState("09:00");
  const [duration, setDuration] = useState(30);
  const [notes, setNotes] = useState("");
  const [notice, setNotice] = useState("");
  const dialog = useRef<HTMLDialogElement>(null);
  const all: Item[] = [
    ...tasks
      .filter(
        (task) => task.dueDate && (showCompleted || !task.done.every(Boolean)),
      )
      .map((task, index) => ({
        id: `task-${index}`,
        title: task.title,
        date: task.dueDate,
        time: "",
        creator: owner,
        task,
      })),
    ...events.map((event) => ({ ...event, event })),
  ];
  const items = all.filter(
    (item) =>
      (filter === "all" || item.creator === filter) &&
      item.title.toLowerCase().includes(search.toLowerCase()),
  );
  const current = parseDate(month);
  const start = new Date(
    Date.UTC(
      current.getUTCFullYear(),
      current.getUTCMonth(),
      1 - ((current.getUTCDay() + (weekStart === "monday" ? 6 : 0)) % 7),
      12,
    ),
  );
  const daysInMonth = new Date(
    Date.UTC(current.getUTCFullYear(), current.getUTCMonth() + 1, 0),
  ).getUTCDate();
  const cellCount =
    Math.ceil(
      (((current.getUTCDay() + (weekStart === "monday" ? 6 : 0)) % 7) +
        daysInMonth) /
        7,
    ) * 7;
  const days = Array.from({ length: cellCount }, (_, index) =>
    dateKey(new Date(start.getTime() + index * 86400000)),
  );
  const agenda = items
    .filter((item) => item.date === selectedDate)
    .sort((a, b) => a.time.localeCompare(b.time));
  function changeMonth(offset: number) {
    const next = new Date(
      Date.UTC(current.getUTCFullYear(), current.getUTCMonth() + offset, 1, 12),
    );
    setMonth(dateKey(next));
    setSelectedDate(dateKey(next));
  }
  function openEvent(event?: CalendarEvent) {
    setEditingId(event?.id ?? null);
    setTaskDetail(null);
    setTitle(event?.title ?? "");
    setDate(event?.date ?? selectedDate);
    setTime(event?.time ?? "09:00");
    setDuration(event?.duration ?? 30);
    setNotes(event?.notes ?? "");
    dialog.current?.showModal();
  }
  function openItem(item: Item) {
    if (item.task) {
      setTaskDetail(item.task);
      dialog.current?.showModal();
    } else openEvent(item.event);
  }
  return (
    <div className="task-calendar">
      <div className="calendar-toolbar">
        <h3>{formatDate(month, { month: "long", year: "numeric" })}</h3>
        <div className="calendar-month-controls">
          <button
            className="secondary-button"
            onClick={() => {
              setMonth(`${today.slice(0, 7)}-01`);
              setSelectedDate(today);
            }}
          >
            Today
          </button>
          <button
            className="icon-button"
            aria-label="Previous month"
            onClick={() => changeMonth(-1)}
          >
            <span className="previous-month">
              <Icon name="chevron" />
            </span>
          </button>
          <button
            className="icon-button"
            aria-label="Next month"
            onClick={() => changeMonth(1)}
          >
            <Icon name="chevron" />
          </button>
        </div>
        <label className="calendar-filter">
          <span>Show</span>
          <select
            aria-label="Calendar creator filter"
            value={filter}
            onChange={(event) => setFilter(event.target.value)}
          >
            <option value="all">Everyone</option>
            <option value="User">Only me</option>
            <option value={owner}>Only {owner}</option>
          </select>
        </label>
        <button
          hidden={readOnlyEvents}
          className="primary-button calendar-new-event"
          onClick={() => openEvent()}
        >
          <Icon name="plus" />
          New event
        </button>
      </div>
      <div className="calendar-legend">
        <span>
          <i className="calendar-me-dot" />
          Your events
        </span>
        <span>
          <i className="calendar-agent-dot" />
          {owner}’s events
        </span>
        <span>
          <Icon name="tasks" />
          Task deadlines
        </span>
      </div>
      <div className="calendar-and-agenda">
        <section
          className="calendar-month"
          aria-label={`${formatDate(month, { month: "long", year: "numeric" })} shared calendar`}
        >
          <div className="calendar-weekdays">
            {(weekStart === "monday"
              ? ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
              : ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]
            ).map((day) => (
              <span key={day}>{day}</span>
            ))}
          </div>
          <div className="calendar-days">
            {days.map((day) => {
              const dayItems = items.filter((item) => item.date === day);
              const outside = day.slice(0, 7) !== month.slice(0, 7);
              return (
                <button
                  key={day}
                  className={`calendar-day ${outside ? "outside-month" : ""} ${day === selectedDate ? "selected-day" : ""} ${day === today ? "calendar-today" : ""}`}
                  aria-label={`${formatDate(day, { weekday: "long", month: "long", day: "numeric", year: "numeric" })}, ${dayItems.length} items`}
                  aria-pressed={day === selectedDate}
                  onClick={() => {
                    setSelectedDate(day);
                    if (outside) setMonth(`${day.slice(0, 7)}-01`);
                  }}
                >
                  <span className="calendar-day-number">
                    {Number(day.slice(-2))}
                  </span>
                  <div className="calendar-day-items">
                    {dayItems.slice(0, 2).map((item) => (
                      <span
                        key={item.id}
                        className={`calendar-chip ${item.task ? "task-chip" : item.creator === "User" ? "my-event-chip" : "agent-event-chip"}`}
                      >
                        <Icon name={item.task ? "tasks" : "calendar"} />
                        <span>
                          {item.time ? `${item.time} ` : ""}
                          {item.title}
                        </span>
                      </span>
                    ))}
                    {dayItems.length > 2 && (
                      <small>+{dayItems.length - 2} more</small>
                    )}
                  </div>
                  <span className="calendar-compact-count" aria-hidden="true">
                    {dayItems.length > 0
                      ? `${dayItems.length} ${dayItems.length === 1 ? "item" : "items"}`
                      : ""}
                  </span>
                </button>
              );
            })}
          </div>
        </section>
        <aside className="calendar-agenda" aria-label="Selected day agenda">
          <div className="agenda-heading">
            <span className="eyebrow">
              {formatDate(selectedDate, { weekday: "long" }).toUpperCase()}
            </span>
            <h3>
              {formatDate(selectedDate, { month: "long", day: "numeric" })}
            </h3>
            <small>
              {agenda.length} {agenda.length === 1 ? "item" : "items"} ·
              America/Edmonton
            </small>
          </div>
          {agenda.length ? (
            agenda.map((item) => (
              <button
                key={item.id}
                className="agenda-item"
                onClick={() => openItem(item)}
              >
                <span
                  className={`agenda-kind ${item.task ? "agenda-task" : item.creator === "User" ? "agenda-mine" : "agenda-agent"}`}
                >
                  <Icon name={item.task ? "tasks" : "calendar"} />
                </span>
                <span>
                  <strong>{item.title}</strong>
                  <small>
                    {item.time
                      ? `${timeFormat === "24" ? item.time : timeLabel(item.time)} · ${item.event?.duration} min`
                      : "All-day task deadline"}
                  </small>
                  <span className="agenda-creator">
                    {item.task
                      ? `${owner}’s task · ${item.task.done.filter(Boolean).length}/${item.task.done.length} steps`
                      : `Added by ${item.creator === "User" ? "you" : owner}`}
                  </span>
                </span>
                <Icon name="chevron" />
              </button>
            ))
          ) : (
            <div className="agenda-empty">
              <Icon name="calendar" />
              <p>A little breathing room.</p>
              <small>No events or task deadlines on this date.</small>
            </div>
          )}
          <button
            hidden={readOnlyEvents}
            className="agenda-add"
            onClick={() => openEvent()}
          >
            <Icon name="plus" />
            Add something together
          </button>
        </aside>
      </div>
      <p className="calendar-prototype-note">
        {readOnlyEvents
          ? "Live task deadlines · America/Edmonton · External calendar synchronization is not connected."
          : "Sample calendar · Today is October 16, 2026. Events stay in this tab; no calendar invitations or external updates are sent."}
      </p>
      {notice && (
        <p className="vault-status" role="status">
          {notice}
        </p>
      )}
      <dialog
        ref={dialog}
        className="prototype-dialog"
        aria-labelledby={`calendar-dialog-${owner}`}
        onCancel={() => dialog.current?.close()}
        onClick={(event) => {
          if (event.target === event.currentTarget) dialog.current?.close();
        }}
      >
        <div className="dialog-top">
          <span className="eyebrow">
            YOU & {owner.toUpperCase()} · SAMPLE CALENDAR
          </span>
          <button
            className="icon-button"
            aria-label="Close calendar dialog"
            onClick={() => dialog.current?.close()}
          >
            <Icon name="close" />
          </button>
        </div>
        <h2 id={`calendar-dialog-${owner}`}>
          {taskDetail
            ? taskDetail.title
            : editingId
              ? "Edit event"
              : "New event"}
        </h2>
        {taskDetail ? (
          <>
            <p className="dialog-description">
              A task deadline from your shared workspace.
            </p>
            <div className="document-preview">
              <p>
                Due{" "}
                {formatDate(taskDetail.dueDate, {
                  weekday: "long",
                  month: "long",
                  day: "numeric",
                })}
              </p>
              <p>
                {taskDetail.done.filter(Boolean).length} of{" "}
                {taskDetail.done.length} steps complete
              </p>
            </div>
            <div className="dialog-actions">
              <button
                className="secondary-button"
                onClick={() => dialog.current?.close()}
              >
                Close
              </button>
              <button
                className="primary-button"
                onClick={() => {
                  dialog.current?.close();
                  onOpenTasks();
                }}
              >
                Open task list
              </button>
            </div>
          </>
        ) : (
          <form
            onSubmit={(event) => {
              event.preventDefault();
              if (!title.trim()) return;
              const creator =
                events.find((item) => item.id === editingId)?.creator ?? "User";
              const next: CalendarEvent = {
                id: editingId ?? crypto.randomUUID(),
                title: title.trim(),
                date,
                time,
                duration,
                notes: notes.trim(),
                creator,
              };
              onChange(
                editingId
                  ? events.map((item) => (item.id === editingId ? next : item))
                  : [...events, next],
              );
              setSelectedDate(date);
              setMonth(`${date.slice(0, 7)}-01`);
              setNotice(
                `${editingId ? "Updated" : "Added"} sample event. Nothing was sent to an external calendar.`,
              );
              dialog.current?.close();
            }}
          >
            <label className="field-label" htmlFor={`event-title-${owner}`}>
              Event title
            </label>
            <input
              id={`event-title-${owner}`}
              className="thread-name-input"
              value={title}
              onChange={(event) => setTitle(event.target.value)}
              placeholder="What are we planning?"
              maxLength={100}
              required
            />
            <div className="calendar-form-grid">
              <div>
                <label className="field-label" htmlFor={`event-date-${owner}`}>
                  Date
                </label>
                <input
                  id={`event-date-${owner}`}
                  className="thread-name-input"
                  type="date"
                  value={date}
                  onChange={(event) => setDate(event.target.value)}
                  min="2020-01-01"
                  max="2100-12-31"
                  required
                />
              </div>
              <div>
                <label className="field-label" htmlFor={`event-time-${owner}`}>
                  Time · Edmonton
                </label>
                <input
                  id={`event-time-${owner}`}
                  className="thread-name-input"
                  type="time"
                  value={time}
                  onChange={(event) => setTime(event.target.value)}
                  required
                />
              </div>
            </div>
            <label className="field-label" htmlFor={`event-duration-${owner}`}>
              Duration
            </label>
            <select
              id={`event-duration-${owner}`}
              className="thread-name-input"
              value={duration}
              onChange={(event) => setDuration(Number(event.target.value))}
            >
              {[15, 30, 45, 60, 90, 120].map((value) => (
                <option key={value} value={value}>
                  {value} minutes
                </option>
              ))}
            </select>
            <label className="field-label" htmlFor={`event-notes-${owner}`}>
              Notes
            </label>
            <textarea
              id={`event-notes-${owner}`}
              className="thread-name-input"
              value={notes}
              onChange={(event) => setNotes(event.target.value)}
              rows={3}
              maxLength={1000}
              placeholder="Context for you and your agent"
            />
            <p className="sample-disclaimer">
              Shared with you and {owner}. Prototype only; no invitations are
              sent.
            </p>
            <div className="dialog-actions">
              <button
                type="button"
                className="secondary-button"
                onClick={() => dialog.current?.close()}
              >
                Cancel
              </button>
              <button className="primary-button">
                {editingId ? "Save event" : "Add event"}
                <Icon name="check" />
              </button>
            </div>
          </form>
        )}
      </dialog>
    </div>
  );
}
