import { assertEndpointContract, assertReadContract } from "./contracts";
export function isLoopbackHost(hostname: string) {
  return ["localhost", "127.0.0.1", "[::1]", "::1"].includes(
    hostname.toLowerCase(),
  );
}

export type Agent = "janet" | "harley";
export type Message = { role: string; content: string };

export class ChatProviderError extends Error {}

const providerFailures: Record<string, string> = {
  provider_auth:
    "Claude rejected the credential. It may be expired or lack access. Update it in App settings → Claude & models.",
  provider_billing:
    "The model provider rejected billing. Check the provider account's credits and billing settings.",
  provider_model:
    "The configured model is unavailable. Check the model in App settings → Claude & models.",
  provider_rate_limit:
    "The model provider's rate limit was reached. Wait before retrying.",
  provider_request:
    "The model provider rejected the request. Check the configured model and inspect Activity.",
  provider_unavailable: "The model provider is unavailable. Try again later.",
  provider_connection:
    "Could not connect to the model provider. Check the gateway's network and TLS configuration.",
};
export type Thread = { id: string; title: string; summary: string };
export type Task = {
  id: string;
  title: string;
  description: string;
  status: string;
  thread_id: string | null;
  priority: "low" | "normal" | "high" | "urgent";
  due_at: string | null;
};
export type Attachment = {
  id: string;
  filename: string;
  checksum: string;
  size: number;
  source: string;
};
export type Fact = {
  content: string;
  category: string;
  confidence: number;
  source_type: string;
  source_ref: string;
  sensitivity: string;
  expires_at: string | null;
};
export type Memory = Fact & {
  id: string;
  agent_owner: Agent;
  visibility: string;
  stale: boolean;
  created_at: string;
  last_verified_at: string | null;
  review_hash: string;
};
export type Activity = {
  events: {
    request_id: string;
    event: string;
    detail: string;
    created_at: string;
  }[];
  workers: { id: string; role?: string; status: string; task_id: string }[];
};

export async function request<T>(
  token: string,
  path: string,
  body?: unknown,
  signal?: AbortSignal,
): Promise<T> {
  if (!path.startsWith("/api/") || path.includes("://"))
    throw new Error("Invalid API path");
  return withRequestDeadline(signal, async (boundedSignal) => {
    const response = await fetch(path, {
      method: body === undefined ? "GET" : "POST",
      signal: boundedSignal,
      headers: {
        ...(token ? { Authorization: `Bearer ${token}` } : {}),
        ...(body === undefined ? {} : { "Content-Type": "application/json" }),
      },
      body: body === undefined ? undefined : JSON.stringify(body),
      credentials: "omit",
      redirect: "error",
      cache: "no-store",
    });
    const value = await response.json();
    if (!response.ok)
      throw new Error(value.error || `Request failed (${response.status})`);
    assertEndpointContract(path, value);
    return value as T;
  });
}
export function operate<T>(
  token: string,
  owner: Agent,
  operation: string,
  fields: object = {},
  signal?: AbortSignal,
) {
  return request<T>(
    token,
    `/api/agents/${owner}/operations`,
    { operation, ...fields },
    signal,
  ).then((value) => {
    assertReadContract(operation, value);
    return value;
  });
}

async function streamChatRequest(
  token: string,
  owner: Agent,
  body: object,
  signal: AbortSignal,
  delta: (text: string) => void,
): Promise<string> {
  const response = await fetch(`/api/chat/${owner}/stream`, {
    method: "POST",
    signal,
    headers: {
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
    credentials: "omit",
    redirect: "error",
    cache: "no-store",
  });
  if (!response.ok || !response.body)
    throw new Error(
      `Chat failed (${response.status}). Refresh history before retrying.`,
    );
  const reader = response.body.getReader();
  const decoder = new TextDecoder();
  let pending = "";
  let ended = false;
  let receivedText = 0;
  try {
    for (;;) {
      const { value, done } = await reader.read();
      pending += done
        ? decoder.decode()
        : decoder.decode(value, { stream: true });
      pending = pending.replace(/\r\n/g, "\n");
      let boundary: number;
      while ((boundary = pending.indexOf("\n\n")) >= 0) {
        if (boundary > 1_000_000) throw new Error("Chat event exceeds limit");
        const event = pending.slice(0, boundary);
        pending = pending.slice(boundary + 2);
        const data = event
          .split("\n")
          .filter((line) => line.startsWith("data:"))
          .map((line) => line.slice(5).trimStart())
          .join("\n");
        if (!data) continue;
        const item = JSON.parse(data);
        if (item.type === "delta" && typeof item.text === "string") {
          receivedText += item.text.length;
          if (receivedText > 1_000_000)
            throw new Error("Chat response exceeds limit");
          delta(item.text);
        } else if (
          item.type === "complete" &&
          typeof item.response === "string"
        )
          return item.response;
        else if (item.type === "error") {
          const message =
            typeof item.code === "string" &&
            Object.hasOwn(providerFailures, item.code)
              ? providerFailures[item.code]
              : undefined;
          if (message) throw new ChatProviderError(message);
          throw new Error(item.error || "Chat failed");
        }
      }
      if (pending.length > 1_000_000)
        throw new Error("Chat event exceeds limit");
      if (done) {
        ended = true;
        break;
      }
    }
  } finally {
    if (!ended) void reader.cancel().catch(() => {});
    reader.releaseLock();
  }
  throw new Error(
    "Connection ended before confirmation. Refresh history before retrying.",
  );
}

// Deadline includes body consumption. Aborting a mutation never implies it was
// rolled back: callers retain their existing unconfirmed-result recovery flow.
export async function withRequestDeadline<T>(
  signal: AbortSignal | undefined,
  action: (signal: AbortSignal) => Promise<T>,
  timeoutMs = 45_000,
): Promise<T> {
  const controller = new AbortController();
  const abort = () => controller.abort(signal?.reason);
  if (signal?.aborted) abort();
  else signal?.addEventListener("abort", abort, { once: true });
  const timer = setTimeout(
    () =>
      controller.abort(
        new DOMException(
          "Request timed out; refresh before retrying.",
          "TimeoutError",
        ),
      ),
    timeoutMs,
  );
  try {
    return await action(controller.signal);
  } finally {
    clearTimeout(timer);
    signal?.removeEventListener("abort", abort);
  }
}

export function streamChat(
  token: string,
  owner: Agent,
  body: object,
  signal: AbortSignal,
  delta: (text: string) => void,
): Promise<string> {
  // Long reasoning/tool turns may outlast ordinary reads, but cannot hang forever.
  return withRequestDeadline(
    signal,
    (boundedSignal) =>
      streamChatRequest(token, owner, body, boundedSignal, delta),
    600_000,
  );
}
