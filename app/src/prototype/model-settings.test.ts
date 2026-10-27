import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import {
  defaultModelDraft,
  modelDraftKey,
  readModelDraft,
  saveModelDraft,
  validateModelDraft,
} from "./model-settings";
describe("model configuration drafts", () => {
  afterEach(() => vi.unstubAllGlobals());
  let values: Map<string, string>;
  beforeEach(() => {
    values = new Map();
    vi.stubGlobal("localStorage", {
      getItem: (key: string) => values.get(key) ?? null,
      setItem: (key: string, value: string) => values.set(key, value),
    });
  });
  it("round-trips only allowlisted model configuration", () => {
    const draft = structuredClone(defaultModelDraft);
    draft.routes.normal.model = "claude-opus-5-5";
    saveModelDraft(draft);
    expect(readModelDraft()).toEqual({ draft, error: "" });
    expect(JSON.parse(values.get(modelDraftKey)!)).not.toHaveProperty(
      "api_key",
    );
  });
  it("rejects unexpected secrets, unknown models, invalid budgets and token limits", () => {
    expect(
      validateModelDraft({ ...defaultModelDraft, api_key: "do-not-store" }),
    ).toBe(false);
    for (const maxOutputTokens of [0, 32001, 1.1, NaN, Infinity]) {
      const value = structuredClone(defaultModelDraft);
      value.routes.normal.maxOutputTokens = maxOutputTokens;
      expect(validateModelDraft(value)).toBe(false);
    }
    for (const agentDaily of [-1, 0.001, 10001, NaN, Infinity]) {
      expect(
        validateModelDraft({
          ...defaultModelDraft,
          budgets: { ...defaultModelDraft.budgets, agentDaily },
        }),
      ).toBe(false);
    }
    expect(
      validateModelDraft({
        ...defaultModelDraft,
        routes: {
          ...defaultModelDraft.routes,
          normal: { model: "caller-controlled-model", maxOutputTokens: 4096 },
        },
      }),
    ).toBe(false);
  });
  it("preserves corrupt or unsupported stored drafts", () => {
    const raw = '{"version":2,"unexpected":"preserve"}';
    values.set(modelDraftKey, raw);
    expect(readModelDraft().error).toContain("preserved");
    expect(values.get(modelDraftKey)).toBe(raw);
  });
  it("surfaces storage failures without changing draft data", () => {
    localStorage.setItem = () => {
      throw new Error("Storage full");
    };
    expect(() => saveModelDraft(defaultModelDraft)).toThrow("Storage full");
    expect(defaultModelDraft.routes.normal.model).toBe("claude-sonnet-5-5");
  });
});
