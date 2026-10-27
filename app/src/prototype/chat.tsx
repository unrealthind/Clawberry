import { Markdown } from "./markdown";
import { createContext, useContext, useMemo, useRef } from "react";
import {
  AssistantRuntimeProvider,
  ComposerPrimitive,
  MessagePrimitive,
  ThreadPrimitive,
  useAuiState,
  useLocalRuntime,
  type ChatModelAdapter,
} from "@assistant-ui/react";
import { Avatar, Icon, starters, type Agent } from "./shared";

const ChatContext = createContext({
  agent: "Janet" as Agent,
  search: "",
  onReview: () => {},
});
function ChatMessage({ user = false }: { user?: boolean }) {
  const { agent, search, onReview } = useContext(ChatContext);
  const text = useAuiState((state) =>
    state.message.content
      .filter((part) => part.type === "text")
      .map((part) => part.text)
      .join("\n"),
  );
  return (
    <MessagePrimitive.Root
      className={`chat-message ${user ? "from-user" : "from-agent"}`}
      hidden={!!search && !text.toLowerCase().includes(search.toLowerCase())}
    >
      {!user && <Avatar agent={agent} small />}
      <div className="message-content">
        <div className="message-meta">
          <strong>{user ? "You" : agent}</strong>
          <span>
            {user
              ? "User"
              : agent === "Janet"
                ? "WORK AGENT"
                : "PERSONAL AGENT"}
          </span>
        </div>
        <div className="message-bubble">
          <MessagePrimitive.Parts
            components={{ Text: ({ text }) => <Markdown>{text}</Markdown> }}
          />
          {!user &&
            /pulled everything into a draft|mountain escape plan|sample draft for your review|drafted the details/i.test(
              text,
            ) && (
              <button className="file-card" onClick={onReview}>
                <span className="file-icon">
                  <Icon name="file" />
                </span>
                <span>
                  <strong>
                    {agent === "Janet"
                      ? "v0.2 Release Plan"
                      : "Mountain Weekend"}
                  </strong>
                  <small>Sample draft · Ready to review</small>
                </span>
                <span className="file-open">
                  Open
                  <Icon name="chevron" />
                </span>
              </button>
            )}
        </div>
        <MessagePrimitive.Error>
          <span className="chat-error">
            The sample response stopped. Try sending another message.
          </span>
        </MessagePrimitive.Error>
      </div>
    </MessagePrimitive.Root>
  );
}
const UserMessage = () => <ChatMessage user />;
const AssistantMessage = () => <ChatMessage />;

