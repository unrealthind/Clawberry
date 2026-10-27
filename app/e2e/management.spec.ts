import { fulfillFixture } from "./api-fixture";
import { test, expect, type Page } from "@playwright/test";
type Body = { operation: string; [key: string]: any };
async function fixture(
  page: Page,
  handler: (owner: string, body: Body) => unknown = () => undefined,
) {
  await page.route("**/api/settings/global", (route) =>
    fulfillFixture(route, {
      contentType: "application/json",
      body: JSON.stringify({
        revision: 0,
        value: {
          preferences: {
            theme: "light",
            notifications: true,
            week_start: "sunday",
            time_format: "12",
            show_completed: true,
          },
          models: {
            profiles: Object.fromEntries(
              ["fast", "normal", "deep"].map((name) => [
                name,
                {
                  provider: "anthropic",
                  model: "claude-sonnet-5-5",
                  max_output_tokens: 4096,
                  input_microusd_per_million: 2000000,
                  output_microusd_per_million: 10000000,
                },
              ]),
            ),
            agent_daily_microusd: 5000000,
            deep_daily_microusd: 2000000,
            task_microusd: 2000000,
          },
        },
      }),
    }),
  );
  await page.route("**/api/runtime/status", (route) =>
    fulfillFixture(route, {
      contentType: "application/json",
      body: JSON.stringify({
        credential_configured: true,
        credential_source: "environment",
        live_verified: false,
      }),
    }),
  );
  await page.route("**/api/agents/*/settings", (route) =>
    fulfillFixture(route, {
      contentType: "application/json",
      body: JSON.stringify({
        revision: 0,
        value: {
          soul: "",
          memory_notes: "",
          default_profile: "normal",
          skills: [],
        },
      }),
    }),
  );
  await page.route("**/api/agents/*/operations", async (route) => {
    const body = route.request().postDataJSON(),
      owner = route.request().url().includes("/janet/") ? "janet" : "harley";
    expect(body).not.toHaveProperty("code");
    expect(route.request().headers().authorization).toBeUndefined();
    let value = await handler(owner, body);
    if (value === undefined)
      value = [
        "chat.page",
        "memory.page",
        "shared.page",
        "notification.list",
      ].includes(body.operation)
        ? { items: [], next_cursor: null }
        : body.operation === "activity.list"
          ? { events: [], workers: [] }
          : body.operation === "watch.settings"
            ? {
                enabled: false,
                interval_minutes: 60,
                quiet_start_utc: 0,
                quiet_end_utc: 0,
                daily_runs: 2,
                daily_budget_microusd: 100000,
                investigation_budget_microusd: 50000,
                stale_after_hours: 48,
              }
            : [];
    await fulfillFixture(route, {
      contentType: "application/json",
      body: JSON.stringify(value),
    });
  });
}
async function nav(page: Page, name: string) {
  const menu = page.getByRole("button", { name: "Open navigation" });
  if (
    (await menu.isVisible()) &&
    (await menu.getAttribute("aria-expanded")) !== "true"
  )
    await menu.click();
  await page
    .locator(".sidebar")
    .getByRole("button", { name, exact: true })
    .click();
}
const review = () => ({
  id: "review-1",
  task_id: "task-1",
  workflow: "test-email",
  manifest: {
    mode: "bounded_session",
    budgets: { "email.move": 2 },
    mailboxes: ["INBOX", "Archive"],
    message_uids: { INBOX: [1] },
    archive_mailbox: "Archive",
  },
  manifest_hash: "exact-manifest-hash",
  expires_at: Math.floor(Date.now() / 1000) + 300,
  state: "pending",
});

test("root and prototype share one shell and approvals stay inside it", async ({
  page,
}, info) => {
  await fixture(page);
  const root = await page.request.get("/"),
    alias = await page.request.get("/prototype.html");
  const script = /<script[^>]+src="([^"]+)"/;
  expect((await root.text()).match(script)?.[1]).toBe(
    (await alias.text()).match(script)?.[1],
  );
  for (const path of ["/", "/prototype.html"]) {
    await page.goto(path);
    await expect(page.locator(".workspace-shell")).toBeVisible();
    await expect(page.locator(".app")).toHaveCount(0);
    await nav(page, "Approvals");
    await expect(
      page.getByRole("heading", { name: "Approvals", exact: true, level: 1 }),
    ).toBeVisible();
    await expect(
      page.getByText("No write requests", { exact: true }),
    ).toBeVisible();
    expect(new URL(page.url()).pathname).toBe(path);
    await expect(
      page.getByRole("link", { name: "Open secure approvals" }),
    ).toHaveCount(0);
  }
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({ path: info.outputPath("unified-approvals.png") });
});

