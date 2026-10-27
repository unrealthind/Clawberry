import { fulfillFixture, fixtureModels } from "./api-fixture";
import { test, expect } from "@playwright/test";

test("connected context restores approval, task and actual usage summaries", async ({
  page,
}) => {
  await page.route("**/api/**", async (route) => {
    const op = route.request().postData()
      ? route.request().postDataJSON()?.operation
      : undefined;
    const value =
      op === "chat.page"
        ? {
            items: [
              {
                role: "assistant",
                content:
                  "## Task update\n\n- **Status:** Queued\n- **Owner:** Janet\n\n```sh\necho ready\n```\n\n| Name | State |\n| --- | --- |\n| Permit | Queued |",
              },
            ],
            next_cursor: null,
          }
        : op === "task.page"
          ? [
              {
                id: "task-1",
                title: "Review permit",
                description: "",
                status: "investigating",
                priority: "high",
                due_at: null,
                thread_id: null,
              },
            ]
          : op === "activity.list"
            ? { events: [], workers: [] }
            : op === "write.reviews"
              ? [
                  {
                    id: "review-1",
                    task_id: "task-1",
                    manifest: { mode: "single_action" },
                    manifest_hash: "reviewed-hash",
                    workflow: "Organize inbox",
                    state: "pending",
                    expires_at: 9999999999,
                  },
                ]
              : op === "usage.list"
                ? [
                    {
                      input_tokens: 1200,
                      output_tokens: 300,
                      cost_microusd: 250000,
                      usage_reported: true,
                    },
                  ]
                : op
                  ? []
                  : {
                      revision: 0,
                      value: {
                        models: fixtureModels,
                        preferences: { theme: "dark" },
                        default_profile: "normal",
                      },
                    };
    await fulfillFixture(route, { json: value });
  });
  await page.goto("/");
  const reply = page.locator(".live-agent .markdown-content");
  await expect(
    reply.getByRole("heading", { name: "Task update" }),
  ).toBeVisible();
  await expect(reply.locator("strong").first()).toHaveText("Status:");
  await expect(reply.locator("li")).toHaveCount(2);
  await expect(reply.locator("pre code")).toContainText("echo ready");
  await expect(reply.getByRole("table")).toContainText("Permit");
  const rail = page.locator(".live-insights-panel");
  await expect(rail).toContainText("AT A GLANCE");
  await expect(rail).toContainText("Organize inbox");
  await expect(rail).toContainText("Review permit");
  await expect(rail).toContainText("1,500");
  await expect(rail).toContainText("0.25");
  await expect(rail).toContainText("Daily budget");
  await expect(rail.locator(".live-insights-stats")).toHaveCount(0);
  await expect(rail).not.toContainText("owner_operation");
  await page.screenshot({
    path: test.info().outputPath("restored-context.png"),
    fullPage: true,
  });
  await expect
    .poll(() => rail.evaluate((el) => el.scrollWidth <= el.clientWidth))
    .toBe(true);
  await page
    .locator(".prototype")
    .evaluate((el) => el.setAttribute("data-theme", "light"));
  await page.screenshot({
    path: test.info().outputPath("restored-context-light.png"),
    fullPage: true,
  });
  await rail
    .getByRole("button", { name: "Review request", exact: true })
    .click();
  await expect(
    page.getByRole("region", { name: "Approvals workspace" }),
  ).toBeVisible();
});
