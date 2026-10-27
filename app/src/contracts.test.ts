import { expect, test } from "vitest";
import { assertEndpointContract, assertReadContract } from "./contracts";
test("read contracts reject malformed approvals, tasks, pages and notification evidence", () => {
  for (const [operation, value] of [
    ["write.reviews", {}],
    ["write.reviews", [{ id: "missing-manifest" }]],
    ["task.list", [null]],
    ["task.list", [{ id: "missing-status" }]],
    ["shared.page", { items: [null], next_cursor: null }],
    [
      "notification.list",
      {
        items: [{ id: "missing-evidence-type", evidence: "bad" }],
        next_cursor: null,
      },
    ],
    ["chat.page", { items: [], next_cursor: "bad" }],
  ] as const)
    expect(() => assertReadContract(operation, value)).toThrow(
      "invalid response",
    );
});
test("read contracts accept additive metadata and reject incomplete creation acknowledgements", () => {
  expect(() =>
    assertReadContract("thread.list", [
      {
        id: "owned-thread",
        title: "Project",
        summary: "Evidence",
        owner_agent: "janet",
        future_metadata: true,
      },
    ]),
  ).not.toThrow();
  expect(() =>
    assertReadContract("task.create", { id: "created-task" }),
  ).toThrow("invalid response");
});

test("task details and session status validate nested evidence and budgets", () => {
  for (const [operation, value] of [
    ["task.get", { task: {}, events: [], workers: [], attachments: [] }],
    [
      "write.status",
      { expires_at: 999, revoked: false, remaining: { move: -1 }, actions: [] },
    ],
    [
      "write.status",
      {
        expires_at: 999,
        revoked: false,
        remaining: {},
        actions: [{ id: "a" }],
      },
    ],
    ["task.page", { items: [], next_cursor: "bad" }],
  ] as const)
    expect(() => assertReadContract(operation, value)).toThrow(
      "invalid response",
    );
  expect(() =>
    assertReadContract("write.status", {
      expires_at: 999,
      revoked: false,
      remaining: { move: 0 },
      actions: [],
      additive: true,
    }),
  ).not.toThrow();
});
test("settings and authorization acknowledgements fail closed on incomplete responses", () => {
  for (const [path, value] of [
    ["/api/settings/global", { revision: 1, value: { preferences: {} } }],
    [
      "/api/agents/janet/settings",
      { revision: 1, value: { soul: "", skills: [null] } },
    ],
    ["/api/runtime/status", { credential_configured: "yes" }],
    [
      "/api/agents/janet/write-authorizations",
      { authorized: false, session_id: "s" },
    ],
    [
      "/api/agents/harley/write-authorizations",
      { authorized: true, session_id: "" },
    ],
  ] as const)
    expect(() => assertEndpointContract(path, value)).toThrow(
      "invalid response",
    );
  expect(() =>
    assertEndpointContract("/api/agents/janet/write-authorizations", {
      authorized: true,
      session_id: "s",
      future: true,
    }),
  ).not.toThrow();
});
