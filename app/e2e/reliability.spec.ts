import { fulfillFixture, fixtureModels } from "./api-fixture";
import { test, expect, type Page } from "@playwright/test";
async function fixture(
  page: Page,
  mode: "invalid-approvals" | "stalled-create",
) {
  let creates = 0;
  await page.route("**/api/**", async (route) => {
    const url = route.request().url();
    const body = route.request().postDataJSON() ?? {};
    let value: unknown = [];
    if (body.operation === "task.create" && mode === "stalled-create") {
      creates++;
      return;
    }
    if (body.operation === "write.reviews" && mode === "invalid-approvals")
      value = [{ id: "invalid", manifest: null }];
    else if (
      body.operation === "chat.page" ||
      body.operation === "notification.list" ||
      body.operation === "memory.page" ||
      body.operation === "shared.page"
    )
      value = { items: [], next_cursor: null };
    else if (body.operation === "activity.list")
      value = { events: [], workers: [] };
    else if (url.endsWith("/settings/global"))
      value = {
        revision: 0,
        value: {
          models: fixtureModels,
          preferences: {
            theme: "dark",
            notifications: true,
            week_start: "sunday",
            time_format: "12",
            show_completed: true,
          },
        },
      };
    else if (url.endsWith("/settings"))
      value = {
        revision: 0,
        value: {
          default_profile: "normal",
          soul: "",
          memory_notes: "",
          skills: [],
        },
      };
    else if (url.endsWith("/runtime/status"))
      value = {
        credential_configured: true,
        credential_source: "environment",
        live_verified: false,
      };
    await fulfillFixture(route, { json: value });
  });
  return () => creates;
}
async function nav(page: Page, name: string) {
  const button = page.getByRole("button", { name: "Open navigation" });
  if (
    (await button.isVisible()) &&
    (await button.getAttribute("aria-expanded")) !== "true"
  )
    await button.click();
  await page
    .locator(".sidebar")
    .getByRole("button", { name, exact: true })
    .click();
}
test("malformed approval responses show a recoverable error without exposing approval controls", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await fixture(page, "invalid-approvals");
  await page.goto("/");
  await nav(page, "Approvals");
  const panel = page.getByRole("region", { name: "External write approvals" });
  await expect(panel.getByRole("alert")).toContainText("Could not load");
  await expect(page.getByLabel("TOTP authorization code")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Refresh approvals", exact: true }),
  ).toBeEnabled();
  expect(errors).toEqual([]);
});
test("a stalled task mutation times out, preserves the draft and blocks blind replay", async ({
  page,
}) => {
  const creates = await fixture(page, "stalled-create");
  await page.goto("/");
  await nav(page, "Tasks");
  await page.getByRole("button", { name: "New task", exact: true }).click();
  await page.getByLabel("Task title").fill("Preserve my pending task");
  await page.clock.install();
  await page.getByRole("button", { name: "Save task", exact: true }).click();
  await expect.poll(creates).toBe(1);
  await page.clock.fastForward(46_000);
  const dialog = page.getByRole("dialog", { name: "New task", exact: true });
  await expect(dialog.getByRole("alert")).toContainText("not confirmed");
  await expect(page.getByLabel("Task title")).toHaveValue(
    "Preserve my pending task",
  );
  await expect(
    dialog.getByRole("button", { name: "Save task", exact: true }),
  ).toBeDisabled();
  expect(creates()).toBe(1);
});

test("malformed settings show a retry state without crashing the shell", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await fixture(page, "invalid-approvals");
  await page.route("**/api/agents/janet/settings", (route) =>
    route.fulfill({
      json: {
        revision: 1,
        value: { soul: "Partial response", skills: [null] },
      },
    }),
  );
  await page.goto("/");
  const navButton = page.getByRole("button", { name: "Open navigation" });
  if (await navButton.isVisible()) await navButton.click();
  await page
    .getByRole("button", { name: "Janet settings", exact: true })
    .click();
  await expect(page.getByRole("alert")).toContainText(
    "Settings could not be loaded",
  );
  await expect(
    page.getByRole("button", { name: "Retry settings", exact: true }),
  ).toBeEnabled();
  await expect(
    page.getByRole("button", { name: "Save settings", exact: true }),
  ).toHaveCount(0);
  expect(errors).toEqual([]);
});