function makeAdapter(
  agent: Agent,
  onRunning: (running: boolean) => void,
): ChatModelAdapter {
  return {
    async *run({ messages, abortSignal }) {
      onRunning(true);
      try {
        const last =
          messages
            .at(-1)
            ?.content.filter((part) => part.type === "text")
            .map((part) => part.text)
            .join(" ") ?? "";
        const text = /send|invite|book|schedule/i.test(last)
          ? agent === "Janet"
            ? "I’ve prepared a sample draft for your review. In the connected app, you’ll authorize any external action through the dedicated approval flow. Nothing has been sent."
            : "A plan? Love it. I’ve drafted the details — you get the final say before anything gets booked or sent. For now, this is all make-believe."
          : /task|demo|priorit|work/i.test(last)
            ? agent === "Janet"
              ? "Happy to help! The release plan is ready for review, the demo agenda is drafted, and infrastructure costs are next. I suggest we start with the release scope. One clear decision, then onward."
              : "Let’s make that list less intimidating. Pick one small thing, finish it, and claim the win. The weekend adventure is already drafted — priorities, obviously."
            : agent === "Janet"
              ? "Of course, User. Let’s give that a little structure: what would a useful result look like? We can keep the next steps here, or open a Focus Thread if it needs its own space. This reply is scripted for the UI prototype."
              : "I’m in. Let’s turn that into a tiny, achievable mission. What’s the first move? We can keep plotting here or give it a Focus Thread. This is a scripted prototype reply — real chaos management comes later.";
        const words = text.split(" ");
        for (let index = 0; index < words.length; index += 3) {
          if (abortSignal.aborted) return;
          await new Promise<void>((resolve) => {
            const finish = () => {
              abortSignal.removeEventListener("abort", cancel);
              resolve();
            };
            const timer = setTimeout(finish, 35);
            const cancel = () => {
              clearTimeout(timer);
              finish();
            };
            abortSignal.addEventListener("abort", cancel, { once: true });
          });
          if (abortSignal.aborted) return;
          yield {
            content: [
              { type: "text", text: words.slice(0, index + 3).join(" ") },
            ],
          };
        }
      } finally {
        onRunning(false);
      }
    },
  };
}
export function PrototypeChat({
  agent,
  thread,
  search,
  onReview,
  onAttach,
  onRunningChange,
}: {
  agent: Agent;
  thread: string | null;
  search: string;
  onReview: () => void;
  onAttach: (names: string[]) => void;
  onRunningChange: (session: string, running: boolean) => void;
}) {
  const adapter = useMemo(
    () =>
      makeAdapter(agent, (running) =>
        onRunningChange(`${agent}:${thread ?? "main"}`, running),
      ),
    [agent, thread, onRunningChange],
  );
  const initialMessages = useMemo(
    () =>
      (thread
        ? [
            {
              user: false,
              text:
                agent === "Janet"
                  ? `Welcome to ${thread}. Everything for this project has a home here. What shall we work through first?`
                  : `${thread}: officially a mission. What are we tackling first?`,
            },
          ]
        : starters[agent]
      ).map((message) => ({
        role: message.user ? ("user" as const) : ("assistant" as const),
        content: [{ type: "text" as const, text: message.text }],
      })),
    [agent, thread],
  );
  const runtime = useLocalRuntime(adapter, { initialMessages });
  const upload = useRef<HTMLInputElement>(null);
  return (
    <AssistantRuntimeProvider runtime={runtime}>
      <ChatContext.Provider value={{ agent, search, onReview }}>
        <ThreadPrimitive.Root className="prototype-thread">
          <div className="conversation-toolbar">
            <span>
              <Icon name={thread ? "thread" : "chat"} />
              {thread ?? "Main conversation"}
            </span>
            <span className="private-label">
              <Icon name="shield" />
              Private to {agent}
            </span>
          </div>
          <ThreadPrimitive.Viewport className="conversation" autoScroll>
            <div className="conversation-inner">
              <div className="date-divider">
                <span>Today, October 16</span>
              </div>
              <ThreadPrimitive.Messages
                components={{ UserMessage, AssistantMessage }}
              />
              <ThreadPrimitive.If running>
                <div className="typing-indicator" role="status">
                  {agent} is replying<span>•••</span>
                </div>
              </ThreadPrimitive.If>
            </div>
          </ThreadPrimitive.Viewport>
          <div className="composer-area">
            <input
              ref={upload}
              type="file"
              multiple
              hidden
              onChange={(event) => {
                const names = Array.from(event.target.files ?? []).map(
                  (file) => file.name,
                );
                if (names.length) onAttach(names);
                event.target.value = "";
              }}
            />
            <ComposerPrimitive.Root className="composer">
              <button
                type="button"
                className="icon-button"
                aria-label="Attach sample file"
                onClick={() => upload.current?.click()}
              >
                <Icon name="attach" />
              </button>
              <ComposerPrimitive.Input
                className="composer-input"
                aria-label={`Message ${agent}`}
                placeholder={`Message ${agent}…`}
                autoFocus={false}
              />
              <ThreadPrimitive.If running={false}>
                <ComposerPrimitive.Send
                  className="send-button"
                  aria-label="Send sample message"
                >
                  <Icon name="arrow" />
                </ComposerPrimitive.Send>
              </ThreadPrimitive.If>
              <ThreadPrimitive.If running>
                <ComposerPrimitive.Cancel
                  className="send-button"
                  aria-label="Stop sample response"
                >
                  <Icon name="close" />
                </ComposerPrimitive.Cancel>
              </ThreadPrimitive.If>
            </ComposerPrimitive.Root>
            <div className="composer-caption">
              <span>UI prototype · Scripted replies · No live actions</span>
              <span>↵ to send</span>
            </div>
          </div>
        </ThreadPrimitive.Root>
      </ChatContext.Provider>
    </AssistantRuntimeProvider>
  );
}
