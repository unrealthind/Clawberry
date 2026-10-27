import { fulfillFixture } from "./api-fixture";
import { test, expect, type Page } from "@playwright/test";
const threadId = "11111111-1111-4111-8111-111111111111";
const task = {
  id: "task-1",
  title: "Review evidence",
  description: "Review",
  status: "investigating",
  priority: "normal",
  thread_id: threadId,
  due_at: null,
};
const thread = {
  id: threadId,
  title: "Evidence project",
  summary: "Initial summary",
};
type Body = { operation: string; [key: string]: any };
async function setup(
  page: Page,
  handler: (owner: string, body: Body) => unknown = () => undefined,
) {
  await page.route("**/api/**", async (route) => {
    const url = route.request().url();
    const body: Body = route.request().postDataJSON() ?? {};
    const owner = url.includes("/harley/") ? "harley" : "janet";
    let value: unknown;
    if (url.endsWith("/operations")) {
      value = await handler(owner, body);
      if (value === undefined)
        value =
          body.operation === "task.page"
            ? [task]
            : body.operation === "thread.page"
              ? [thread]
              : body.operation === "chat.page" ||
                  body.operation === "notification.list"
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
    } else if (url.endsWith("/settings/global"))
      value = {
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
                },
              ]),
            ),
            agent_daily_microusd: 5000000,
          },
        },
      };
    else if (url.endsWith("/settings"))
      value = {
        revision: 0,
        value: {
          soul: "",
          memory_notes: "",
          skills: [],
          default_profile: "normal",
        },
      };
    else
      value = {
        credential_configured: true,
        credential_source: "environment",
        live_verified: false,
      };
    await fulfillFixture(route, { json: value });
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
async function openEvidence(page: Page) {
  await nav(page, "Focus Threads");
  await page
    .getByRole("button", { name: "Open Evidence project", exact: true })
    .click();
  await page.getByRole("tab", { name: "Files & evidence" }).click();
}

test("chat cards validate payloads and only navigate to owned threads", async ({
  page,
}) => {
  const card = (id: string) =>
    "```raaviclaw-card\n" +
    JSON.stringify({
      version: 1,
      type: "thread_reference",
      title: "Project update",
      summary: "Review the evidence",
      thread_id: id,
    }) +
    "\n```";
  await setup(page, (_owner, body) =>
    body.operation === "chat.page"
      ? {
          items: [
            { role: "assistant", content: card(threadId) },
            {
              role: "assistant",
              content: card("22222222-2222-4222-8222-222222222222"),
            },
            {
              role: "assistant",
              content:
                '```raaviclaw-card\n{"version":99,"type":"info","title":"Future","summary":"Readable fallback"}\n```',
            },
            { role: "user", content: card(threadId) },
          ],
          next_cursor: null,
        }
      : undefined,
  );
  await page.goto("/");
  await expect(page.locator(".conversation-card")).toHaveCount(2);
  await expect(page.locator(".live-user .conversation-card")).toHaveCount(0);
  await expect(page.locator(".live-agent pre")).toContainText(
    "Readable fallback",
  );
  await page
    .getByRole("button", { name: "Open Focus Thread", exact: true })
    .first()
    .click();
  await expect(page.getByRole("alert")).toContainText(
    "unavailable in this agent",
  );
  await page
    .getByRole("button", { name: "Open Focus Thread", exact: true })
    .last()
    .click();
  await expect(
    page.getByRole("tab", { name: "Files & evidence" }),
  ).toBeVisible();
  await page.screenshot({
    path: test.info().outputPath("conversation-cards.png"),
    fullPage: true,
  });
});