test("write approval uses dedicated TOTP transport, shows budgets and ends sessions", async ({
  page,
}, info) => {
  const row = review(),
    session = {
      expires_at: Math.floor(Date.now() / 1000) + 300,
      revoked: false,
      gateway_end_confirmed: null as boolean | null,
      remaining: { "email.move": 1 },
      actions: [
        {
          id: "fixture-action",
          operation: "email.move",
          status: "succeeded",
          gateway_action_id: "fixture-action",
          gateway_audit_id: "fixture-audit",
          n8n_execution_id: "fixture-execution",
        },
      ],
    };
  const operations: Body[] = [];
  await fixture(page, (owner, body) => {
    operations.push(body);
    if (body.operation === "write.reviews")
      return owner === "janet" ? [row] : [];
    if (body.operation === "write.status") return session;
    if (body.operation === "write.end") {
      session.revoked = true;
      session.gateway_end_confirmed = false;
      return { revoked: true, gateway_end_confirmed: false };
    }
  });
  let requests = 0;
  await page.route(
    "**/api/agents/janet/write-authorizations",
    async (route) => {
      requests++;
      expect(route.request().postDataJSON()).toEqual({
        review_id: row.id,
        reviewed_hash: row.manifest_hash,
        code: "918273",
      });
      row.state = "authorized";
      await fulfillFixture(route, {
        contentType: "application/json",
        body: JSON.stringify({ session_id: row.id, authorized: true }),
      });
    },
  );
  await page.goto("/");
  await nav(page, "Approvals");
  const card = page.getByRole("article", { name: "test-email approval" });
  await expect(card).toContainText("Bounded session");
  await expect(card).toContainText('"email.move": 2');
  await expect(
    card.getByRole("button", { name: "Authorize with TOTP" }),
  ).toBeDisabled();
  await card.getByLabel("TOTP authorization code").fill("918273");
  await expect(
    card.getByRole("button", { name: "Authorize with TOTP" }),
  ).toBeDisabled();
  await card.getByRole("checkbox").check();
  await card.getByRole("button", { name: "Authorize with TOTP" }).click();
  await expect(card.getByText("Active", { exact: true })).toBeVisible();
  await expect(card.getByLabel("TOTP authorization code")).toHaveCount(0);
  expect(requests).toBe(1);
  await expect(card).toContainText("Remaining operation budgets");
  await expect(card).toContainText("Audit fixture-audit");
  await expect(card).toContainText("n8n fixture-execution");
  await card.getByRole("button", { name: "End session", exact: true }).click();
  await expect(card.getByText("Ended", { exact: true })).toBeVisible();
  await expect(card).toContainText("Gateway closure is unconfirmed");
  expect(operations).toContainEqual({
    operation: "write.end",
    task_id: row.task_id,
    session_id: row.id,
  });
  expect(
    await page.evaluate(() => [localStorage.length, sessionStorage.length]),
  ).toEqual([0, 0]);
  await page.screenshot({ path: info.outputPath("unified-write-session.png") });
  await nav(page, "Harley");
  await nav(page, "Approvals");
  await expect(
    page.getByRole("article", { name: "test-email approval" }),
  ).toHaveCount(0);
});

