import { describe, it, expect } from "vitest";
import { cardSchema } from "./cards";
describe("closed rich card contract", () => {
  it("accepts a safe info card", () =>
    expect(
      cardSchema.safeParse({
        version: 1,
        type: "info",
        title: "Update",
        summary: "Ready",
      }).success,
    ).toBe(true));
  it("rejects executable payloads and unimplemented actions", () => {
    for (const payload of [
      {
        version: 1,
        type: "info",
        title: "Update",
        summary: "Ready",
        html: "<script>alert(1)</script>",
      },
      { version: 1, type: "totp", title: "Authorize", summary: "Send code" },
      { version: 2, type: "info", title: "Update", summary: "Ready" },
      {
        version: 1,
        type: "thread_reference",
        title: "Thread",
        summary: "Open",
      },
    ])
      expect(cardSchema.safeParse(payload).success).toBe(false);
  });
});
