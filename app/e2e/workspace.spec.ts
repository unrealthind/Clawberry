import { fulfillFixture } from "./api-fixture";
import { test, expect, type Page } from "@playwright/test";
function initialGlobal() {
  return {
    revision: 0,
    value: {
      preferences: {
        theme: "dark",
        notifications: true,
        week_start: "sunday",
        time_format: "12",
        show_completed: true,
      },
      models: {
        profiles: Object.fromEntries(
          ["fast", "normal", "deep"].map((p) => [
            p,
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
  };
}
test.beforeEach(async ({ page }) => {
  let global = initialGlobal();
  const agents: Record<
    string,
    { revision: number; value: Record<string, unknown> }
  > = {};
  await page.route("**/api/settings/global", async (route) => {
    if (route.request().method() === "POST") {
      const input = route.request().postDataJSON();
      if (input.revision !== global.revision) {
        await fulfillFixture(route, {
          status: 409,
          contentType: "application/json",
          body: JSON.stringify({
            error:
              "Settings changed elsewhere. Reload before saving; your draft is preserved.",
          }),
        });
        return;
      }
      global = { revision: global.revision + 1, value: input.value };
    }
    await fulfillFixture(route, {
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(global),
    });
  });
  await page.route("**/api/runtime/status", (route) =>
    fulfillFixture(route, {
      status: 200,
      contentType: "application/json",
      body: JSON.stringify({
        credential_configured: true,
        credential_source: "environment",
        live_verified: false,
      }),
    }),
  );
  await page.route("**/api/agents/*/settings", async (route) => {
    const owner = route.request().url().includes("/janet/")
      ? "janet"
      : "harley";
    agents[owner] ??= {
      revision: 0,
      value: {
        soul: owner + " guidance",
        memory_notes: "",
        default_profile: "normal",
        skills: [],
      },
    };
    if (route.request().method() === "POST") {
      const input = route.request().postDataJSON();
      agents[owner] = {
        revision: agents[owner].revision + 1,
        value: input.value,
      };
    }
    await fulfillFixture(route, {
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(agents[owner]),
    });
  });
});
async function navigate(page: Page, name: string) {
  if (name === "Settings") {
    await settings(page);
    return;
  }
  const menu = page.getByRole("button", { name: "Open navigation" });
  if (await menu.isVisible()) await menu.click();
  await page
    .locator(".sidebar")
    .getByRole("button", { name, exact: true })
    .click();
}
async function settings(page: Page) {
  const menu = page.getByRole("button", { name: "Open navigation" });
  if (await menu.isVisible()) await menu.click();
  await page.locator(".profile").click();
}
test("task lifecycle, deadlines, thread evidence, and settings", async ({
  page,
}, testInfo) => {
  await page.goto("/prototype.html?mode=sample");
  await navigate(page, "Tasks 3");
  await page.getByRole("button", { name: "New task", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "New task" });
  await dialog.getByLabel("Task title").fill("Ship workspace");
  await dialog.getByLabel("Due date").fill("2026-10-25");
  await dialog.getByLabel("Priority").selectOption("urgent");
  await dialog.getByLabel("Focus Thread").selectOption("v0.2 release");
  await dialog.getByRole("button", { name: "Save task" }).click();
  let task = page
    .locator(".task-grid .task-card")
    .filter({ hasText: "Ship workspace" });
  await expect(task).toContainText("2026-10-25");
  await expect(page.locator(".task-grid .task-card").first()).toContainText(
    "Ship workspace",
  );
  await task.getByRole("button", { name: "Edit task" }).click();
  await page
    .getByRole("dialog", { name: "Edit task" })
    .getByLabel("Task title")
    .fill("Ship polished workspace");
  await page.getByRole("button", { name: "Save task" }).click();
  task = page
    .locator(".task-grid .task-card")
    .filter({ hasText: "Ship polished workspace" });
  await task
    .getByRole("button", { name: "Complete task", exact: true })
    .click();
  await expect(task).toContainText("100%");
  await page.getByRole("button", { name: "Calendar", exact: true }).click();
  await page
    .getByRole("button", { name: "Sunday, October 25, 2026, 1 items" })
    .click();
  await expect(
    page.getByRole("complementary", { name: "Selected day agenda" }),
  ).toContainText("Ship polished workspace");
  await navigate(page, "Focus Threads");
  await page
    .locator(".thread-card")
    .filter({ hasText: "v0.2 release" })
    .click();
  await page
    .getByRole("button", { name: "Thread workspace", exact: true })
    .click();
  const workspace = page.getByRole("region", { name: "Thread workspace" });
  await expect(workspace).toContainText("Ship polished workspace");
  await expect(workspace).toContainText("v0.2 Release Plan.md");
  await workspace
    .getByLabel("Record a decision")
    .fill("Keep the release focused");
  await workspace.getByRole("button", { name: "Add decision" }).click();
  await expect(workspace).toContainText("Keep the release focused");
  await workspace.getByRole("button", { name: /Release researcher/ }).click();
  await page.getByRole("button", { name: "Complete sample run" }).click();
  await expect(workspace).toContainText("structured handoff prepared");
  await page.screenshot({
    path: testInfo.outputPath("thread-workspace.png"),
  });
  await settings(page);
  await page
    .getByRole("navigation", { name: "Settings sections" })
    .getByRole("button", { name: "Calendar", exact: true })
    .click();
  await page.getByLabel("Week starts on").selectOption("monday");
  await page.getByLabel("Calendar time format").selectOption("24");
  await navigate(page, "Harley Personal agent");
  await navigate(page, "Focus Threads");
  await expect(page.getByText("Keep the release focused")).toHaveCount(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("local chat and tasks connect without a gateway token", async ({
  page,
}) => {
  let tasks: Record<string, unknown>[] = [];
  let answered = false;
  const operations: string[] = [];
  await page.route("**/api/agents/*/operations", async (route) => {
    expect(route.request().headers().authorization).toBeUndefined();
    const body = route.request().postDataJSON();
    operations.push(body.operation);
    let value: unknown = {};
    switch (body.operation) {
      case "task.page":
        value = route.request().url().includes("/janet/") ? tasks : [];
        break;
      case "thread.page":
        value = [];
        break;
      case "chat.page":
        value = {
          items: answered
            ? [{ role: "assistant", content: "Real gateway response" }]
            : [],
          next_cursor: null,
        };
        break;
      case "activity.list":
        value = { events: [], workers: [] };
        break;
      case "task.create":
        value = {
          id: "task-1",
          ...body,
          status: "queued",
          priority: "normal",
          due_at: null,
        };
        tasks.push(value as Record<string, unknown>);
        break;
      case "task.edit":
        tasks = tasks.map((t) => ({ ...t, ...body }));
        value = tasks[0];
        break;
      case "task.status":
        tasks = tasks.map((t) => ({ ...t, status: body.status }));
        break;
    }
    await fulfillFixture(route, {
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(value),
    });
  });
  await page.route("**/api/chat/janet/stream", async (route) => {
    expect(route.request().postDataJSON().message).toBe("Hello gateway");
    expect(route.request().postDataJSON().profile).toBe("deep");
    answered = true;
    await fulfillFixture(route, {
      status: 200,
      contentType: "text/event-stream",
      body: 'data: {"type":"complete","response":"Real gateway response"}\n\n',
    });
  });
  await page.goto("/prototype.html?mode=sample");
  await settings(page);
  await page
    .getByRole("navigation", { name: "Settings sections" })
    .getByRole("button", { name: "Connection & security", exact: true })
    .click();
  await page.getByRole("button", { name: "Connect gateway" }).click();
  await expect(page.getByLabel("Paired gateway token")).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "What’s on your mind?" }),
  ).toBeVisible();
  await page.getByLabel("Reasoning profile").selectOption("deep");
  await page.getByLabel("Message Janet").fill("Hello gateway");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(
    page.getByText("Real gateway response", { exact: true }),
  ).toBeVisible();
  await navigate(page, "Tasks");
  await page.getByRole("button", { name: "New task", exact: true }).click();
  await page.getByLabel("Task title").fill("Connected task");
  await page.getByLabel("Due date").fill("2026-10-25");
  await page.getByRole("button", { name: "Save task" }).click();
  await expect(page.locator("article.task-card")).toContainText(
    "Connected task",
  );
  await page
    .getByRole("button", { name: "Complete task", exact: true })
    .click();
  await expect(
    page.getByText("No active tasks here.", { exact: false }),
  ).toBeVisible();
  expect(operations).toEqual(
    expect.arrayContaining(["task.create", "task.edit", "task.status"]),
  );
  expect(
    await page.evaluate(() =>
      JSON.stringify({ ...localStorage, ...sessionStorage }),
    ),
  ).not.toContain("test-pairing");
  await navigate(page, "Harley");
  await expect(page.getByText("Connected task", { exact: true })).toHaveCount(
    0,
  );
  await page.locator(".live-disconnect").click();
  await expect(
    page.getByRole("heading", { name: "Connect your workspace", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("connection failures recover and uncertain task creation cannot be blindly retried", async ({
  page,
}) => {
  let failLoad = true;
  let creates = 0;
  await page.route("**/api/agents/*/operations", async (route) => {
    const body = route.request().postDataJSON();
    if (failLoad) {
      await fulfillFixture(route, {
        status: 503,
        contentType: "application/json",
        body: '{"error":"Unavailable"}',
      });
      return;
    }
    if (body.operation === "task.create") {
      creates++;
      await fulfillFixture(route, {
        status: 500,
        contentType: "application/json",
        body: '{"error":"Unknown outcome"}',
      });
      return;
    }
    const value =
      body.operation === "chat.page"
        ? { items: [], next_cursor: null }
        : body.operation === "activity.list"
          ? { events: [], workers: [] }
          : [];
    await fulfillFixture(route, {
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(value),
    });
  });
  await page.goto("/prototype.html?mode=sample");
  await settings(page);
  await page
    .getByRole("navigation", { name: "Settings sections" })
    .getByRole("button", { name: "Connection & security", exact: true })
    .click();
  await page.getByRole("button", { name: "Connect gateway" }).click();
  await expect(page.getByLabel("Paired gateway token")).toHaveCount(0);
  await expect(
    page
      .getByRole("alert")
      .filter({ hasText: "Could not load this local workspace" }),
  ).toBeVisible();
  failLoad = false;
  await page.getByRole("button", { name: "Refresh history and data" }).click();
  await expect(
    page.getByRole("heading", { name: "What’s on your mind?" }),
  ).toBeVisible();
  await navigate(page, "Tasks");
  await page.getByRole("button", { name: "New task", exact: true }).click();
  await page.getByLabel("Task title").fill("Uncertain task");
  await page.getByRole("button", { name: "Save task" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Refresh tasks before trying again",
  );
  await expect(page.getByRole("button", { name: "Save task" })).toBeDisabled();
  expect(creates).toBe(1);
  await page.keyboard.press("Escape");
  await expect(page.getByRole("dialog")).not.toBeVisible();
});

test("Main Settings contains shared models and preferences without agent setup", async ({
  page,
}, testInfo) => {
  const requests: string[] = [];
  page.on("request", (request) => {
    if (request.url().includes("/api/")) requests.push(request.url());
  });
  await page.goto("/prototype.html?mode=sample");
  await navigate(page, "Settings");
  await expect(
    page.getByRole("heading", { name: "Settings", exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("dialog")).not.toBeVisible();
  const nav = page.getByRole("navigation", { name: "Settings sections" });
  await nav
    .getByRole("button", { name: "Claude & models", exact: true })
    .click();
  await expect(page.getByLabel("API key", { exact: true })).toBeDisabled();
  await page
    .getByLabel("Normal model", { exact: true })
    .selectOption("claude-opus-5-5");
  await page
    .getByLabel("Normal output token limit", { exact: true })
    .fill("8192");
  await page.getByLabel("Daily budget per agent", { exact: true }).fill("8.50");
  await expect(
    page.getByRole("button", { name: "Save model draft" }),
  ).toBeEnabled();
  await nav.getByRole("button", { name: "General", exact: true }).click();
  await nav
    .getByRole("button", { name: "Claude & models", exact: true })
    .click();
  await expect(page.getByLabel("Normal model", { exact: true })).toHaveValue(
    "claude-opus-5-5",
  );
  await page.getByRole("button", { name: "Discard changes" }).click();
  await expect(page.getByLabel("Normal model", { exact: true })).toHaveValue(
    "claude-sonnet-5-5",
  );
  await page
    .getByLabel("Normal model", { exact: true })
    .selectOption("claude-opus-5-5");
  await page.getByRole("button", { name: "Save model draft" }).click();
  await expect(page.getByRole("status")).toContainText(
    "Runtime models have not changed",
  );
  await page.screenshot({
    path: testInfo.outputPath("settings.png"),
  });
  await page.reload();
  await navigate(page, "Settings");
  await nav
    .getByRole("button", { name: "Claude & models", exact: true })
    .click();
  await expect(page.getByLabel("Normal model", { exact: true })).toHaveValue(
    "claude-opus-5-5",
  );
  await page
    .getByLabel("Normal output token limit", { exact: true })
    .fill("32001");
  await expect(
    page.getByRole("button", { name: "Save model draft" }),
  ).toBeDisabled();
  await page.getByRole("button", { name: "Discard changes" }).click();
  await expect(
    nav.getByRole("button", { name: "Agents", exact: true }),
  ).toHaveCount(0);
  await nav.getByRole("button", { name: "Calendar", exact: true }).click();
  await page.getByLabel("Week starts on").selectOption("monday");
  expect(requests).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("Settings preserves an unreadable stored model draft", async ({
  page,
}) => {
  await page.addInitScript(() =>
    localStorage.setItem(
      "raaviclaw.prototype.model-draft.v1",
      '{"version":99}',
    ),
  );
  await page.goto("/prototype.html?mode=sample");
  await navigate(page, "Settings");
  await page
    .getByRole("navigation", { name: "Settings sections" })
    .getByRole("button", { name: "Claude & models", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText("preserved");
  await page
    .getByLabel("Normal model", { exact: true })
    .selectOption("claude-opus-5-5");
  await expect(
    page.getByRole("button", { name: "Save model draft" }),
  ).toBeDisabled();
  expect(
    await page.evaluate(() =>
      localStorage.getItem("raaviclaw.prototype.model-draft.v1"),
    ),
  ).toBe('{"version":99}');
});

test("agent setup is consolidated outside main Settings", async ({ page }) => {
  await page.goto("/prototype.html?mode=sample");
  const menu = page.getByRole("button", { name: "Open navigation" });
  if (await menu.isVisible()) await menu.click();
  const sidebar = page.locator(".sidebar");
  await expect(
    sidebar
      .getByRole("navigation", { name: "Workspace" })
      .getByRole("button", { name: "Skills", exact: true }),
  ).toHaveCount(0);
  await expect(
    sidebar
      .getByRole("navigation", { name: "Workspace" })
      .getByRole("button", { name: "Vault", exact: true }),
  ).toHaveCount(0);
  await sidebar
    .getByRole("button", { name: "Janet settings", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Janet settings", exact: true }),
  ).toBeVisible();
  const agentNav = page.getByRole("navigation", {
    name: "Janet settings sections",
  });
  await expect(
    page.getByRole("region", { name: "Janet permanent files" }),
  ).toBeVisible();
  await agentNav.getByRole("button", { name: "Skills", exact: true }).click();
  await expect(page.locator(".skills-library:visible")).toContainText("Janet");
  await agentNav
    .getByRole("button", { name: "Credentials", exact: true })
    .click();
  await expect(
    page.getByRole("navigation", { name: "Janet credential references" }),
  ).toBeVisible();
  await agentNav
    .getByRole("button", { name: "Access & boundaries", exact: true })
    .click();
  await expect(page.getByText("Janet’s domain", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Open chat", exact: true }).click();
  await navigate(page, "Harley settings");
  await expect(
    page.getByRole("heading", { name: "Harley settings", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("region", { name: "Harley permanent files" }),
  ).toBeVisible();
  await expect(
    page.getByRole("region", { name: "Janet permanent files" }),
  ).not.toBeVisible();
  await navigate(page, "Settings");
  const mainNav = page.getByRole("navigation", { name: "Settings sections" });
  await expect(
    mainNav.getByRole("button", { name: "Agents", exact: true }),
  ).toHaveCount(0);
  await mainNav
    .getByRole("button", { name: "Claude & models", exact: true })
    .click();
  await expect(page.getByLabel("Normal model", { exact: true })).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("live Focus Threads retain server IDs, context and paginated history", async ({
  page,
}, testInfo) => {
  const threads: { id: string; title: string; summary: string }[] = [];
  let contextUnavailable = false;
  const operations: Record<string, unknown>[] = [];
  await page.route("**/api/agents/*/operations", async (route) => {
    const body = route.request().postDataJSON();
    operations.push(body);
    if (
      contextUnavailable &&
      ["file.list", "activity.list"].includes(body.operation)
    ) {
      await fulfillFixture(route, {
        status: 503,
        contentType: "application/json",
        body: '{"error":"Unavailable"}',
      });
      return;
    }
    const owner = route.request().url().includes("/janet/")
      ? "janet"
      : "harley";
    let result: unknown = [];
    if (body.operation === "thread.page")
      result = owner === "janet" ? threads : [];
    if (body.operation === "thread.create") {
      result = {
        id: "server-thread-1",
        title: body.title,
        summary: "A durable project summary",
      };
      threads.push(result as (typeof threads)[number]);
    }
    if (body.operation === "chat.page")
      result = {
        items: body.page?.before
          ? [{ role: "user", content: "Earlier real message" }]
          : [
              {
                role: "assistant",
                content: body.thread_id ? "Thread history" : "Main history",
              },
            ],
        next_cursor: body.thread_id && !body.page?.before ? 42 : null,
      };
    if (body.operation === "task.page")
      result =
        owner === "janet" && threads.length
          ? [
              {
                id: "real-task",
                title: "Review project evidence",
                description: "Read the source",
                thread_id: "server-thread-1",
                status: "queued",
                priority: "high",
                due_at: null,
              },
            ]
          : [];
    if (body.operation === "file.list") {
      expect(body.thread_id).toBe("server-thread-1");
      result = [
        {
          id: "real-file",
          filename: "evidence.txt",
          size: 18,
          checksum: "fixture",
          source: "user",
        },
      ];
    }
    if (body.operation === "agent.list")
      result = [
        {
          id: "worker-1",
          task_id: "real-task",
          spec: { role: "Evidence reviewer", objective: "Review evidence" },
          status: "completed",
          error: null,
          archive_path: null,
          result: null,
        },
      ];
    if (body.operation === "activity.list")
      result = {
        events: [],
        workers: [
          {
            id: "worker-1",
            task_id: "real-task",
            role: "Evidence reviewer",
            status: "completed",
          },
        ],
      };
    await fulfillFixture(route, {
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(result),
    });
  });
  await page.goto("/prototype.html?mode=sample");
  await settings(page);
  await page
    .getByRole("button", { name: "Connection & security", exact: true })
    .click();
  await page.getByRole("button", { name: "Connect gateway" }).click();
  await expect(page.getByLabel("Paired gateway token")).toHaveCount(0);
  await expect(page.getByText("Main history", { exact: true })).toBeVisible();
  await navigate(page, "Focus Threads");
  await page.getByRole("button", { name: "New thread", exact: true }).click();
  await page.getByLabel("Thread title", { exact: true }).fill("Project live");
  await page
    .getByRole("button", { name: "Create thread", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Project live", exact: true }),
  ).toBeVisible();
  await page.getByRole("tab", { name: "Files & evidence" }).click();
  const context = page.getByRole("region", { name: "Focus Thread workspace" });
  await expect(context).toContainText("Review project evidence");
  await expect(context).toContainText("evidence.txt");
  await expect(context).toContainText("Evidence reviewer · completed");
  await page.getByRole("tab", { name: "Conversation", exact: true }).click();
  await page
    .getByRole("button", { name: "Load earlier messages", exact: true })
    .click();
  await expect(
    page.getByText("Earlier real message", { exact: true }),
  ).toBeVisible();
  expect(operations).toContainEqual({
    operation: "chat.page",
    thread_id: "server-thread-1",
    page: { before: 42 },
  });
  await page.getByRole("button", { name: "Refresh history and data" }).click();
  await page.getByRole("tab", { name: "Files & evidence" }).click();
  await expect(context).toContainText("A durable project summary");
  await page.getByRole("tab", { name: "Conversation", exact: true }).click();
  contextUnavailable = true;
  await page.getByRole("button", { name: "Refresh history and data" }).click();
  await expect(
    page
      .getByRole("status")
      .filter({ hasText: "Chat and tasks remain available" }),
  ).toBeVisible();
  await expect(page.getByLabel("Message Janet")).toBeVisible();
  await expect(page.getByText("Thread history", { exact: true })).toBeVisible();
  contextUnavailable = false;
  await page.getByRole("button", { name: "Refresh history and data" }).click();
  await page.getByRole("tab", { name: "Files & evidence" }).click();
  await expect(context).toContainText("evidence.txt");
  await page.screenshot({
    path: testInfo.outputPath("live-workspace.png"),
    fullPage: true,
  });
  await navigate(page, "Harley");
  await expect(page.getByText("Main history", { exact: true })).toBeVisible();
  await expect(page.getByText("evidence.txt", { exact: false })).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "Project live", exact: true }),
  ).toHaveCount(0);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("normal workspace uses real data and persists global and separate agent settings", async ({
  page,
}, testInfo) => {
  await page.route("**/api/agents/*/operations", async (route) => {
    const { operation } = route.request().postDataJSON();
    const value =
      operation === "chat.page"
        ? { items: [], next_cursor: null }
        : operation === "activity.list"
          ? { events: [], workers: [] }
          : [];
    await fulfillFixture(route, {
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(value),
    });
  });
  await page.goto("/prototype.html");
  await expect(
    page.getByRole("heading", { name: "What’s on your mind?" }),
  ).toBeVisible();
  await expect(
    page.getByText("Prepare customer demo", { exact: true }),
  ).toHaveCount(0);
  await page.keyboard.press("Control+,");
  await page.getByRole("button", { name: "Light theme", exact: true }).click();
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  await expect(
    page.getByText("Saved to runtime.", { exact: false }),
  ).toBeVisible();
  await expect(page.locator(".connected-workspace")).toHaveAttribute(
    "data-theme",
    "light",
  );
  await page.reload();
  await expect(page.locator(".connected-workspace")).toHaveAttribute(
    "data-theme",
    "light",
  );
  const open = page.getByRole("button", { name: "Open navigation" });
  if (await open.isVisible()) await open.click();
  await page
    .getByRole("button", { name: "Janet settings", exact: true })
    .click();
  await page
    .getByLabel("soul.md", { exact: true })
    .fill("Janet persisted work guidance");
  await page
    .getByLabel("memory.md", { exact: true })
    .fill("Private work notes");
  await page.getByLabel("Default reasoning profile").selectOption("fast");
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  await expect(
    page.getByText("Saved to runtime.", { exact: false }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Skills", exact: true }).click();
  await page.getByRole("button", { name: "Create skill", exact: true }).click();
  await page.locator("summary").filter({ hasText: "New skill" }).click();
  await page.getByLabel("Skill name", { exact: true }).fill("Evidence brief");
  await page
    .getByLabel("SKILL.md", { exact: true })
    .fill("Summarize evidence with sources.");
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  await expect(
    page.getByText("Saved to runtime.", { exact: false }),
  ).toBeVisible();
  if (await open.isVisible()) await open.click();
  await page
    .getByRole("button", { name: "Harley settings", exact: true })
    .click();
  await expect(page.getByLabel("soul.md", { exact: true })).toHaveValue(
    "harley guidance",
  );
  if (await open.isVisible()) await open.click();
  await page
    .getByRole("button", { name: "Janet settings", exact: true })
    .click();
  await expect(page.getByLabel("soul.md", { exact: true })).toHaveValue(
    "Janet persisted work guidance",
  );
  await expect(page.getByLabel("Default reasoning profile")).toHaveValue(
    "fast",
  );
  await page.screenshot({
    path: testInfo.outputPath("persistent-agent-settings.png"),
    fullPage: true,
  });
  await settings(page);
  await page.getByRole("button", { name: "Dark theme", exact: true }).click();
  await page.evaluate(async () => {
    const current = await fetch("/api/settings/global").then((r) => r.json());
    current.value.preferences.time_format = "24";
    await fetch("/api/settings/global", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(current),
    });
  });
  await page
    .getByRole("button", { name: "Save settings", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText(
    "Settings changed elsewhere",
  );
  await expect(
    page.getByRole("button", { name: "Dark theme", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("live thread transfers files, records decisions and opens archived results", async ({
  page,
}, testInfo) => {
  const decisions: { id: string; content: string; created_at: string }[] = [];
  const files: {
    id: string;
    filename: string;
    size: number;
    checksum: string;
    source: string;
  }[] = [];
  const attempted: string[] = [];
  let failSave = true;
  const worker = {
    id: "archived-worker",
    task_id: "linked-task",
    spec: { role: "Evidence reviewer", objective: "Review supplied evidence" },
    status: "completed",
    error: null,
    archive_path: "private-archive",
    result: {
      summary: "Supported recommendation",
      findings: ["Reviewed evidence"],
      confidence: 0.9,
      sources: [{ source_type: "file", source_ref: "source-1" }],
      artifacts: [{ filename: "review.md" }],
      errors: [],
    },
  };
  await page.route("**/api/agents/*/operations", async (route) => {
    const body = route.request().postDataJSON();
    const janet = route.request().url().includes("/janet/");
    let value: unknown = [];
    if (body.operation === "chat.page")
      value = { items: [], next_cursor: null };
    if (body.operation === "activity.list")
      value = { events: [], workers: janet ? [worker] : [] };
    if (body.operation === "thread.page")
      value = janet
        ? [
            {
              id: "live-thread",
              title: "Evidence project",
              summary: "Owned summary",
            },
          ]
        : [];
    if (body.operation === "task.page")
      value = janet
        ? [
            {
              id: "linked-task",
              title: "Review project",
              description: "Read",
              thread_id: "live-thread",
              status: "needs_manni",
              priority: "normal",
              due_at: null,
            },
          ]
        : [];
    if (body.operation === "file.list") value = files;
    if (body.operation === "agent.list")
      value = [
        worker,
        {
          ...worker,
          id: "unrelated-worker",
          task_id: "other-task",
          spec: { role: "Unrelated worker", objective: "Other" },
        },
      ];
    if (body.operation === "thread.decisions") value = decisions;
    if (body.operation === "thread.decision.record") {
      attempted.push(body.id);
      if (!decisions.some((d) => d.id === body.id))
        decisions.push({
          id: body.id,
          content: body.content,
          created_at: "2026-10-19 12:00:00",
        });
      if (failSave) {
        failSave = false;
        await route.abort();
        return;
      }
      value = decisions;
    }
    if (body.operation === "archive.read")
      value = {
        filename: body.filename,
        data_base64: Buffer.from("Archived evidence").toString("base64"),
      };
    await fulfillFixture(route, {
      contentType: "application/json",
      body: JSON.stringify(value),
    });
  });
  await page.route(
    "**/api/agents/janet/threads/live-thread/uploads?*",
    async (route) => {
      expect(route.request().headers().authorization).toBeUndefined();
      expect(route.request().postDataBuffer()?.toString()).toBe(
        "Evidence bytes",
      );
      const file = {
        id: "uploaded-file",
        filename: "source.md",
        size: 14,
        checksum: "fixture",
        source: "user_file",
      };
      files.push(file);
      await fulfillFixture(route, {
        status: 201,
        contentType: "application/json",
        body: JSON.stringify(file),
      });
    },
  );
  await page.route("**/api/agents/janet/files/uploaded-file", (route) =>
    fulfillFixture(route, {
      contentType: "application/octet-stream",
      body: "# Evidence\n\n**Reviewed** document.",
    }),
  );
  await page.goto("/prototype.html");
  await navigate(page, "Focus Threads");
  await page.getByRole("button", { name: "Open Evidence project" }).click();
  await page.getByRole("tab", { name: "Files & evidence" }).click();
  const workspace = page.getByRole("region", {
    name: "Focus Thread workspace",
  });
  const upload = workspace.getByLabel("Upload a file", { exact: false });
  await expect(upload).toBeEnabled();
  await upload.setInputFiles({
    name: "source.md",
    mimeType: "text/plain",
    buffer: Buffer.from("Evidence bytes"),
  });
  const downloadWait = page.waitForEvent("download");
  await workspace.getByRole("button", { name: "Download source.md" }).click();
  expect((await downloadWait).suggestedFilename()).toBe("source.md");
  await workspace
    .getByRole("button", { name: "Preview source.md", exact: true })
    .click();
  const preview = workspace.getByRole("region", {
    name: "Preview of source.md",
  });
  await expect(
    preview.getByRole("heading", { name: "Evidence" }),
  ).toBeVisible();
  await expect(preview.locator("strong")).toHaveText("Reviewed");
  await workspace
    .getByRole("button", { name: "Close preview", exact: true })
    .click();
  await expect(preview).toHaveCount(0);
  await workspace.getByLabel("Record a decision").fill("🌍".repeat(1100));
  await workspace.getByRole("button", { name: "Add decision" }).click();
  await expect(workspace.getByRole("alert")).toContainText("4,000 bytes");
  expect(attempted).toHaveLength(0);
  await workspace
    .getByLabel("Record a decision")
    .fill("Keep the scope focused");
  await workspace.getByRole("button", { name: "Add decision" }).click();
  await expect(workspace.getByRole("alert")).toContainText("not confirmed");
  await workspace.getByRole("button", { name: "Retry decision save" }).click();
  await expect(workspace).toContainText("Keep the scope focused");
  expect(attempted).toHaveLength(2);
  expect(attempted[0]).toBe(attempted[1]);
  expect(decisions).toHaveLength(1);
  await workspace
    .getByText("Evidence reviewer · completed", { exact: true })
    .click();
  await expect(workspace).toContainText("Supported recommendation");
  await expect(workspace).toContainText("Confidence: 90%");
  await expect(workspace).toContainText("file: source-1");
  await expect(workspace).not.toContainText("Unrelated worker");
  const archiveWait = page.waitForEvent("download");
  await workspace
    .getByRole("button", { name: "Download review.md", exact: true })
    .click();
  expect((await archiveWait).suggestedFilename()).toBe("review.md");
  await page.reload();
  await navigate(page, "Focus Threads");
  await page.getByRole("button", { name: "Open Evidence project" }).click();
  await page.getByRole("tab", { name: "Files & evidence" }).click();
  await expect(workspace).toContainText("Keep the scope focused");
  await expect(
    workspace.getByRole("button", { name: "Download source.md" }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await workspace.screenshot({
    path: testInfo.outputPath("live-thread-evidence.png"),
  });
  await navigate(page, "Harley");
  await expect(
    page.getByText("Keep the scope focused", { exact: true }),
  ).toHaveCount(0);
});

test("live layout exposes one App settings entry and separate agent controls", async ({
  page,
}, testInfo) => {
  await page.route("**/api/agents/*/operations", async (route) => {
    const { operation } = route.request().postDataJSON();
    await fulfillFixture(route, {
      contentType: "application/json",
      body: JSON.stringify(
        operation === "chat.page"
          ? {
              items: [
                { role: "user", content: "Hello Janet" },
                {
                  role: "assistant",
                  content: "What would you like to work on today?",
                },
              ],
              next_cursor: null,
            }
          : operation === "activity.list"
            ? { events: [], workers: [] }
            : [],
      ),
    });
  });
  await page.goto("/prototype.html");
  await expect(
    page.getByText("What would you like to work on today?", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Switch to light theme" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Switch to light theme" }).click();
  await expect(page.locator(".connected-workspace")).toHaveAttribute(
    "data-theme",
    "light",
  );
  await page.reload();
  await expect(page.locator(".connected-workspace")).toHaveAttribute(
    "data-theme",
    "light",
  );
  const menu = page.getByRole("button", { name: "Open navigation" });
  if (await menu.isVisible()) await menu.click();
  const appSettings = page.getByRole("button", {
    name: "App settings",
    exact: true,
  });
  await expect(appSettings).toHaveCount(1);
  await expect(appSettings).toBeInViewport({ ratio: 1 });
  await expect(page.locator(".prototype-brand")).toBeInViewport({ ratio: 1 });
  await expect(appSettings).toContainText("App settings");
  await expect(appSettings.locator("svg")).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Main conversation", exact: true }),
  ).toBeInViewport({ ratio: 1 });
  await page
    .locator(".sidebar")
    .screenshot({ path: testInfo.outputPath("live-navigation-light.png") });
  if (await menu.isVisible()) {
    await appSettings.focus();
    await page.keyboard.press("Tab");
    await expect(
      page.getByRole("button", { name: "Janet", exact: true }),
    ).toBeFocused();
    await page.keyboard.press("Shift+Tab");
    await expect(appSettings).toBeFocused();
    await page.keyboard.press("Escape");
    await expect(menu).toBeFocused();
    await expect(
      page.getByRole("button", { name: "App settings", exact: true }),
    ).toHaveCount(0);
    await menu.click();
  }
  await page.locator("#live-content").evaluate((el) => {
    el.scrollTop = el.scrollHeight;
  });
  await appSettings.focus();
  await page.keyboard.press("Enter");
  await expect(
    page.getByRole("heading", { name: "App settings", exact: true }),
  ).toBeInViewport();
  await expect(
    page.getByRole("heading", { name: "General", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Light theme", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await expect(page.getByLabel("soul.md", { exact: true })).toHaveCount(0);
  await page
    .getByRole("button", { name: "Manage models", exact: false })
    .click();
  await expect(
    page.getByRole("heading", { name: "Claude & models", exact: true }),
  ).toBeVisible();
  const sections = page.getByRole("navigation", { name: "Settings sections" });
  await expect(
    sections.getByRole("button", { name: "Claude & models", exact: true }),
  ).toHaveAttribute("aria-current", "page");
  await sections
    .getByRole("button", { name: "Connection & security", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Workspace connection", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("textbox", { name: /API key|token/i }),
  ).toHaveCount(0);
  await sections.getByRole("button", { name: "General", exact: true }).click();
  await page.screenshot({
    path: testInfo.outputPath("live-app-settings-light.png"),
  });
  if (await menu.isVisible()) await menu.click();
  await expect(appSettings).toHaveAttribute("aria-current", "page");
  await page
    .getByRole("button", { name: "Janet settings", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Janet settings", exact: true }),
  ).toBeVisible();
  await expect(page.getByLabel("soul.md", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "Credentials", exact: true }).click();
  await expect(
    page.getByRole("heading", { name: "Janet’s credentials", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByText("Not available yet", { exact: true }),
  ).toBeVisible();
  await expect(page.getByRole("textbox")).toHaveCount(0);
  await expect(
    page.getByRole("navigation", { name: "Settings sections", exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Manage models", exact: false }),
  ).toHaveCount(0);
  if (await menu.isVisible()) await menu.click();
  await expect(
    page.getByRole("button", { name: "Janet settings", exact: true }),
  ).toHaveAttribute("aria-pressed", "true");
  await page
    .locator(".sidebar")
    .getByRole("button", { name: "Chat", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Janet", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({ path: testInfo.outputPath("live-chat-light.png") });
  const saved = await page.evaluate(async () =>
    fetch("/api/settings/global").then((r) => r.json()),
  );
  expect(saved.value.models.profiles.normal.model).toBe("claude-sonnet-5-5");
  expect(saved.value.preferences.notifications).toBe(true);
  await page.route("**/api/settings/global", async (route) => {
    if (route.request().method() === "POST")
      await fulfillFixture(route, {
        status: 409,
        contentType: "application/json",
        body: JSON.stringify({ error: "Settings changed elsewhere" }),
      });
    else await route.fallback();
  });
  await page.getByRole("button", { name: "Switch to dark theme" }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Appearance could not be saved",
  );
  await expect(page.locator(".connected-workspace")).toHaveAttribute(
    "data-theme",
    "light",
  );
});