test("thread previews include reviewed text in drafts and save summaries", async ({
  page,
}) => {
  let summary = "Initial summary";
  const writes: Body[] = [];
  await setup(page, (_owner, body) => {
    if (body.operation === "thread.page") return [{ ...thread, summary }];
    if (body.operation === "file.list")
      return [
        {
          id: "file-1",
          filename: "notes.txt",
          size: 17,
          checksum: "source-checksum",
          source: "user_file",
        },
        {
          id: "file-2",
          filename: "fake.png",
          size: 20,
          checksum: "bad-image",
          source: "user_file",
        },
        {
          id: "file-3",
          filename: "image.png",
          size: 68,
          checksum: "image-checksum",
          source: "user_file",
        },
      ];
    if (body.operation === "thread.summary") {
      summary = body.summary;
      writes.push(body);
      return { saved: true };
    }
  });
  await page.route("**/api/agents/janet/files/file-1", (route) =>
    fulfillFixture(route, {
      body: "Reviewed evidence",
      contentType: "application/octet-stream",
    }),
  );
  await page.route("**/api/agents/janet/files/file-2", (route) =>
    fulfillFixture(route, {
      body: "<script>bad()</script>",
      contentType: "image/png",
    }),
  );
  await page.route("**/api/agents/janet/files/file-3", (route) =>
    fulfillFixture(route, {
      body: Buffer.from(
        "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aEm0AAAAASUVORK5CYII=",
        "base64",
      ),
      contentType: "application/octet-stream",
    }),
  );
  await page.goto("/");
  await openEvidence(page);
  await page.getByRole("button", { name: "Preview fake.png" }).click();
  await expect(page.getByRole("alert")).toContainText("Could not preview");
  await expect(page.locator(".file-preview img")).toHaveCount(0);
  await page.getByRole("button", { name: "Preview image.png" }).click();
  await expect(page.getByRole("img", { name: "image.png" })).toBeVisible();
  await expect(page.locator(".file-preview")).toContainText(
    "Image interpretation is not connected",
  );
  await page.getByRole("button", { name: "Preview notes.txt" }).click();
  await expect(
    page.getByRole("region", { name: "Preview of notes.txt" }),
  ).toContainText("Reviewed evidence");
  await page.getByRole("button", { name: "Include text in message" }).click();
  await expect(
    page.getByRole("tab", { name: "Conversation", exact: true }),
  ).toHaveAttribute("aria-selected", "true");
  await expect(page.getByLabel("Message Janet")).toHaveValue(/source-checksum/);
  await expect(page.getByLabel("Message Janet")).toHaveValue(
    /Reviewed evidence/,
  );
  await page.getByRole("button", { name: "Attach files" }).click();
  await page.getByRole("button", { name: "Edit thread summary" }).click();
  await page
    .getByRole("textbox", { name: "Thread summary", exact: true })
    .fill("Reviewed summary");
  await page.getByRole("button", { name: "Save summary" }).click();
  await expect(
    page.getByRole("region", { name: "Focus Thread workspace" }),
  ).toContainText("Reviewed summary");
  expect(writes).toEqual([
    {
      operation: "thread.summary",
      thread_id: threadId,
      summary: "Reviewed summary",
    },
  ]);
  await page.getByRole("tab", { name: "Files & evidence" }).focus();
  await page.keyboard.press("ArrowLeft");
  await expect(
    page.getByRole("tab", { name: "Conversation", exact: true }),
  ).toBeFocused();
  await expect(page.getByLabel("Message Janet")).toHaveValue(
    /Reviewed evidence/,
  );
});

test("main chat attachment handoff preserves drafts and uncertain uploads require refresh", async ({
  page,
}) => {
  let created = false,
    uploads = 0;
  let files: unknown[] = [];
  await setup(page, (_owner, body) => {
    if (body.operation === "thread.page") return created ? [thread] : [];
    if (body.operation === "thread.create") {
      created = true;
      return thread;
    }
    if (body.operation === "file.list") return files;
  });
  await page.route("**/threads/*/uploads?*", async (route) => {
    uploads++;
    files = [
      {
        id: "file-1",
        filename: "notes.txt",
        checksum: "fixture",
        size: 5,
        source: "user_file",
      },
    ];
    await route.abort();
  });
  await page.goto("/");
  await page.getByLabel("Message Janet").fill("Please review my notes");
  await page.getByRole("button", { name: "Attach files" }).click();
  await expect(
    page.getByRole("form", { name: "Create Focus Thread" }),
  ).toContainText("your message draft will come with you");
  await page
    .getByLabel("Thread title", { exact: true })
    .fill("Evidence project");
  await page
    .getByRole("button", { name: "Create thread", exact: true })
    .click();
  const upload = page.getByLabel("Upload a file", { exact: false });
  await expect(upload).toBeEnabled();
  await upload.setInputFiles({
    name: "notes.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("Notes"),
  });
  await expect(page.getByRole("alert")).toContainText(
    "Upload was not confirmed",
  );
  await expect(upload).toBeDisabled();
  expect(uploads).toBe(1);
  await page
    .getByRole("button", { name: "Refresh evidence", exact: true })
    .click();
  await expect(
    page.getByRole("button", { name: "Download notes.txt" }),
  ).toBeVisible();
  await expect(upload).toBeEnabled();
  await page.getByRole("tab", { name: "Conversation", exact: true }).click();
  await expect(page.getByLabel("Message Janet")).toHaveValue(
    "Please review my notes",
  );
});

