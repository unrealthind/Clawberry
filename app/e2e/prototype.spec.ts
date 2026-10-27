import { test, expect, type Page } from "@playwright/test";

async function navigate(page: Page, name: string) {
  const menu = page.getByRole("button", { name: "Open navigation" });
  if (
    (await menu.isVisible()) &&
    !(await page
      .locator(".sidebar")
      .evaluate((el) => el.classList.contains("nav-open")))
  )
    await menu.click();
  if (["Skills", "Vault", "Identity & memory"].includes(name)) {
    const owner = await page
      .locator(".agent-choice.selected strong")
      .innerText();
    await page
      .locator(".sidebar")
      .getByRole("button", { name: `${owner} settings`, exact: true })
      .click();
    await page
      .getByRole("navigation", { name: `${owner} settings sections` })
      .getByRole("button", {
        name: name === "Vault" ? "Credentials" : name,
        exact: true,
      })
      .click();
    return;
  }
  await page
    .locator(".sidebar")
    .getByRole("button", { name, exact: true })
    .click();
}

test("sample conversations and draft decisions stay with their agent", async ({
  page,
}) => {
  const errors: string[] = [];
  const external: string[] = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("request", (request) => {
    if (
      !request.url().startsWith("http://127.0.0.1:4173/") ||
      request.url().includes("/api/")
    )
      external.push(request.url());
  });
  await page.goto("/prototype.html?mode=sample");
  const chat = page.locator(".chat-session:not([hidden])");
  await chat
    .getByRole("textbox", { name: "Message Janet" })
    .fill("A unique work-only note");
  await chat.getByRole("button", { name: "Send sample message" }).click();
  await expect(
    chat.getByText("A unique work-only note", { exact: true }),
  ).toBeVisible();
  await expect(
    chat.getByRole("button", { name: "Send sample message" }),
  ).toBeVisible();
  await navigate(page, "Tasks 3");
  await navigate(page, "Janet Work agent");
  await expect(
    chat.getByText("A unique work-only note", { exact: true }),
  ).toBeVisible();
  await navigate(page, "Harley Personal agent");
  await expect(
    page.getByRole("heading", { name: "Harley", exact: true }),
  ).toBeVisible();
  await expect(
    chat.getByText("A unique work-only note", { exact: true }),
  ).toHaveCount(0);
  await navigate(page, "Approvals 1");
  await page
    .getByRole("button", { name: "Review draft", exact: true })
    .first()
    .click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page
    .getByRole("button", { name: "Approve draft", exact: true })
    .click();
  await expect(page.getByRole("status")).toContainText(
    "No external action was taken",
  );
  await navigate(page, "Janet Work agent");
  await navigate(page, "Approvals 1");
  await expect(page.getByText("AWAITING YOUR REVIEW")).toBeVisible();
  expect(errors).toEqual([]);
  expect(external).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("tasks, Focus Threads, and sample attachments are interactive", async ({
  page,
}) => {
  await page.goto("/prototype.html?mode=sample");
  await navigate(page, "Tasks 3");
  const demo = page.locator(".task-grid .task-card").first();
  await expect(demo).toContainText("33%");
  await demo
    .getByRole("checkbox", { name: "Prepare demo environment" })
    .check();
  await expect(demo).toContainText("67%");
  await navigate(page, "Focus Threads");
  await page.getByRole("button", { name: "New thread", exact: true }).click();
  await page
    .getByRole("textbox", { name: "Thread name" })
    .fill("Prototype exploration");
  await page.getByRole("button", { name: "Create thread" }).click();
  const chat = page.locator(".chat-session:not([hidden])");
  await expect(chat).toContainText("Prototype exploration");
  await chat
    .getByRole("textbox", { name: "Message Janet" })
    .fill("A thread-specific note");
  await chat.getByRole("button", { name: "Send sample message" }).click();
  await expect(
    chat.getByRole("button", { name: "Send sample message" }),
  ).toBeVisible();
  await navigate(page, "Janet Work agent");
  await expect(
    chat.getByText("A thread-specific note", { exact: true }),
  ).toHaveCount(0);
  const chooser = page.waitForEvent("filechooser");
  await chat.getByRole("button", { name: "Attach sample file" }).click();
  await (
    await chooser
  ).setFiles({
    name: "prototype-note.txt",
    mimeType: "text/plain",
    buffer: Buffer.from("Sample only"),
  });
  await navigate(page, "Files");
  await expect(
    page.getByRole("button", { name: /prototype-note.txt/ }),
  ).toBeVisible();
});

test("sample streaming can be stopped and the composer remains usable", async ({
  page,
}) => {
  await page.goto("/prototype.html?mode=sample");
  const chat = page.locator(".chat-session:not([hidden])");
  await chat
    .getByRole("textbox", { name: "Message Janet" })
    .fill("Help me plan");
  await chat.getByRole("button", { name: "Send sample message" }).click();
  await chat.getByRole("button", { name: "Stop sample response" }).click();
  await expect(
    chat.getByRole("button", { name: "Send sample message" }),
  ).toBeVisible();
  await chat.getByRole("textbox", { name: "Message Janet" }).fill("Try again");
  await chat.getByRole("button", { name: "Send sample message" }).click();
  await expect(chat.getByText(/This reply is scripted/).last()).toBeVisible();
});

test("sidebar agent directory shows running workers and removes completed workers", async ({
  page,
}) => {
  await page.goto("/prototype.html?mode=sample");
  const menu = page.getByRole("button", { name: "Open navigation" });
  if (await menu.isVisible()) await menu.click();
  const sidebar = page.locator(".sidebar");
  await expect(
    sidebar.getByRole("region", { name: "Agents", exact: true }),
  ).toBeVisible();
  await expect(page.getByText("Finance", { exact: true })).toHaveCount(0);
  await expect(page.locator(".context-panel .registry-section")).toHaveCount(0);
  await expect(
    sidebar.getByRole("list", { name: "Janet running subagents" }),
  ).toBeVisible();
  await sidebar.getByRole("button", { name: "2 running subagents" }).click();
  await expect(
    sidebar.getByRole("list", { name: "Janet running subagents" }),
  ).toBeHidden();
  await sidebar.getByRole("button", { name: "2 running subagents" }).click();
  await sidebar.getByRole("button", { name: /Release researcher/ }).click();
  await expect(page.getByRole("dialog")).toContainText(
    "Janet’s temporary subagent",
  );
  await page.getByRole("button", { name: "Complete sample run" }).click();
  if (await menu.isVisible()) await menu.click();
  await expect(
    sidebar.getByRole("button", { name: /Release researcher/ }),
  ).toHaveCount(0);
  await expect(
    sidebar.getByRole("button", { name: /Demo preparation/ }),
  ).toBeVisible();
  await sidebar
    .getByRole("button", { name: "Harley Personal agent", exact: true })
    .click();
  await expect(
    page.getByRole("heading", { name: "Harley", exact: true }),
  ).toBeVisible();
  expect(
    await page
      .locator(".workspace-header .avatar img")
      .evaluate(
        (image: HTMLImageElement) => image.complete && image.naturalWidth > 0,
      ),
  ).toBe(true);
  if (await menu.isVisible()) await menu.click();
  await sidebar
    .getByRole("button", { name: "Add agent", exact: true })
    .first()
    .click();
  await page.getByRole("textbox", { name: "Agent name" }).fill("Research");
  await page.getByRole("button", { name: "Add planned agent" }).click();
  await expect(
    sidebar.locator(".planned-agent").filter({ hasText: "Research" }),
  ).toContainText("Planned agent");
  await sidebar
    .getByRole("button", { name: "Janet Work agent", exact: true })
    .click();
  await navigate(page, "Activity");
  await expect(
    page.getByRole("heading", {
      name: "Release researcher completed — sample result archived",
    }),
  ).toBeVisible();
});

test("light appearance preserves workspace state when switching themes", async ({
  page,
}) => {
  await page.goto("/prototype.html?mode=sample&theme=light");
  await expect(page.locator(".prototype")).toHaveAttribute(
    "data-theme",
    "light",
  );
  await navigate(page, "Tasks 3");
  const demo = page.locator(".task-grid .task-card").first();
  await demo
    .getByRole("checkbox", { name: "Prepare demo environment" })
    .check();
  await page.getByRole("button", { name: "Switch to dark theme" }).click();
  await expect(page.locator(".prototype")).toHaveAttribute(
    "data-theme",
    "dark",
  );
  await expect(demo).toContainText("67%");
  await page.getByRole("button", { name: "Switch to light theme" }).click();
  await expect(
    demo.getByRole("checkbox", { name: "Prepare demo environment" }),
  ).toBeChecked();
  await navigate(page, "Harley Personal agent");
  await expect(page.locator(".prototype")).toHaveAttribute(
    "data-theme",
    "light",
  );
  await page.getByRole("button", { name: "Switch to dark theme" }).click();
  await expect(
    page.getByRole("heading", { name: "Harley", exact: true }),
  ).toBeVisible();
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("skills can be created, edited as individual files, downloaded, and reopened", async ({
  page,
}) => {
  await page.goto("/prototype.html?mode=sample&theme=light");
  await navigate(page, "Skills");
  const library = page.locator(".skills-library:visible");
  await library
    .getByRole("button", { name: "Create skill", exact: true })
    .click();
  const dialog = page.getByRole("dialog", { name: "Create a skill" });
  await dialog
    .getByRole("textbox", { name: "Skill name", exact: true })
    .fill("Video review");
  await dialog
    .getByRole("textbox", { name: "Description" })
    .fill("Review selected videos and summarize evidence.");
  await dialog
    .getByRole("button", { name: "Create skill", exact: true })
    .click();
  await expect(library.getByLabel("Skill file source")).toContainText(
    "name: video-review",
  );
  await library.getByRole("button", { name: "Create skill file" }).click();
  await page
    .getByRole("textbox", { name: "File path" })
    .fill("scripts/extract_frames.py");
  await page.getByRole("button", { name: "Create file", exact: true }).click();
  await library
    .getByRole("textbox", { name: "Skill file content" })
    .fill('# Sample helper\nprint("prototype only")\n');
  await library.getByRole("button", { name: "Save file", exact: true }).click();
  await expect(library.getByLabel("Skill file source")).toContainText(
    'print("prototype only")',
  );
  const pending = page.waitForEvent("download");
  await library.getByRole("button", { name: "Download skill file" }).click();
  expect((await pending).suggestedFilename()).toBe("extract_frames.py");
  await page.reload();
  await navigate(page, "Skills");
  await library
    .getByRole("navigation", { name: "Janet skills" })
    .getByRole("button", { name: /Video review/ })
    .click();
  await library
    .getByRole("button", { name: "scripts/extract_frames.py", exact: true })
    .click();
  await expect(library.getByLabel("Skill file source")).toContainText(
    'print("prototype only")',
  );
  await navigate(page, "Harley Personal agent");
  await navigate(page, "Skills");
  await expect(
    library.getByRole("navigation", { name: "Harley skills" }),
  ).toContainText("Weekend planner");
  await expect(library.getByText("Video review", { exact: true })).toHaveCount(
    0,
  );
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("vault previews named credentials without accepting or persisting secret values", async ({
  page,
}) => {
  const apiRequests: string[] = [];
  page.on("request", (request) => {
    if (request.url().includes("/api/")) apiRequests.push(request.url());
  });
  await page.goto("/prototype.html?mode=sample&theme=light");
  await navigate(page, "Vault");
  const vault = page.locator(".vault-library:visible");
  await vault.getByRole("button", { name: "Preview injection" }).click();
  await expect(vault.locator(".vault-request-preview")).toContainText(
    "Authorization: Bearer [injected by broker]",
  );
  await vault.getByRole("button", { name: "Pause sample access" }).click();
  await expect(vault.locator(".vault-request-preview")).toContainText(
    "DENIED — sample access paused",
  );
  await vault
    .getByRole("button", { name: "Add credential", exact: true })
    .click();
  const dialog = page.getByRole("dialog", { name: "Add a sample credential" });
  await expect(
    dialog.getByLabel("Secret value", { exact: true }),
  ).toBeDisabled();
  await dialog
    .getByRole("textbox", { name: "Service name" })
    .fill("Project system");
  await dialog
    .getByRole("textbox", { name: "Credential reference" })
    .fill("project_system_read");
  await dialog
    .getByLabel("Credential type", { exact: true })
    .selectOption("API key header");
  await dialog
    .getByRole("textbox", { name: "Allowed capability" })
    .fill("projects.read");
  await dialog
    .getByRole("textbox", { name: "Allowed destination" })
    .fill("https://projects.example.test");
  await dialog.getByRole("button", { name: "Create sample reference" }).click();
  await expect(vault.getByRole("status")).toContainText(
    "No secret was collected or stored",
  );
  await expect(
    vault.getByRole("region", { name: "Credential details" }),
  ).toContainText("X-API-Key header");
  await navigate(page, "Harley Personal agent");
  await navigate(page, "Vault");
  await expect(vault).not.toContainText("project_system_read");
  await expect(vault).toContainText("personal_calendar_read");
  const storage = await page.evaluate(() => ({
    local: Object.keys(localStorage),
    session: Object.keys(sessionStorage),
  }));
  expect(storage).toEqual({ local: [], session: [] });
  expect(apiRequests).toEqual([]);
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("permanent soul and memory files stay separate and retain saved edits", async ({
  page,
}) => {
  await page.goto("/prototype.html?mode=sample&theme=light");
  await navigate(page, "Identity & memory");
  const files = page.getByRole("region", { name: "Janet permanent files" });
  await expect(files.locator(".core-file-card")).toHaveCount(2);
  await files.getByRole("button", { name: /soul.md/ }).click();
  let dialog = page.getByRole("dialog", { name: "soul.md", exact: true });
  await expect(dialog.getByLabel("Permanent file source")).toContainText(
    "# Janet",
  );
  await dialog.getByRole("button", { name: "Edit file", exact: true }).click();
  await dialog
    .getByRole("textbox", { name: "Permanent file content" })
    .fill("# Janet\nA saved work-only identity note.");
  await dialog.getByRole("button", { name: "Save file", exact: true }).click();
  await expect(dialog.getByRole("status")).toContainText(
    "Saved in this browser for Janet",
  );
  await dialog.getByRole("button", { name: "Close", exact: true }).click();
  await page.reload();
  await navigate(page, "Identity & memory");
  await files.getByRole("button", { name: /soul.md/ }).click();
  await expect(
    page.getByRole("dialog").getByLabel("Permanent file source"),
  ).toContainText("A saved work-only identity note");
  await page.getByRole("button", { name: "Close", exact: true }).click();
  await navigate(page, "Harley Personal agent");
  await navigate(page, "Identity & memory");
  await page
    .getByRole("region", { name: "Harley permanent files" })
    .getByRole("button", { name: /memory.md/ })
    .click();
  await expect(
    page.getByRole("dialog").getByLabel("Permanent file source"),
  ).toContainText("# Harley");
  await expect(
    page.getByRole("dialog").getByLabel("Permanent file source"),
  ).not.toContainText("work-only identity note");
});

test("shared task calendar supports events, deadlines, filters, and separate agent spaces", async ({
  page,
}) => {
  await page.goto("/prototype.html?mode=sample&theme=light");
  await navigate(page, "Tasks 3");
  await page.getByRole("button", { name: "Calendar", exact: true }).click();
  const calendar = page.locator(".task-calendar:visible");
  await expect(
    calendar.getByRole("heading", { name: "October 2026", exact: true }),
  ).toBeVisible();
  await expect(
    calendar.getByRole("complementary", { name: "Selected day agenda" }),
  ).toContainText("Weekly check-in");
  await calendar
    .getByRole("button", { name: "New event", exact: true })
    .click();
  let dialog = page.getByRole("dialog", { name: "New event", exact: true });
  await dialog
    .getByRole("textbox", { name: "Event title" })
    .fill("Shared planning session");
  await dialog.getByLabel("Date", { exact: true }).fill("2026-10-21");
  await dialog.getByLabel("Time · Edmonton", { exact: true }).fill("15:00");
  await dialog.getByLabel("Duration", { exact: true }).selectOption("45");
  await dialog.getByRole("button", { name: "Add event", exact: true }).click();
  const agenda = calendar.getByRole("complementary", {
    name: "Selected day agenda",
  });
  await expect(agenda).toContainText("Shared planning session");
  await expect(agenda).toContainText("Finalize v0.2 release plan");
  await agenda.getByRole("button", { name: /Shared planning session/ }).click();
  dialog = page.getByRole("dialog", { name: "Edit event", exact: true });
  await dialog
    .getByRole("textbox", { name: "Event title" })
    .fill("Release planning together");
  await dialog.getByRole("button", { name: "Save event", exact: true }).click();
  await expect(agenda).toContainText("Release planning together");
  await calendar
    .getByRole("combobox", { name: "Calendar creator filter" })
    .selectOption("User");
  await expect(agenda).not.toContainText("Finalize v0.2 release plan");
  await calendar
    .getByRole("combobox", { name: "Calendar creator filter" })
    .selectOption("all");
  await calendar
    .getByRole("button", { name: "Next month", exact: true })
    .click();
  await expect(
    calendar.getByRole("heading", { name: "November 2026", exact: true }),
  ).toBeVisible();
  await calendar.getByRole("button", { name: "Today", exact: true }).click();
  await expect(
    calendar.getByRole("heading", { name: "October 2026", exact: true }),
  ).toBeVisible();
  await navigate(page, "Harley Personal agent");
  await navigate(page, "Tasks 2");
  await page.getByRole("button", { name: "Calendar", exact: true }).click();
  await expect(calendar).toContainText("Plan the weekend");
  await expect(calendar).not.toContainText("Release planning together");
  await navigate(page, "Janet Work agent");
  await navigate(page, "Tasks 3");
  await page.getByRole("button", { name: "Calendar", exact: true }).click();
  await calendar
    .getByRole("button", { name: /Wednesday, October 21, 2026/ })
    .click();
  await expect(
    calendar.getByRole("complementary", { name: "Selected day agenda" }),
  ).toContainText("Release planning together");
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
});

test("header activity follows chat generation and running workers", async ({
  page,
}) => {
  await page.goto("/prototype.html?mode=sample&theme=light");
  const activity = page.getByLabel("Current agent activity", { exact: true });
  await expect(activity).toContainText("Gathering feedback for v0.2");
  await expect(
    page.locator(".workspace-header .agent-heading"),
  ).not.toContainText("Work agent");
  await expect(page.locator(".subagent-toggle .worker-live-dot")).toHaveCount(
    0,
  );
  const chat = page.locator(".chat-session:not([hidden])");
  await chat
    .getByRole("textbox", { name: "Message Janet" })
    .fill("Help me plan");
  await chat.getByRole("button", { name: "Send sample message" }).click();
  await expect(activity).toContainText("Replying to your message");
  await chat.getByRole("button", { name: "Stop sample response" }).click();
  await expect(activity).toContainText("Gathering feedback for v0.2");
  const menu = page.getByRole("button", { name: "Open navigation" });
  if (await menu.isVisible()) await menu.click();
  await page
    .locator(".sidebar")
    .getByRole("button", { name: /Release researcher:/ })
    .click();
  await page.getByRole("button", { name: "Complete sample run" }).click();
  await expect(activity).toContainText("Checking the sample demo checklist");
  await navigate(page, "Harley Personal agent");
  await expect(activity).toContainText("Comparing weekend trail options");
});