test("uncertain TOTP submission clears codes and requires refreshed authority", async ({
  page,
}) => {
  const row = review();
  let requests = 0;
  await fixture(page, (_owner, body) =>
    body.operation === "write.reviews"
      ? [row]
      : body.operation === "write.status"
        ? {
            expires_at: row.expires_at,
            revoked: false,
            remaining: { "email.move": 2 },
            actions: [],
          }
        : undefined,
  );
  await page.route(
    "**/api/agents/janet/write-authorizations",
    async (route) => {
      requests++;
      row.state = "authorized";
      await route.abort();
    },
  );
  await page.goto("/prototype.html");
  await nav(page, "Approvals");
  await page.getByLabel("TOTP authorization code").fill("918273");
  await page.getByRole("checkbox").check();
  await page.getByRole("button", { name: "Authorize with TOTP" }).click();
  await expect(page.getByRole("alert")).toContainText("not confirmed");
  await expect(page.getByLabel("TOTP authorization code")).toHaveValue("");
  await expect(
    page.getByRole("button", { name: "Authorize with TOTP" }),
  ).toBeDisabled();
  await page
    .getByRole("button", { name: "Refresh approvals", exact: true })
    .click();
  await expect(page.getByText("Active", { exact: true })).toBeVisible();
  expect(requests).toBe(1);
  await nav(page, "Chat");
  await nav(page, "Approvals");
  await expect(page.getByLabel("TOTP authorization code")).toHaveCount(0);
});

test("peer review binds the exact response and expired writes cannot authorize", async ({
  page,
}) => {
  const expired = {
    ...review(),
    manifest: {
      mode: "single_action",
      operation: "email.flag",
      payload: { uid: 1 },
    },
    expires_at: Math.floor(Date.now() / 1000) - 1,
  };
  const peer = {
    id: "peer-1",
    requester: "janet",
    recipient: "harley",
    task_id: "task-1",
    question: "Share the approved meeting hour",
    question_hash: "question-binding",
    response: "The meeting is at 14:00",
    response_hash: "response-binding",
    status: "response_pending",
    expires_at: Math.floor(Date.now() / 1000) + 300,
  };
  const decisions: Body[] = [];
  await fixture(page, (owner, body) => {
    if (body.operation === "write.reviews") return [expired];
    if (body.operation === "cross.review_queue")
      return owner === "janet" ? [peer] : [];
    if (body.operation === "cross.review") {
      decisions.push(body);
      peer.status = "closed";
      return { reviewed: true };
    }
  });
  await page.goto("/");
  await nav(page, "Approvals");
  await expect(
    page.getByRole("heading", {
      name: "Single action · test-email",
      exact: true,
    }),
  ).toBeVisible();
  await expect(page.getByLabel("TOTP authorization code")).toHaveCount(0);
  const card = page.getByRole("article", { name: "Cross-agent request" });
  await expect(card).toContainText(peer.response);
  await expect(
    card.getByRole("button", { name: "Approve response" }),
  ).toBeDisabled();
  await card.getByRole("checkbox").check();
  await card.getByRole("button", { name: "Approve response" }).click();
  expect(decisions).toEqual([
    {
      operation: "cross.review",
      id: peer.id,
      reviewed_hash: peer.response_hash,
      decision: "approve_response",
    },
  ]);
  await expect(
    card.getByRole("button", { name: "Approve response" }),
  ).toHaveCount(0);
});

