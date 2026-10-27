import type { Route } from "@playwright/test";

export const fixtureModels = {
  profiles: Object.fromEntries(
    ["fast", "normal", "deep"].map((name) => [
      name,
      {
        provider: "anthropic",
        model: "claude-sonnet-5-5",
        max_output_tokens: 4096,
        input_microusd_per_million: 1,
        output_microusd_per_million: 1,
      },
    ]),
  ),
  agent_daily_microusd: 1000000,
  deep_daily_microusd: 1000000,
  task_microusd: 100000,
};
// Wrap small array fixtures in the paged wire envelope. Explicit malformed
// objects and explicit cursor fixtures pass through unchanged.
export async function fulfillFixture(
  route: Route,
  options: Parameters<Route["fulfill"]>[0],
) {
  const operation = route
    .request()
    .headers()
    ["content-type"]?.includes("application/json")
    ? route.request().postDataJSON()?.operation
    : undefined;
  if (["task.page", "thread.page"].includes(operation)) {
    const value =
      options?.json ??
      (typeof options?.body === "string"
        ? JSON.parse(options.body)
        : undefined);
    if (Array.isArray(value)) {
      const { body: _body, ...rest } = options ?? {};
      options = { ...rest, json: { items: value, next_cursor: null } };
    }
  }
  return route.fulfill(options);
}
