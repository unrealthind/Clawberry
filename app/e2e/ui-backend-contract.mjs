import { chromium, expect } from "@playwright/test";
import assert from "node:assert/strict";
const base = process.env.UI_AUDIT_URL;
if (!base || !/^http:\/\/127\.0\.0\.1:\d+$/.test(base))
  throw new Error("Isolated loopback fixture URL required");
const token = "ui-audit-fixture-token"; // Fictional fixture credential, never a user key.
const browser = await chromium.launch({ headless: true });
const context = await browser.newContext();
const page = await context.newPage();
const errors = [];
page.on("pageerror", (error) => errors.push(error.message));
async function nav(name) {
  await page
    .locator(".sidebar")
    .getByRole("button", { name, exact: true })
    .click();
}
async function api(owner, operation, fields = {}) {
  return context.request.post(`${base}/api/agents/${owner}/operations`, {
    headers: { Authorization: `Bearer ${token}` },
    data: { operation, ...fields },
  });
}
async function login() {
  await page.getByLabel("Paired gateway token", { exact: true }).fill(token);
  await page.getByRole("button", { name: "Connect", exact: true }).click();
  await expect(page.getByLabel("Message Janet", { exact: true })).toBeVisible();
}
try {
  for (const credential of ["", "invalid-fixture-key"]) {
    const response = await context.request.get(`${base}/api/agents`, {
      headers: credential ? { Authorization: `Bearer ${credential}` } : {},
    });
    assert.equal(response.status(), 401);
    assert.equal(response.headers()["cache-control"], "no-store");
  }
  // LAN Vite must not serve runtime configuration, credentials or arbitrary host files.
  for (const path of [
    "/.env.local",
    "/@fs/opt/raaviclaw/.runtime/config.toml",
    "/@fs/etc/passwd",
  ]) {
    const response = await context.request.get(base + path);
    if (response.status() === 200) {
      // Vite may return its SPA entry for a nonexistent URL, never file bytes.
      assert(response.headers()["content-type"]?.includes("text/html"));
      assert(
        (await response.text()).includes('<div id="root">'),
        "Only the app fallback may be served",
      );
    }
  }
  const probeName = process.env.UI_AUDIT_PROBE_NAME;
  assert(probeName?.startsWith(".env.audit-"));
  for (const path of [`/${probeName}`, `/@fs${process.cwd()}/${probeName}`]) {
    const denied = await context.request.get(base + path);
    assert(
      !(await denied.text()).includes("PRIVATE_AUDIT_SENTINEL"),
      "Vite must not expose a real private-file probe",
    );
  }
  await page.goto(base);
  await expect(
    page.locator('meta[name="raaviclaw-auth-mode"]'),
  ).toHaveAttribute("content", "paired");
  await login();
  await nav("Focus Threads");
  await page.getByRole("button", { name: "New thread", exact: true }).click();
  await page
    .getByLabel("Thread title", { exact: true })
    .fill("Audit evidence project");
  const created = page.waitForResponse(
    (response) =>
      response.url().endsWith("/operations") &&
      response.request().postDataJSON()?.operation === "thread.create",
  );
  await page
    .getByRole("button", { name: "Create thread", exact: true })
    .click();
  const thread = await (await created).json();
  await expect(
    page.getByRole("heading", { name: "Audit evidence project", exact: true }),
  ).toBeVisible();
  await nav("Tasks");
  await page.getByRole("button", { name: "New task", exact: true }).click();
  await page.getByLabel("Task title").fill("Audit contract task");
  await page
    .getByLabel("Description", { exact: true })
    .fill("Retain evidence for this audit");
  await page.getByRole("button", { name: "Save task", exact: true }).click();
  await expect(
    page
      .locator("article.task-card")
      .getByRole("heading", { name: "Audit contract task", exact: true }),
  ).toBeVisible();
  let tasks = await (await api("janet", "task.list")).json();
  assert.equal(tasks.length, 1);
  const task = tasks[0];
  assert.equal(task.thread_id, thread.id);
  assert.equal(task.description, "Retain evidence for this audit");
  assert.equal(
    (await api("harley", "task.get", { task_id: task.id })).status(),
    400,
  );
  assert.equal(
    (await api("harley", "thread.read", { thread_id: thread.id })).status(),
    400,
  );
  await page.getByText("Task history and evidence", { exact: true }).click();
  await expect(page.locator(".task-evidence")).toContainText(
    "Recorded status: queued",
  );
  await page
    .getByRole("button", { name: "Open thread evidence", exact: true })
    .click();
  const workspace = page.getByRole("region", {
    name: "Focus Thread workspace",
    exact: true,
  });
  const upload = workspace.getByLabel("Upload a file", { exact: false });
  await expect(upload).toBeEnabled();
  await upload.setInputFiles({
    name: "audit.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("Audit source text"),
  });
  await expect(
    workspace.getByRole("button", { name: "Download audit.txt", exact: true }),
  ).toBeVisible();
  const files = await (
    await api("janet", "file.list", { thread_id: thread.id })
  ).json();
  assert.equal(files.length, 1);
  const deniedFile = await context.request.get(
    `${base}/api/agents/harley/files/${files[0].id}`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  assert.equal(deniedFile.status(), 404);
  const downloaded = await context.request.get(
    `${base}/api/agents/janet/files/${files[0].id}`,
    { headers: { Authorization: `Bearer ${token}` } },
  );
  assert.equal(await downloaded.text(), "Audit source text");
  assert.equal(downloaded.headers()["cache-control"], "no-store");
  assert.equal(downloaded.headers()["x-content-type-options"], "nosniff");
  await workspace
    .getByRole("button", { name: "Preview audit.txt", exact: true })
    .click();
  await expect(
    workspace.getByRole("region", { name: "Preview of audit.txt" }),
  ).toContainText("Audit source text");
  await workspace
    .getByRole("button", { name: "Include text in message", exact: true })
    .click();
  await expect(page.getByLabel("Message Janet")).toHaveValue(
    /Audit source text/,
  );
  await page
    .getByRole("combobox", { name: "Reasoning profile", exact: true })
    .selectOption("fast");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(page.locator(".live-agent")).toContainText(
    "Verified local fixture reply",
  );
  await expect(page.locator(".live-agent")).toContainText("fixture-fast");
  const history = await (
    await api("janet", "chat.page", { thread_id: thread.id })
  ).json();
  assert.equal(history.items.length, 2);
  assert(
    history.items.some((message) =>
      message.content.includes("Audit source text"),
    ),
  );
  await page
    .getByRole("tab", { name: "Files & evidence", exact: true })
    .click();
  await workspace
    .getByLabel("Record a decision")
    .fill("Retain this verified source");
  await workspace
    .getByRole("button", { name: "Add decision", exact: true })
    .click();
  await expect(workspace).toContainText("Retain this verified source");
  assert.equal(
    (
      await (
        await api("janet", "thread.decisions", { thread_id: thread.id })
      ).json()
    ).length,
    1,
  );

  // Drop the browser response after the real backend has committed the turn.
  // Recovery must reveal that commit without issuing another model request.
  await page.getByRole("tab", { name: "Conversation", exact: true }).click();
  let ambiguousRequests = 0;
  const loseConfirmation = async (route) => {
    ambiguousRequests++;
    const committed = await route.fetch();
    assert.equal(committed.status(), 200);
    await committed.body();
    await route.abort();
  };
  await page.route("**/api/chat/janet/stream", loseConfirmation);
  await page
    .getByLabel("Message Janet", { exact: true })
    .fill("Ambiguous committed fixture turn");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText("not confirmed");
  await expect(
    page.getByRole("button", { name: "Send message", exact: true }),
  ).toBeDisabled();
  await expect(page.getByLabel("Message Janet", { exact: true })).toHaveValue(
    "Ambiguous committed fixture turn",
  );
  await page
    .getByRole("button", { name: "Refresh history", exact: true })
    .click();
  await expect(
    page
      .locator(".live-user")
      .filter({ hasText: "Ambiguous committed fixture turn" }),
  ).toBeVisible();
  const recovered = await (
    await api("janet", "chat.page", { thread_id: thread.id })
  ).json();
  assert.equal(
    recovered.items.filter(
      (message) =>
        message.role === "user" &&
        message.content === "Ambiguous committed fixture turn",
    ).length,
    1,
  );
  assert.equal(ambiguousRequests, 1);
  await page.unroute("**/api/chat/janet/stream", loseConfirmation);

  // Real settings revision conflicts must fail without overwriting the saved record.
  const settingsUrl = `${base}/api/settings/global`;
  const headers = { Authorization: `Bearer ${token}` };
  const original = await (
    await context.request.get(settingsUrl, { headers })
  ).json();
  // Runtime fixture profiles are not valid user-editable Claude profile choices.
  // Use the real settings allowlist for this isolated settings-save check.
  for (const profile of Object.values(original.value.models.profiles)) {
    profile.provider = "anthropic";
    profile.model = "claude-sonnet-5-5";
  }
  const updated = structuredClone(original);
  updated.value.preferences.theme = "light";
  assert.equal(
    (
      await context.request.post(settingsUrl, { headers, data: updated })
    ).status(),
    200,
  );
  assert.equal(
    (
      await context.request.post(settingsUrl, { headers, data: original })
    ).status(),
    409,
  );
  assert.equal(
    (await (await context.request.get(settingsUrl, { headers })).json()).value
      .preferences.theme,
    "light",
  );

  const fact = {
    content: "Audit project uses selected evidence",
    category: "project",
    confidence: 0.9,
    source_type: "user",
    source_ref: "audit-fixture",
    sensitivity: "ordinary",
    expires_at: "2020-01-01T00:00:00Z",
  };
  assert.equal(
    (await api("janet", "memory.save", { id: null, fact })).status(),
    200,
  );
  await nav("Memory");
  const privateCard = page.locator(".memory-record");
  await expect(privateCard).toContainText(fact.content);
  await privateCard
    .getByRole("button", { name: "Verify", exact: true })
    .click();
  await expect(page.getByRole("alert")).toBeVisible();
  await privateCard.getByRole("button", { name: "Edit", exact: true }).click();
  const privateEditor = page.getByRole("dialog", {
    name: "Edit memory",
    exact: true,
  });
  await privateEditor.getByLabel("Expiry (RFC 3339, optional)").fill("");
  await privateEditor
    .getByRole("button", { name: "Save changes", exact: true })
    .click();
  await expect(privateEditor).toHaveCount(0);
  await privateCard
    .getByRole("button", { name: "Verify", exact: true })
    .click();
  await expect(privateCard).toContainText("Verified");
  await privateCard.getByRole("button", { name: "Share", exact: true }).click();
  await page
    .getByRole("button", { name: "Share exact fact", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Edit shared fact", exact: true }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Edit shared fact", exact: true })
    .click();
  const editor = page.getByRole("dialog", {
    name: "Edit shared fact",
    exact: true,
  });
  await editor
    .getByRole("textbox", { name: "Fact", exact: true })
    .fill("Edited shared audit fact");
  await editor
    .getByLabel("Expiry (RFC 3339, optional)")
    .fill("2035-01-01T00:00:00Z");
  await editor
    .getByRole("button", { name: "Save changes", exact: true })
    .click();
  await expect(editor).toHaveCount(0);
  await expect(privateCard).toContainText(fact.content);
  await expect(
    page.getByText("Edited shared audit fact", { exact: true }),
  ).toBeVisible();
  const shared = await (await api("janet", "shared.page")).json();
  assert.equal(shared.items[0].expires_at, "2035-01-01T00:00:00Z");
  assert.equal(shared.items[0].last_verified_at, null);
  const privateRows = await (await api("janet", "memory.page")).json();
  assert.equal(privateRows.items[0].expires_at, null);
  assert.equal(
    (
      await api("janet", "shared.stale", {
        id: shared.items[0].id,
        reviewed_hash: "stale-binding",
      })
    ).status(),
    400,
  );
  await nav("Harley");
  await nav("Memory");
  await expect(page.locator(".memory-record")).toHaveCount(0);
  await expect(
    page.getByText("Edited shared audit fact", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Edit shared fact", exact: true }),
  ).toHaveCount(0);
  await nav("Janet");
  await nav("Approvals");
  await page
    .getByText("Prepare a cross-agent request", { exact: true })
    .click();
  await page
    .getByRole("combobox", { name: "Related task", exact: true })
    .selectOption(task.id);
  await page
    .getByLabel("Exact question", { exact: true })
    .fill("Share only the reviewed meeting hour");
  await page
    .getByRole("button", { name: "Prepare question for review", exact: true })
    .click();
  const peer = page.getByRole("article", {
    name: "Cross-agent request",
    exact: true,
  });
  await expect(
    peer.getByRole("button", { name: "Approve question", exact: true }),
  ).toBeDisabled();
  await peer.getByRole("checkbox").check();
  await peer
    .getByRole("button", { name: "Approve question", exact: true })
    .click();
  await expect(peer).toContainText("Waiting for a scoped response");
  await nav("Harley");
  await nav("Approvals");
  await peer
    .getByRole("textbox", { name: "Scoped response", exact: true })
    .fill("Only 14:00 is available.");
  await peer
    .getByRole("button", { name: "Prepare response for review", exact: true })
    .click();
  await expect(peer).toContainText("Waiting for User’s exact response review");
  await nav("Janet");
  await nav("Approvals");
  await peer.getByRole("checkbox").check();
  await peer
    .getByRole("button", { name: "Approve response", exact: true })
    .click();
  await expect(peer).toContainText("temporary exchange is closed");
  const auth = await context.request.post(
    `${base}/api/agents/janet/write-authorizations`,
    {
      headers,
      data: {
        review_id: "missing-fixture-review",
        reviewed_hash: "fixture",
        code: "314159",
      },
    },
  );
  assert.equal(auth.status(), 400);
  assert.equal(auth.headers()["cache-control"], "no-store");
  const stored = await page.evaluate(() =>
    JSON.stringify({ ...localStorage, ...sessionStorage }),
  );
  assert(!stored.includes(token));
  assert(!stored.includes("314159"));
  assert(!page.url().includes(token));
  await page.getByRole("button", { name: "Disconnect", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Connect your workspace" }),
  ).toBeVisible();
  await page.reload();
  await expect(
    page.getByLabel("Paired gateway token", { exact: true }),
  ).toHaveValue("");
  await login();
  await nav("Focus Threads");
  await page
    .getByRole("button", { name: "Open Audit evidence project", exact: true })
    .click();
  await page
    .getByRole("tab", { name: "Files & evidence", exact: true })
    .click();
  await expect(workspace).toContainText("Retain this verified source");
  await expect(
    workspace.getByRole("button", { name: "Download audit.txt", exact: true }),
  ).toBeVisible();
  assert.deepEqual(errors, [], "No browser rendering errors");
  console.log(
    "PASS: real paired browser/Vite/Rust auth, CRUD, SSE/history, files, owner isolation, settings conflicts, exact shared/peer reviews, TOTP fail-closed, no credential storage, lost-confirmation recovery without replay, disconnect/reconnect persistence.",
  );
} finally {
  await browser.close();
}
