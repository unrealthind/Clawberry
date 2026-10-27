// Only allowlisted, nonsecret configuration is persisted by this editor.
export const claudeModels = [
  {
    id: "claude-haiku-4-5-20251001",
    name: "Claude Haiku 4.5",
    detail: "Existing runtime Fast default",
  },
  {
    id: "claude-haiku-5-5",
    name: "Claude Haiku 5.5",
    detail: "High-volume tasks",
  },
  {
    id: "claude-sonnet-5-5",
    name: "Claude Sonnet 5.5",
    detail: "Everyday work",
  },
  { id: "claude-opus-5-5", name: "Claude Opus 5.5", detail: "Deep reasoning" },
  {
    id: "claude-fable-5-1",
    name: "Claude Fable 5.1",
    detail: "Demanding reasoning",
  },
] as const;
export type ModelId = (typeof claudeModels)[number]["id"];
export const profiles = ["fast", "normal", "deep"] as const;
export type Profile = (typeof profiles)[number];
export type ModelDraft = {
  version: 1;
  provider: "anthropic";
  routes: Record<Profile, { model: ModelId; maxOutputTokens: number }>;
  budgets: { agentDaily: number; deepDaily: number; task: number };
};
export const modelDraftKey = "raaviclaw.prototype.model-draft.v1";
export const defaultModelDraft: ModelDraft = {
  version: 1,
  provider: "anthropic",
  routes: {
    fast: { model: "claude-haiku-4-5-20251001", maxOutputTokens: 4096 },
    normal: { model: "claude-sonnet-5-5", maxOutputTokens: 4096 },
    deep: { model: "claude-opus-5-5", maxOutputTokens: 4096 },
  },
  budgets: { agentDaily: 5, deepDaily: 2, task: 2 },
};
function exact(
  value: unknown,
  keys: string[],
): value is Record<string, unknown> {
  return (
    !!value &&
    typeof value === "object" &&
    !Array.isArray(value) &&
    Object.keys(value).length === keys.length &&
    Object.keys(value).every((key) => keys.includes(key))
  );
}
export function validateModelDraft(value: unknown): value is ModelDraft {
  if (
    !exact(value, ["version", "provider", "routes", "budgets"]) ||
    value.version !== 1 ||
    value.provider !== "anthropic" ||
    !exact(value.routes, [...profiles]) ||
    !exact(value.budgets, ["agentDaily", "deepDaily", "task"])
  )
    return false;
  for (const profile of profiles) {
    const route = value.routes[profile];
    if (
      !exact(route, ["model", "maxOutputTokens"]) ||
      !claudeModels.some((model) => model.id === route.model) ||
      typeof route.maxOutputTokens !== "number" ||
      !Number.isInteger(route.maxOutputTokens) ||
      route.maxOutputTokens < 1 ||
      route.maxOutputTokens > 32000
    )
      return false;
  }
  return Object.values(value.budgets).every(
    (amount) =>
      typeof amount === "number" &&
      Number.isFinite(amount) &&
      amount >= 0 &&
      amount <= 10000 &&
      Math.abs(Math.round(amount * 100) - amount * 100) < 0.00001,
  );
}
export function readModelDraft(): { draft: ModelDraft; error: string } {
  try {
    const raw = localStorage.getItem(modelDraftKey);
    if (raw === null)
      return { draft: structuredClone(defaultModelDraft), error: "" };
    if (raw.length > 10000) throw new Error();
    const value: unknown = JSON.parse(raw);
    if (!validateModelDraft(value)) throw new Error();
    return { draft: value, error: "" };
  } catch {
    return {
      draft: structuredClone(defaultModelDraft),
      error:
        "The saved model draft could not be read. It has been preserved; saving is disabled. You can still explore the editor.",
    };
  }
}
export function saveModelDraft(draft: ModelDraft) {
  if (!validateModelDraft(draft))
    throw new Error(
      "Check model choices, output limits, and budgets before saving.",
    );
  localStorage.setItem(modelDraftKey, JSON.stringify(draft));
}
