import { fulfillFixture, fixtureModels } from "./api-fixture";
import { test, expect, type Page } from "@playwright/test";

async function mockApi(page: Page) {
  await page.route("**/api/settings/global", (route) =>
    fulfillFixture(route, {
      contentType: "application/json",
      body: JSON.stringify({
        revision: 0,
        value: {
          models: fixtureModels,
          preferences: {
            theme: "light",
            notifications: true,
            week_start: "sunday",
            time_format: "12",
            show_completed: true,
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
  let chatCompleted = false;
  await page.route("**/api/agents/*/operations", async (route) => {
    const request = route.request();
    expect(request.headers().authorization).toBeUndefined();
    const body = request.postDataJSON() as { operation?: string };
    let value: unknown;
    switch (body.operation) {
      case "chat.page":
        value = {
          items: chatCompleted
            ? [
                {
                  sequence: 1,
                  role: "assistant",
                  content: "Browser acceptance reply",
                },
              ]
            : [],
          next_cursor: null,
        };
        break;
      case "thread.page":
      case "task.page":
        value = [];
        break;
      case "activity.list":
        value = { events: [], workers: [] };
        break;
      default:
        value = [];
        break;
    }
    await fulfillFixture(route, {
      status: 200,
      contentType: "application/json",
      body: JSON.stringify(value),
    });
  });

  await page.route("**/api/chat/*/stream", async (route) => {
    const request = route.request();
    expect(request.headers().authorization).toBeUndefined();
    chatCompleted = true;
    await fulfillFixture(route, {
      status: 200,
      headers: {
        "content-type": "text/event-stream",
        "cache-control": "no-store",
      },
      body:
        'data: {"type":"delta","text":"Browser acceptance reply"}\n\n' +
        'data: {"type":"complete","response":"Browser acceptance reply"}\n\n',
    });
  });
}

async function nav(page: Page, name: string) {
  const menu = page.getByRole("button", { name: "Open navigation" });
  if (
    (await menu.isVisible()) &&
    !(await menu
      .getAttribute("aria-expanded")
      .then((value) => value === "true"))
  )
    await menu.click();
  await page
    .locator(".sidebar")
    .getByRole("button", { name, exact: true })
    .click();
}
async function connect(page: Page) {
  await page.goto("/");
  await expect(page.getByLabel("Paired gateway token")).toHaveCount(0);
  await expect(page.getByRole("heading", { name: "Janet" })).toBeVisible();
}

test("local access, agent isolation navigation, SSE chat, and disconnect", async ({
  page,
}) => {
  await mockApi(page);
  await connect(page);

  expect(await page.evaluate(() => localStorage.length)).toBe(0);
  expect(await page.evaluate(() => sessionStorage.length)).toBe(0);

  await nav(page, "Harley");
  await expect(page.getByRole("heading", { name: "Harley" })).toBeVisible();
  await nav(page, "Janet");

  await page.getByLabel("Message Janet").fill("browser smoke");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(page.getByText("Browser acceptance reply")).toBeVisible();

  await page.getByRole("button", { name: "Disconnect" }).click();
  await expect(page.getByLabel("Paired gateway token")).toHaveCount(0);
  await expect(
    page.getByRole("button", { name: "Connect", exact: true }),
  ).toBeVisible();
  expect(await page.evaluate(() => localStorage.length)).toBe(0);
  expect(await page.evaluate(() => sessionStorage.length)).toBe(0);
});

test("local reload reconnects without storing a token", async ({ page }) => {
  await mockApi(page);
  await connect(page);
  await page.reload();
  await expect(page.getByLabel("Paired gateway token")).toHaveCount(0);
  await expect(
    page.getByRole("heading", { name: "Janet", exact: true }),
  ).toBeVisible();
});

test("phone viewport keeps both assistants and workspace navigation usable", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "phone-chromium", "phone project only");
  await mockApi(page);
  await connect(page);
  const menu = page.getByRole("button", { name: "Open navigation" });
  if (await menu.isVisible()) await menu.click();
  await expect(
    page.getByRole("button", { name: "Janet", exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Harley", exact: true }),
  ).toBeVisible();
  await nav(page, "Tasks");
  await expect(page.getByRole("heading", { name: "Tasks" })).toBeVisible();
});

test("chat composer grows, keeps touch newlines and submits the chosen profile", async ({
  page,
}, info) => {
  await mockApi(page);
  await connect(page);
  await expect(
    page.getByRole("heading", { name: "What’s on your mind?" }),
  ).toBeVisible();
  await page
    .getByRole("button", { name: "Help me plan my priorities" })
    .click();
  const field = page.getByRole("textbox", { name: "Message Janet" });
  await expect(field).toHaveValue("Help me plan my priorities");
  const initialHeight = await field.evaluate(
    (el) => el.getBoundingClientRect().height,
  );
  await field.fill("first line\nsecond line\nthird line\nfourth line");
  expect(
    await field.evaluate((el) => el.getBoundingClientRect().height),
  ).toBeGreaterThan(initialHeight);
  await page
    .getByRole("combobox", { name: "Reasoning profile" })
    .selectOption("deep");
  const request = page.waitForRequest("**/api/chat/janet/stream");
  if (info.project.name === "phone-chromium") {
    await field.press("End");
    await field.press("Enter");
    await expect(field).toHaveValue(
      "first line\nsecond line\nthird line\nfourth line\n",
    );
    await page
      .getByRole("button", { name: "Send message", exact: true })
      .click();
  } else {
    await field.press("Enter");
  }
  expect((await request).postDataJSON()).toMatchObject({ profile: "deep" });
  await expect(page.getByText("Browser acceptance reply")).toBeVisible();
  await expect(field).toHaveValue("");
  if (info.project.name === "desktop-chromium")
    await expect(field).toBeFocused();
  await page.screenshot({ path: info.outputPath("chat-conversation.png") });
});

test("unconfirmed chat retains conversation and draft until history is checked", async ({
  page,
}) => {
  await mockApi(page);
  await page.route("**/api/chat/*/stream", (route) =>
    fulfillFixture(route, {
      status: 200,
      contentType: "text/event-stream",
      body: 'data: {"type":"delta","text":"Partial answer retained"}\n\n',
    }),
  );
  await connect(page);
  const field = page.getByRole("textbox", { name: "Message Janet" });
  await field.fill("Keep my message");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(page.getByRole("alert")).toContainText(
    "Response was not confirmed",
  );
  await expect(field).toHaveValue("Keep my message");
  await expect(
    page.getByText("Partial answer retained", { exact: true }),
  ).toBeVisible();
  await expect(
    page.getByRole("button", { name: "Send message", exact: true }),
  ).toBeDisabled();
  await page
    .getByRole("button", { name: "Refresh history", exact: true })
    .click();
  await expect(page.getByRole("alert")).toHaveCount(0);
  await expect(field).toHaveValue("Keep my message");
  await expect(
    page.getByRole("button", { name: "Send message", exact: true }),
  ).toBeEnabled();
});

test("long conversations keep the composer reachable without pulling away from history", async ({
  page,
}, info) => {
  await mockApi(page);
  await page.route("**/api/agents/*/operations", (route) => {
    const op = route.request().postDataJSON().operation;
    return fulfillFixture(route, {
      contentType: "application/json",
      body: JSON.stringify(
        op === "chat.page"
          ? {
              items: Array.from({ length: 35 }, (_, i) => ({
                role: i % 2 ? "user" : "assistant",
                content:
                  `Message ${35 - i}: ` +
                  "Readable conversation text. ".repeat(10),
              })),
              next_cursor: null,
            }
          : op === "activity.list"
            ? { events: [], workers: [] }
            : [],
      ),
    });
  });
  await connect(page);
  const transcript = page.getByRole("region", { name: "Live conversation" });
  await expect(
    page.getByRole("button", { name: "Send message", exact: true }),
  ).toBeInViewport();
  await transcript.evaluate((el) => {
    el.scrollTop = 0;
    el.dispatchEvent(new Event("scroll"));
  });
  await expect(
    page.getByRole("button", { name: "Jump to latest" }),
  ).toBeVisible();
  await page.getByRole("button", { name: "Jump to latest" }).click();
  await expect(
    page.getByRole("button", { name: "Jump to latest" }),
  ).toHaveCount(0);
  expect(
    await transcript.evaluate(
      (el) => el.scrollHeight - el.scrollTop - el.clientHeight,
    ),
  ).toBeLessThan(3);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({
    path: info.outputPath("chat-long-conversation.png"),
  });
});

test("stopping a reply preserves the draft and requires history recovery", async ({
  page,
}) => {
  await mockApi(page);
  let release!: () => void;
  const gate = new Promise<void>((resolve) => {
    release = resolve;
  });
  await page.route("**/api/chat/*/stream", async (route) => {
    await gate;
    await route
      .fulfill({ contentType: "text/event-stream", body: "" })
      .catch(() => {});
  });
  await connect(page);
  const field = page.getByRole("textbox", { name: "Message Janet" });
  await field.fill("A request I may need to recover");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(page.getByText("Thinking through your message…")).toBeVisible();
  await page
    .getByRole("button", { name: "Stop response", exact: true })
    .click();
  release();
  await expect(page.getByRole("alert")).toContainText(
    "submitted work may still finish",
  );
  await expect(field).toHaveValue("A request I may need to recover");
  await expect(
    page.getByRole("button", { name: "Send message", exact: true }),
  ).toBeDisabled();
});

test("copy reply copies only the answer and reports clipboard failures", async ({
  page,
}) => {
  await mockApi(page);
  await connect(page);
  await page
    .getByRole("textbox", { name: "Message Janet" })
    .fill("Copy this reply");
  await page.getByRole("button", { name: "Send message", exact: true }).click();
  await expect(page.getByText("Browser acceptance reply")).toBeVisible();
  await page.evaluate(() => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: async (text: string) => {
          document.documentElement.dataset.copied = text;
        },
      },
    });
  });
  await page.getByRole("button", { name: "Copy reply 1", exact: true }).click();
  await expect(page.locator("html")).toHaveAttribute(
    "data-copied",
    "Browser acceptance reply",
  );
  await expect(page.getByRole("status")).toContainText("Reply copied.");
  await page.evaluate(() => {
    Object.defineProperty(navigator, "clipboard", {
      configurable: true,
      value: {
        writeText: async () => {
          throw new Error("denied");
        },
      },
    });
  });
  await page.getByRole("button", { name: "Copy reply 1", exact: true }).click();
  await expect(page.getByRole("status")).toContainText("Could not copy");
});
