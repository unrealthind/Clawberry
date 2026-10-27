import { afterEach, expect, test, vi } from "vitest";
import {
  streamChat,
  operate,
  isLoopbackHost,
  request,
  withRequestDeadline,
} from "./api";
afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
});
function fixture(text: string) {
  const bytes = new TextEncoder().encode(text);
  vi.stubGlobal(
    "fetch",
    vi.fn(
      async () =>
        new Response(
          new ReadableStream({
            start(controller) {
              for (const byte of bytes) controller.enqueue(Uint8Array.of(byte));
              controller.close();
            },
          }),
          { status: 200 },
        ),
    ),
  );
}
test("stream handles split UTF-8 and SSE boundaries and requires completion", async () => {
  fixture(
    'data: {"type":"delta","text":"Hello 🌍"}\r\n\r\ndata: {"type":"complete","response":"Hello 🌍"}\r\n\r\n',
  );
  const deltas: string[] = [];
  expect(
    await streamChat(
      "fixture-token",
      "janet",
      { message: "hello" },
      new AbortController().signal,
      (text) => deltas.push(text),
    ),
  ).toBe("Hello 🌍");
  expect(deltas).toEqual(["Hello 🌍"]);
  const options = vi.mocked(fetch).mock.calls[0][1]!;
  expect(options.redirect).toBe("error");
  expect(options.credentials).toBe("omit");
});
test("partial text and error events cannot be mistaken for successful completion", async () => {
  fixture('data: {"type":"delta","text":"partial"}\n\n');
  await expect(
    streamChat(
      "fixture-token",
      "harley",
      {},
      new AbortController().signal,
      () => {},
    ),
  ).rejects.toThrow("before confirmation");
  fixture('data: {"type":"error","error":"Review activity"}\n\n');
  await expect(
    streamChat(
      "fixture-token",
      "harley",
      {},
      new AbortController().signal,
      () => {},
    ),
  ).rejects.toThrow("Review activity");
});

test("provider failures use fixed guidance without exposing provider error bodies", async () => {
  fixture(
    'data: {"type":"error","code":"provider_auth","error":"private provider response"}\n\n',
  );
  await expect(
    streamChat("", "janet", {}, new AbortController().signal, () => {}),
  ).rejects.toThrow("Claude rejected the credential");
});

test("local requests omit bearer headers and loopback detection excludes remote names", async () => {
  fixture('data: {"type":"complete","response":"Local reply"}\n\n');
  await streamChat(
    "",
    "janet",
    { message: "hello" },
    new AbortController().signal,
    () => {},
  );
  expect(vi.mocked(fetch).mock.calls[0][1]?.headers).not.toHaveProperty(
    "Authorization",
  );
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => new Response("[]", { status: 200 })),
  );
  await operate("", "janet", "task.list");
  expect(vi.mocked(fetch).mock.calls[0][1]?.headers).not.toHaveProperty(
    "Authorization",
  );
  for (const host of ["localhost", "127.0.0.1", "[::1]"])
    expect(isLoopbackHost(host)).toBe(true);
  for (const host of ["localhost.evil.example", "192.168.1.10", "example.com"])
    expect(isLoopbackHost(host)).toBe(false);
});

test("a complete event finishes without waiting for the server to close its stream", async () => {
  let cancelled = false;
  vi.stubGlobal(
    "fetch",
    vi.fn(
      async () =>
        new Response(
          new ReadableStream({
            start(controller) {
              controller.enqueue(
                new TextEncoder().encode(
                  'data: {"type":"complete","response":"done"}\n\n',
                ),
              );
            },
            cancel() {
              cancelled = true;
            },
          }),
        ),
    ),
  );
  const result = await Promise.race([
    streamChat("", "janet", {}, new AbortController().signal, () => {}),
    new Promise((resolve) =>
      setTimeout(() => resolve("still waiting for EOF"), 100),
    ),
  ]);
  expect(result).toBe("done");
  expect(cancelled).toBe(true);
});

test("oversized complete events are rejected before parsing or displaying content", async () => {
  vi.stubGlobal(
    "fetch",
    vi.fn(
      async () =>
        new Response(
          "data: " +
            JSON.stringify({
              type: "complete",
              response: "x".repeat(1_000_001),
            }) +
            "\n\n",
        ),
    ),
  );
  await expect(
    streamChat("", "janet", {}, new AbortController().signal, () => {}),
  ).rejects.toThrow("limit");
});

test("stalled JSON mutations time out without automatic replay", async () => {
  vi.useFakeTimers();
  vi.stubGlobal(
    "fetch",
    vi.fn(
      (_path, options) =>
        new Promise((_resolve, reject) => {
          options.signal.addEventListener(
            "abort",
            () => reject(options.signal.reason),
            { once: true },
          );
        }),
    ),
  );
  const pending = expect(
    request("fixture-token", "/api/agents/janet/operations", {
      operation: "task.create",
    }),
  ).rejects.toThrow("timed out");
  await vi.advanceTimersByTimeAsync(45_000);
  await pending;
  expect(fetch).toHaveBeenCalledTimes(1);
});

test("request deadlines forward caller cancellation and release their timers", async () => {
  vi.useFakeTimers();
  const controller = new AbortController();
  const action = withRequestDeadline(
    controller.signal,
    async (signal) =>
      new Promise((_resolve, reject) => {
        signal.addEventListener("abort", () => reject(signal.reason), {
          once: true,
        });
      }),
  );
  const rejected = expect(action).rejects.toThrow("cancelled fixture");
  controller.abort(new Error("cancelled fixture"));
  await rejected;
  expect(vi.getTimerCount()).toBe(0);
});

test("many individually valid SSE events cannot grow an unbounded response", async () => {
  const event =
    "data: " +
    JSON.stringify({ type: "delta", text: "x".repeat(600_000) }) +
    "\n\n";
  vi.stubGlobal(
    "fetch",
    vi.fn(async () => new Response(event + event)),
  );
  const delta = vi.fn();
  await expect(
    streamChat("", "janet", {}, new AbortController().signal, delta),
  ).rejects.toThrow("limit");
  expect(delta).toHaveBeenCalledTimes(1);
});