test("cross-agent questions and scoped responses require separate exact reviews", async ({
  page,
}) => {
  let exchange: any;
  const writes: Body[] = [];
  await setup(page, (owner, body) => {
    if (body.operation === "cross.review_queue")
      return exchange && (owner === "janet" || exchange.status !== "pending")
        ? [exchange]
        : [];
    if (body.operation === "cross.request") {
      writes.push(body);
      exchange = {
        id: "exchange-1",
        requester: "janet",
        recipient: "harley",
        task_id: "task-1",
        question: body.question,
        question_hash: "question-binding",
        response: null,
        response_hash: null,
        status: "pending",
        expires_at: Math.floor(Date.now() / 1000) + 900,
      };
      return exchange.id;
    }
    if (body.operation === "cross.review") {
      writes.push(body);
      expect(body.reviewed_hash).toBe(
        exchange.status === "pending" ? "question-binding" : "response-binding",
      );
      exchange.status =
        body.decision === "approve_question" ? "approved" : "closed";
      return {};
    }
    if (body.operation === "cross.respond") {
      writes.push(body);
      expect(owner).toBe("harley");
      expect(exchange.status).toBe("approved");
      exchange.response = body.response;
      exchange.response_hash = "response-binding";
      exchange.status = "response_pending";
      return {};
    }
  });
  await page.goto("/");
  await nav(page, "Approvals");
  await page
    .getByText("Prepare a cross-agent request", { exact: true })
    .click();
  await page
    .getByRole("combobox", { name: "Related task", exact: true })
    .selectOption("task-1");
  await page
    .getByLabel("Exact question", { exact: true })
    .fill("Which hour is free for the meeting?");
  await page
    .getByRole("button", { name: "Prepare question for review" })
    .click();
  const request = page.getByRole("article", { name: "Cross-agent request" });
  await expect(request).toContainText("Which hour is free");
  await expect(
    request.getByRole("button", { name: "Approve question" }),
  ).toBeDisabled();
  expect(writes.filter((b) => b.operation === "cross.review")).toHaveLength(0);
  await request.getByRole("checkbox").check();
  await request.getByRole("button", { name: "Approve question" }).click();
  await expect(request).toContainText("Waiting for a scoped response");
  await expect(
    request.getByRole("button", { name: "Revoke exchange" }),
  ).toBeEnabled();
  await nav(page, "Harley");
  await nav(page, "Approvals");
  await request.getByLabel("Scoped response").fill("Only 14:00 is available.");
  await request
    .getByRole("button", { name: "Prepare response for review" })
    .click();
  await expect(request).toContainText(
    "Waiting for User’s exact response review",
  );
  await nav(page, "Janet");
  await nav(page, "Approvals");
  await expect(request).toContainText("Only 14:00 is available.");
  await request.getByRole("checkbox").check();
  await request.getByRole("button", { name: "Approve response" }).click();
  await expect(request).toContainText("temporary exchange is closed");
  expect(
    writes
      .filter((b) => b.operation === "cross.review")
      .map((b) => b.reviewed_hash),
  ).toEqual(["question-binding", "response-binding"]);
  await page.screenshot({
    path: test.info().outputPath("closed-peer-exchange.png"),
    fullPage: true,
  });
});

test("watch recommendations expose provenance and reservation budgets", async ({
  page,
}) => {
  await setup(page, (_owner, body) =>
    body.operation === "notification.list"
      ? {
          items: [
            {
              id: "notice-1",
              title: "Deadline needs review",
              summary: "Review the permit evidence before deciding.",
              status: "needs_manni",
              task_id: "task-1",
              run_id: "run-1",
              worker_id: "worker-1",
              budget_microusd: 50000,
              created_at: 1792512000,
              read: false,
              evidence: [
                {
                  source_type: "task",
                  source_ref: "task-1",
                  content: '{"title":"Permit","status":"waiting"}',
                },
              ],
            },
          ],
          next_cursor: null,
        }
      : undefined,
  );
  await page.goto("/");
  await nav(page, "Activity");
  await page.getByText("Evidence and investigation", { exact: true }).click();
  const notices = page.getByRole("region", { name: "Watch recommendations" });
  await expect(notices).toContainText("Investigation run-1");
  await expect(notices).toContainText("Worker worker-1");
  await expect(notices).toContainText("Reserved budget: US$0.05");
  await expect(notices.locator("pre")).toContainText('"status": "waiting"');
  await page.screenshot({
    path: test.info().outputPath("recommendation-evidence.png"),
    fullPage: true,
  });
  await notices.getByRole("button", { name: "View related tasks" }).click();
  await expect(
    page.getByRole("heading", { name: "Tasks", exact: true }),
  ).toBeVisible();
});