test("private memory edits, exact sharing, revocation and deletion use the current shell", async ({
  page,
}, info) => {
  let memory: any = {
    id: "memory-1",
    agent_owner: "janet",
    content: "Project uses Rust",
    category: "project",
    confidence: 0.8,
    source_type: "user",
    source_ref: "message:1",
    sensitivity: "ordinary",
    visibility: "private",
    stale: false,
    created_at: "2026-10-19",
    last_verified_at: null,
    expires_at: null,
    review_hash: "memory-binding",
  };
  let shared: any[] = [];
  const operations: Body[] = [];
  await fixture(page, (owner, body) => {
    operations.push(body);
    if (body.operation === "memory.page")
      return {
        items:
          owner === "janet" &&
          memory &&
          (!body.filter.query || memory.content.includes(body.filter.query))
            ? [memory]
            : [],
        next_cursor: null,
      };
    if (body.operation === "shared.page")
      return { items: shared, next_cursor: null };
    if (body.operation === "memory.save") {
      memory = { ...memory, ...body.fact, review_hash: "edited-binding" };
      return { saved: true };
    }
    if (body.operation === "memory.share") {
      shared = [{ ...memory }];
      return { shared: true };
    }
    if (
      ["shared.save", "shared.verify", "shared.stale"].includes(body.operation)
    ) {
      expect(body.reviewed_hash).toBe(shared[0].review_hash);
      shared = [
        {
          ...shared[0],
          ...(body.fact ?? {}),
          stale: body.operation === "shared.stale",
          review_hash: `${body.operation}-binding`,
        },
      ];
      return { saved: true };
    }
    if (body.operation === "memory.unshare") {
      shared = [];
      return { revoked: true };
    }
    if (body.operation === "memory.delete") {
      memory = null;
      return { deleted: true };
    }
  });
  await page.goto("/");
  await nav(page, "Memory");
  await page
    .locator(".memory-record")
    .getByRole("button", { name: "Edit", exact: true })
    .click();
  const dialog = page.getByRole("dialog", { name: "Edit memory" });
  await dialog
    .getByRole("textbox", { name: "Fact", exact: true })
    .fill("Project uses Rust and SQLite");
  await dialog.getByLabel("Confidence (0–1)").fill("0.95");
  await dialog
    .getByLabel("Expiry (RFC 3339, optional)")
    .fill("2035-01-01T00:00:00Z");
  await dialog.getByRole("button", { name: "Save changes" }).click();
  await expect(dialog).toHaveCount(0);
  await expect(page.locator(".memory-record")).toContainText("95%");
  expect(
    operations.find((b) => b.operation === "memory.save").fact.expires_at,
  ).toBe("2035-01-01T00:00:00Z");
  await page
    .locator(".memory-record")
    .getByRole("button", { name: "Share", exact: true })
    .click();
  const share = page.getByRole("dialog", {
    name: "Share this fact with Janet and Harley?",
  });
  await expect(share).toContainText("Project uses Rust and SQLite");
  await share.getByRole("button", { name: "Share exact fact" }).click();
  expect(operations).toContainEqual({
    operation: "memory.share",
    id: "memory-1",
    reviewed_hash: "edited-binding",
  });
  await expect(
    page.getByRole("button", { name: "Revoke shared copy" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Edit shared fact", exact: true })
    .click();
  const sharedEditor = page.getByRole("dialog", { name: "Edit shared fact" });
  await sharedEditor
    .getByRole("textbox", { name: "Fact", exact: true })
    .fill("Shared project summary");
  await sharedEditor.getByLabel("Expiry (RFC 3339, optional)").fill("");
  await sharedEditor.getByRole("button", { name: "Save changes" }).click();
  await expect(sharedEditor).toHaveCount(0);
  await expect(page.locator(".memory-record")).toContainText(
    "Project uses Rust and SQLite",
  );
  expect(operations.find((b) => b.operation === "shared.save")).toMatchObject({
    id: "memory-1",
    reviewed_hash: "edited-binding",
    fact: { content: "Shared project summary", expires_at: null },
  });
  for (const [button, title, confirm, operation] of [
    [
      "Verify shared fact",
      "Verify this shared fact?",
      "Verify exact shared fact",
      "shared.verify",
    ],
    [
      "Mark shared fact stale",
      "Mark this shared fact stale?",
      "Mark exact shared fact stale",
      "shared.stale",
    ],
  ]) {
    await page.getByRole("button", { name: button, exact: true }).click();
    const review = page.getByRole("dialog", { name: title });
    await expect(review).toContainText("Shared project summary");
    await review.getByRole("button", { name: confirm, exact: true }).click();
    await expect(review).toHaveCount(0);
    expect(operations.some((b) => b.operation === operation)).toBe(true);
  }
  await nav(page, "Harley");
  await nav(page, "Memory");
  await expect(
    page.getByText("Shared project summary", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Edit shared fact", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Verify shared fact", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Mark shared fact stale", exact: true }),
  ).toHaveCount(0);
  await nav(page, "Janet");
  await nav(page, "Memory");
  await page.getByRole("button", { name: "Revoke shared copy" }).click();
  await expect(
    page.getByRole("button", { name: "Revoke shared copy" }),
  ).toHaveCount(0);
  await page.screenshot({ path: info.outputPath("unified-memory.png") });
  await nav(page, "Harley");
  await nav(page, "Memory");
  await expect(page.locator(".memory-record")).toHaveCount(0);
  await nav(page, "Janet");
  await nav(page, "Memory");
  await page
    .locator(".memory-record")
    .getByRole("button", { name: "Delete", exact: true })
    .click();
  await page
    .getByRole("dialog", { name: "Delete this memory?" })
    .getByRole("button", { name: "Delete memory", exact: true })
    .click();
  await expect(page.locator(".memory-record")).toHaveCount(0);
  await expect(
    page.getByText("No matching memories", { exact: true }),
  ).toBeVisible();
});

test("watch recommendations and disabled schedules remain available in Activity", async ({
  page,
}) => {
  let read = false;
  const changes: Body[] = [];
  await fixture(page, (_owner, body) => {
    if (body.operation === "notification.list")
      return {
        items: [
          {
            id: "notice-1",
            title: "Review deadline",
            summary: "A task needs your attention",
            status: "needs_manni",
            task_id: "task-1",
            created_at: Math.floor(Date.now() / 1000),
            read,
          },
        ],
        next_cursor: null,
      };
    if (body.operation === "notification.read") {
      read = true;
      return { read: true };
    }
    if (body.operation === "watch.configure") {
      changes.push(body);
      return { saved: true };
    }
  });
  await page.goto("/");
  await nav(page, "Activity");
  const panel = page.getByRole("region", { name: "Watch recommendations" });
  await expect(panel).toContainText("Review deadline");
  await panel.getByRole("button", { name: "Mark reviewed" }).click();
  await expect(
    panel.getByRole("button", { name: "Mark reviewed" }),
  ).toHaveCount(0);
  await panel.getByText("Watch schedule for janet", { exact: true }).click();
  await expect(panel.getByRole("checkbox")).not.toBeChecked();
  await panel.getByLabel("Check every (minutes)").fill("30");
  await panel.getByRole("button", { name: "Save schedule" }).click();
  await expect.poll(() => changes.length).toBe(1);
  expect(changes[0].settings.enabled).toBe(false);
  expect(changes[0].settings.interval_minutes).toBe(30);
});

test("Files navigation opens owned thread uploads", async ({ page }) => {
  await fixture(page, (owner, body) => {
    if (body.operation === "thread.page")
      return [
        {
          id: `${owner}-thread`,
          title: `${owner} project`,
          summary: "Project evidence",
        },
      ];
    if (body.operation === "file.list")
      return [
        {
          id: `${owner}-file`,
          filename: `${owner}-notes.txt`,
          size: 12,
          source: "upload",
          checksum: "fixture",
        },
      ];
  });
  await page.goto("/");
  await nav(page, "Files");
  const files = page.getByRole("region", { name: "Conversation files" });
  await files
    .getByRole("button", { name: "janet project Project folder" })
    .click();
  await expect(
    files.getByRole("button", { name: "Download janet-notes.txt" }),
  ).toBeVisible();
  await expect(
    files.getByText("Record a decision", { exact: true }),
  ).toHaveCount(0);
  await expect(files.getByText("harley-notes.txt")).toHaveCount(0);
  await expect(files.getByLabel("Upload a file (up to 4 MB)")).toBeVisible();
});

test("global Claude token uses dedicated endpoint and clears the field", async ({
  page,
}) => {
  await fixture(page);
  let saved = false;
  await page.route("**/api/settings/claude-credential", async (route) => {
    expect(route.request().postDataJSON()).toEqual({
      token: "sk-ant-api03-ui-fixture-token",
    });
    saved = true;
    await fulfillFixture(route, {
      contentType: "application/json",
      body: JSON.stringify({ configured: true, source: "secure_store" }),
    });
  });
  await page.goto("/");
  await nav(page, "App settings");
  await page.getByRole("button", { name: "Manage models" }).click();
  const field = page.getByLabel("Anthropic API token", { exact: true });
  await expect(field).toHaveAttribute("type", "password");
  await field.fill("sk-ant-api03-ui-fixture-token");
  await page
    .getByRole("button", { name: "Save API token", exact: true })
    .click();
  await expect(
    page.getByText("Claude API token saved securely.", { exact: false }),
  ).toBeVisible();
  await expect(field).toHaveValue("");
  expect(saved).toBe(true);
  expect(await page.evaluate(() => JSON.stringify(localStorage))).not.toContain(
    "sk-ant-api03-ui-fixture-token",
  );
});
