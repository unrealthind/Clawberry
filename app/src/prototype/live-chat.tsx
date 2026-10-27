import { Markdown } from "./markdown";
import { MessageContent } from "./message-content";
import {
  useEffect,
  useLayoutEffect,
  useRef,
  useState,
  type FormEvent,
} from "react";
import type { Agent, Message } from "../api";
import { Avatar, Icon } from "./shared";

type Props = {
  owner: Agent;
  openThread: (id: string) => void;
  onAttach: () => void;
  messages: Message[];
  draft: string;
  onDraft: (value: string) => void;
  busy: boolean;
  partial: string;
  needsRefresh: boolean;
  error: string;
  onRefresh: () => void;
  onSend: (event: FormEvent) => void;
  onStop: () => void;
  profile: string;
  onProfile: (value: string) => void;
  defaultProfile: string;
  model?: string;
  historyBusy: boolean;
  historyError: string;
  hasEarlier: boolean;
  onEarlier: () => Promise<void>;
};

export function LiveChat(p: Props) {
  const name = p.owner === "janet" ? "Janet" : "Harley";
  const textarea = useRef<HTMLTextAreaElement>(null);
  const conversation = useRef<HTMLElement>(null);
  const follow = useRef(true);
  const [away, setAway] = useState(false);
  const [copyStatus, setCopyStatus] = useState("");
  const [copiedIndex, setCopiedIndex] = useState<number | null>(null);
  useEffect(() => {
    setCopyStatus("");
    setCopiedIndex(null);
  }, [p.messages]);
  const lastBusy = useRef(p.busy);
  useLayoutEffect(() => {
    const field = textarea.current;
    if (!field) return;
    field.style.height = "auto";
    field.style.height = `${Math.min(field.scrollHeight, 180)}px`;
  }, [p.draft]);
  useLayoutEffect(() => {
    if (follow.current && conversation.current)
      conversation.current.scrollTop = conversation.current.scrollHeight;
  }, [p.messages, p.partial, p.busy]);
  useEffect(() => {
    if (lastBusy.current && !p.busy)
      textarea.current?.focus({ preventScroll: true });
    lastBusy.current = p.busy;
  }, [p.busy]);
  async function earlier() {
    const panel = conversation.current;
    if (!panel) return;
    const height = panel.scrollHeight;
    const top = panel.scrollTop;
    follow.current = false;
    await p.onEarlier();
    // React commits the prepended page before the next frame.
    requestAnimationFrame(() => {
      if (conversation.current === panel)
        panel.scrollTop = top + panel.scrollHeight - height;
    });
  }
  return (
    <div className="live-chat">
      <section
        className="live-messages"
        aria-label="Live conversation"
        ref={conversation}
        onScroll={(event) => {
          const panel = event.currentTarget;
          follow.current =
            panel.scrollHeight - panel.scrollTop - panel.clientHeight < 80;
          setAway(!follow.current);
        }}
      >
        {p.historyError && <p role="alert">{p.historyError}</p>}
        {p.hasEarlier && (
          <button
            className="secondary-button live-earlier"
            disabled={p.historyBusy || p.busy}
            onClick={() => void earlier()}
          >
            {p.historyBusy
              ? "Loading earlier messages…"
              : "Load earlier messages"}
          </button>
        )}
        {!p.messages.length && !p.busy && (
          <div className="live-chat-empty">
            <Avatar agent={name} />
            <h2>What’s on your mind?</h2>
            <p>
              {name === "Janet"
                ? "Plan your work, investigate a question, or talk through an idea."
                : "Make a plan, explore an idea, or get help with your day."}
            </p>
            <div className="live-chat-prompts">
              {(name === "Janet"
                ? ["Help me plan my priorities", "Talk through a project idea"]
                : ["Help me plan my week", "Talk through a personal project"]
              ).map((text) => (
                <button
                  key={text}
                  className="secondary-button"
                  disabled={p.needsRefresh}
                  onClick={() => {
                    p.onDraft(text);
                    textarea.current?.focus();
                  }}
                >
                  {text}
                </button>
              ))}
            </div>
          </div>
        )}
        {p.messages.map((message, i) => (
          <article
            key={i}
            className={message.role === "user" ? "live-user" : "live-agent"}
          >
            <div className="live-message-heading">
              {message.role !== "user" && <Avatar agent={name} small />}
              <strong>{message.role === "user" ? "You" : name}</strong>
              {message.role !== "user" && (
                <button
                  type="button"
                  className="live-copy"
                  aria-label={`Copy reply ${i + 1}`}
                  onClick={async () => {
                    try {
                      await navigator.clipboard.writeText(message.content);
                      setCopyStatus("Reply copied.");
                      setCopiedIndex(i);
                    } catch {
                      setCopyStatus(
                        "Could not copy. Select the reply text to copy it.",
                      );
                      setCopiedIndex(null);
                    }
                  }}
                >
                  {copiedIndex === i ? "Copied" : "Copy"}
                </button>
              )}
            </div>
            {message.role === "user" ? (
              <Markdown>{message.content}</Markdown>
            ) : (
              <MessageContent
                content={message.content}
                openThread={p.openThread}
              />
            )}
          </article>
        ))}
        {p.busy && (
          <article className="live-user">
            <strong>You</strong>
            <p>{p.draft}</p>
          </article>
        )}
        {(p.busy || p.partial) && (
          <article className="live-agent live-stream">
            <div className="live-message-heading">
              <Avatar agent={name} small />
              <strong>{name}</strong>
              <span className="live-stream-label">
                {p.busy ? "Replying…" : "Unconfirmed response"}
              </span>
            </div>
            {p.partial ? (
              <MessageContent content={p.partial} openThread={p.openThread} />
            ) : (
              <p className="live-thinking">Thinking through your message…</p>
            )}
          </article>
        )}
      </section>
      {away && (
        <button
          className="secondary-button live-jump"
          onClick={() => {
            const panel = conversation.current;
            if (panel) panel.scrollTop = panel.scrollHeight;
            follow.current = true;
            setAway(false);
          }}
        >
          Jump to latest <span aria-hidden="true">↓</span>
        </button>
      )}
      <p className={copyStatus ? "live-copy-status" : "sr-only"} role="status">
        {p.busy ? `${name} is replying…` : copyStatus}
      </p>
      {p.error && (
        <div className="live-chat-error" role="alert">
          <p>{p.error}</p>
          <button className="secondary-button" onClick={p.onRefresh}>
            Refresh history
          </button>
        </div>
      )}
      <form
        className="workspace-form live-chat-form"
        onSubmit={(event) => {
          follow.current = true;
          setAway(false);
          p.onSend(event);
        }}
      >
        <label className="sr-only" htmlFor="live-message">
          Message {name}
        </label>
        <textarea
          id="live-message"
          ref={textarea}
          rows={1}
          placeholder={`Message ${name}…`}
          value={p.draft}
          maxLength={16000}
          aria-describedby="live-composer-hint"
          disabled={p.busy}
          onChange={(event) => p.onDraft(event.target.value)}
          onKeyDown={(event) => {
            if (
              event.key === "Enter" &&
              !event.shiftKey &&
              !event.nativeEvent.isComposing &&
              !matchMedia("(pointer: coarse)").matches
            ) {
              event.preventDefault();
              event.currentTarget.form?.requestSubmit();
            }
          }}
        />
        <div className="live-composer-controls">
          <button
            type="button"
            className="secondary-button live-attach"
            aria-label="Attach files"
            disabled={p.busy || p.needsRefresh}
            onClick={p.onAttach}
          >
            <Icon name="attach" />
            <span>Files</span>
          </button>
          <label className="live-profile">
            <Icon name="soul" />
            <span className="sr-only">Reasoning profile</span>
            <select
              title={
                p.model
                  ? `Configured model: ${p.model}`
                  : "Choose a reasoning profile"
              }
              value={p.profile}
              disabled={p.busy || p.needsRefresh}
              onChange={(event) => p.onProfile(event.target.value)}
            >
              <option value="default">
                Agent default ({p.defaultProfile})
              </option>
              <option value="fast">Fast</option>
              <option value="normal">Normal</option>
              <option value="deep">Deep</option>
            </select>
          </label>
          <span className="live-model-name" title={p.model}>
            {p.model}
          </span>
          {p.busy ? (
            <button
              type="button"
              className="secondary-button live-send"
              onClick={p.onStop}
            >
              Stop response
            </button>
          ) : (
            <button
              className="primary-button live-send"
              aria-label="Send message"
              disabled={p.needsRefresh || !p.draft.trim()}
            >
              <Icon name="arrow" />
              <span>Send</span>
            </button>
          )}
        </div>
      </form>
      <small id="live-composer-hint" className="live-composer-hint">
        <span className="live-keyboard-hint">
          Enter to send · Shift + Enter for a new line
        </span>
        <span className="live-touch-hint">Tap Send when you’re ready</span>
        <span>
          {p.draft.length >= 14000
            ? `${p.draft.length.toLocaleString()} / 16,000`
            : `Private to ${name}`}
        </span>
      </small>
    </div>
  );
}