test("task evidence loads recorded history and findings on demand", async ({
  page,
}) => {
  let reads = 0;
  await setup(page, (_owner, body) => {
    if (body.operation === "task.get") {
      reads++;
      return {
        task,
        events: [
          {
            event: "status_changed",
            detail: "Evidence review started",
            created_at: "2026-10-21",
          },
        ],
        attachments: [],
        workers: [
          {
            id: "worker-1",
            status: "completed",
            spec: { role: "Reviewer" },
            result: {
              summary: "Two verified findings",
              findings: ["Deadline recorded"],
              confidence: 0.9,
              sources: [{ source_type: "file", source_ref: "permit.md" }],
            },
          },
        ],
      };
    }
  });
  await page.goto("/");
  await nav(page, "Tasks");
  expect(reads).toBe(0);
  await page.getByText("Task history and evidence", { exact: true }).click();
  await expect(page.locator(".task-evidence")).toContainText(
    "Evidence review started",
  );
  await expect(page.locator(".task-evidence")).toContainText("Confidence 90%");
  await expect(page.locator(".task-evidence")).toContainText("file: permit.md");
  await page.getByRole("button", { name: "Open thread evidence" }).click();
  await expect(
    page.getByRole("tab", { name: "Files & evidence" }),
  ).toHaveAttribute("aria-selected", "true");
  await page.screenshot({
    path: test.info().outputPath("task-evidence.png"),
    fullPage: true,
  });
});

test("older task and thread pages stay reachable without losing selected thread ownership", async ({
  page,
}) => {
  const beforeValues: unknown[] = [];
  const olderThread = {
    ...thread,
    id: "33333333-3333-4333-8333-333333333333",
    title: "Older project",
  };
  await setup(page, (_owner, body) => {
    if (body.operation === "task.page") {
      beforeValues.push(body.page?.before ?? null);
      return body.page?.before
        ? {
            items: [{ ...task, id: "older-task", title: "Older task" }],
            next_cursor: null,
          }
        : { items: [task], next_cursor: 51 };
    }
    if (body.operation === "thread.page")
      return body.page?.before
        ? { items: [olderThread], next_cursor: null }
        : { items: [thread], next_cursor: 51 };
    if (body.operation === "thread.get") {
      expect(body.thread_id).toBe(olderThread.id);
      return olderThread;
    }
  });
  await page.goto("/");
  await nav(page, "Tasks");
  await expect(
    page.getByText("Showing 1 loaded tasks.", { exact: false }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Load more tasks", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Older task", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Load more tasks", exact: true }),
  ).toHaveCount(0);
  expect(beforeValues).toContain(51);
  await nav(page, "Focus Threads");
  await page
    .getByRole("button", { name: "Load more threads", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Open Older project", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Older project", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("tab", { name: "Files & evidence" }),
  ).toBeVisible();
  await expect(
    page.getByText("Could not load this local workspace", { exact: false }),
  ).toHaveCount(0);
});

test("malformed nested task evidence remains recoverable without crashing", async ({
  page,
}) => {
  const errors: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await setup(page, (_owner, body) =>
    body.operation === "task.get"
      ? {
          task,
          events: [],
          attachments: [],
          workers: [{ id: "bad", status: "completed", spec: null }],
        }
      : undefined,
  );
  await page.goto("/");
  await nav(page, "Tasks");
  await page.getByText("Task history and evidence", { exact: true }).click();
  await expect(page.locator(".task-evidence").getByRole("alert")).toContainText(
    "could not be loaded",
  );
  await expect(
    page.getByRole("button", { name: "Refresh task evidence", exact: true }),
  ).toBeEnabled();
  expect(errors).toEqual([]);
});
