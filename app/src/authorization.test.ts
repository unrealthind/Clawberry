import { afterEach, describe, expect, it, vi } from "vitest";
import { authorizeWrite } from "./authorization";

afterEach(() => vi.unstubAllGlobals());
describe("dedicated write authorization transport", () => {
  it("sends codes only to the authenticated non-caching endpoint", async () => {
    const fetch = vi.fn().mockResolvedValue({
      ok: true,
      json: async () => ({ session_id: "review", authorized: true }),
    });
    vi.stubGlobal("fetch", fetch);
    await authorizeWrite(
      "fake-paired-token",
      "janet",
      "review",
      "hash",
      "918273",
    );
    const [path, options] = fetch.mock.calls[0];
    expect(path).toBe("/api/agents/janet/write-authorizations");
    expect(options.cache).toBe("no-store");
    expect(options.redirect).toBe("error");
    expect(options.credentials).toBe("omit");
    expect(options.headers.Authorization).toBe("Bearer fake-paired-token");
    expect(JSON.parse(options.body)).toEqual({
      review_id: "review",
      reviewed_hash: "hash",
      code: "918273",
    });
  });
  it("does not dispatch malformed authorization input", () => {
    const fetch = vi.fn();
    vi.stubGlobal("fetch", fetch);
    expect(() =>
      authorizeWrite("token", "harley", "review", "hash", "12345x"),
    ).toThrow("Invalid authorization code");
    expect(fetch).not.toHaveBeenCalled();
  });
});