test("malformed write session status exposes no fabricated active grant or budget", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await fixture(page, "invalid-approvals");
  await page.route("**/api/agents/*/operations", (route) => {
    const { operation } = route.request().postDataJSON();
    if (operation === "write.reviews")
      return route.fulfill({
        json: [
          {
            id: "session-1",
            task_id: "task-1",
            workflow: "email",
            manifest: {},
            manifest_hash: "binding",
            expires_at: Math.floor(Date.now() / 1000) + 120,
            state: "authorized",
          },
        ],
      });
    if (operation === "write.status")
      return route.fulfill({
        json: {
          expires_at: Math.floor(Date.now() / 1000) + 120,
          revoked: false,
          remaining: { "email.move": "unbounded" },
          actions: [],
        },
      });
    return route.fallback();
  });
  await page.goto("/");
  await nav(page, "Approvals");
  await expect(page.getByRole("alert")).toContainText(
    "Session status unavailable",
  );
  await expect(
    page.getByText("Remaining operation budgets", { exact: true }),
  ).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Refresh session status", exact: true }),
  ).toBeEnabled();
  expect(errors).toEqual([]);
});

test("failed chat refreshes task state without replay and hides legacy internal payloads", async ({
  page,
}) => {
  await fixture(page, "invalid-approvals");
  let submitted = 0;
  await page.route("**/api/agents/janet/operations", async (route) => {
    const operation = route.request().postDataJSON()?.operation;
    if (operation === "chat.page")
      return fulfillFixture(route, {
        json: {
          items: [
            {
              role: "assistant",
              content:
                '<internal_action>{"operation":"task.list"}</internal_action>',
            },
          ],
          next_cursor: null,
        },
      });
    if (operation === "task.page")
      return fulfillFixture(route, {
        json: [
          {
            id: "task-1",
            title: submitted ? "New runtime task" : "Previous task",
            description: "",
            status: "queued",
            priority: "normal",
            due_at: null,
            thread_id: null,
          },
        ],
      });
    await route.fallback();
  });
  await page.route("**/api/chat/janet/stream", async (route) => {
    submitted++;
    await route.fulfill({
      contentType: "text/event-stream",
      body: 'data: {"type":"error","code":"provider_auth","error":"private provider response"}\n\n',
    });
  });
  await page.goto("/");
  await expect(page.locator(".live-insights-panel")).toContainText(
    "Previous task",
  );
  await expect(page.locator(".live-messages")).toContainText(
    "Internal operation requested",
  );
  await expect(page.locator(".live-messages")).not.toContainText("task.list");
  await page
    .getByLabel("Message Janet", { exact: true })
    .fill("check my tasks");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(page.locator(".live-insights-panel")).toContainText(
    "New runtime task",
  );
  await expect(page.locator(".live-chat-error")).toContainText(
    "Claude rejected the credential",
  );
  await expect(page.locator(".live-chat-error")).not.toContainText(
    "private provider response",
  );
  await expect(
    page.getByRole("button", { name: "Send message", exact: true }),
  ).toBeDisabled();
  await expect(page.getByLabel("Message Janet", { exact: true })).toHaveValue(
    "check my tasks",
  );
  await page
    .getByRole("button", { name: "Refresh history", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Send message", exact: true }),
  ).toBeEnabled();
  expect(submitted).toBe(1);
});

test("global Tasks includes unlinked and other-thread work after selecting a Focus Thread", async ({
  page,
}) => {
  await fixture(page, "invalid-approvals");
  await page.route("**/api/agents/janet/operations", async (route) => {
    const operation = route.request().postDataJSON()?.operation;
    if (operation === "thread.page")
      return fulfillFixture(route, {
        json: [
          { id: "thread-test", title: "test", summary: "" },
          { id: "thread-other", title: "Other project", summary: "" },
        ],
      });
    if (operation === "task.page")
      return fulfillFixture(route, {
        json: [
          {
            id: "task-unlinked",
            title: "Enable HTTPS",
            description: "",
            status: "queued",
            priority: "normal",
            due_at: null,
            thread_id: null,
          },
          {
            id: "task-other",
            title: "Other project work",
            description: "",
            status: "queued",
            priority: "normal",
            due_at: null,
            thread_id: "thread-other",
          },
          {
            id: "task-test",
            title: "Selected thread work",
            description: "",
            status: "completed",
            priority: "normal",
            due_at: null,
            thread_id: "thread-test",
          },
        ],
      });
    await route.fallback();
  });
  await page.goto("/");
  await nav(page, "# test");
  await nav(page, "Tasks");
  await expect(page.locator(".task-grid")).toContainText("Enable HTTPS");
  await expect(page.locator(".task-grid")).toContainText("Other project work");
  await expect(page.locator(".task-grid")).not.toContainText(
    "Selected thread work",
  );
  await page.locator(".task-controls select").selectOption("all");
  await expect(page.locator(".task-grid")).toContainText(
    "Selected thread work",
  );
  await expect(page.locator(".task-grid .task-card")).toHaveCount(3);
  await page.getByRole("button", { name: "New task", exact: true }).click();
  await expect(page.getByRole("dialog").getByLabel("Focus Thread")).toHaveValue(
    "",
  );
});
