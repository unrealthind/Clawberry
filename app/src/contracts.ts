import { z } from "zod";
const owner = z.enum(["janet", "harley"]);
const text = z.string();
const number = z.number().finite();
const task = z.object({
  id: text,
  title: text,
  description: text,
  status: text,
  priority: z.enum(["low", "normal", "high", "urgent"]),
  thread_id: text.nullable(),
  due_at: text.nullable(),
});
const thread = z.object({ id: text, title: text, summary: text });
const file = z.object({
  id: text,
  filename: text,
  checksum: text,
  size: number,
  source: text,
});
const memory = z.object({
  id: text,
  agent_owner: owner,
  content: text,
  category: text,
  confidence: number,
  source_type: text,
  source_ref: text,
  sensitivity: text,
  expires_at: text.nullable(),
  visibility: text,
  stale: z.boolean(),
  created_at: text,
  last_verified_at: text.nullable(),
  review_hash: text,
});
const watch = z.object({
  enabled: z.boolean(),
  interval_minutes: number,
  quiet_start_utc: number,
  quiet_end_utc: number,
  daily_runs: number,
  daily_budget_microusd: number,
  investigation_budget_microusd: number,
  stale_after_hours: number,
});
const notice = z.object({
  id: text,
  title: text,
  summary: text,
  status: text,
  task_id: text,
  created_at: number,
  read: z.boolean(),
  evidence: z
    .array(z.object({ source_type: text, source_ref: text, content: text }))
    .optional(),
});
const review = z.object({
  id: text,
  task_id: text,
  workflow: text,
  manifest: z.record(text, z.unknown()),
  manifest_hash: text,
  expires_at: number,
  state: text,
});
const exchange = z.object({
  id: text,
  requester: owner,
  recipient: owner,
  task_id: text,
  question: text,
  question_hash: text,
  response: text.nullable(),
  response_hash: text.nullable(),
  status: text,
  expires_at: number,
});
const result = z.object({
  summary: text,
  findings: z.array(text),
  confidence: number,
  sources: z.array(z.object({ source_type: text, source_ref: text })),
  artifacts: z.array(z.object({ filename: text })),
  errors: z.array(text),
});
const worker = z.object({
  id: text,
  task_id: text,
  status: text,
  spec: z.object({ role: text, objective: text }),
  result: result.nullable().optional(),
});
const page = (item: z.ZodType) =>
  z.object({
    items: z.array(item),
    next_cursor: number.int().positive().nullable(),
  });
// Validate rendered reads and record-creation acknowledgements. Validation failure
// never retries a mutation or grants authority; callers must reconcile uncertainty.
const reads: Record<string, z.ZodType> = {
  "task.get": z.object({
    task,
    events: z.array(z.object({ event: text, detail: text, created_at: text })),
    workers: z.array(
      z.object({
        id: text,
        status: text,
        error: text.nullable().optional(),
        spec: z.object({ role: text }),
        result: result
          .omit({ artifacts: true, errors: true })
          .nullable()
          .optional(),
      }),
    ),
    attachments: z.array(
      z.object({ id: text, filename: text, checksum: text }),
    ),
  }),
  "write.status": z.object({
    expires_at: number,
    revoked: z.boolean(),
    gateway_end_confirmed: z.boolean().nullable().optional(),
    remaining: z.record(text, number.int().nonnegative()),
    actions: z.array(
      z.object({
        id: text,
        operation: text,
        status: text,
        gateway_action_id: text.nullable().optional(),
        gateway_audit_id: text.nullable().optional(),
        n8n_execution_id: text.nullable().optional(),
      }),
    ),
  }),
  "task.create": task,
  "task.edit": task,
  "thread.create": thread,
  "task.page": page(task),
  "thread.page": page(thread),
  "thread.get": thread,
  "task.list": z.array(task),
  "thread.list": z.array(thread),
  "file.list": z.array(file),
  "agent.list": z.array(worker),
  "chat.page": page(z.object({ role: text, content: text })),
  "memory.page": page(memory),
  "shared.page": page(memory),
  "notification.list": page(notice),
  "write.reviews": z.array(review),
  "cross.review_queue": z.array(exchange),
  "watch.settings": watch,
  "usage.list": z.array(
    z.object({
      input_tokens: number,
      output_tokens: number,
      cost_microusd: number,
      usage_reported: z.boolean(),
    }),
  ),
  "activity.list": z.object({
    events: z.array(
      z.object({
        request_id: text,
        event: text,
        detail: text,
        created_at: text,
      }),
    ),
    workers: z.array(z.object({ id: text, status: text, task_id: text })),
  }),
  "thread.decisions": z.array(
    z.object({ id: text, content: text, created_at: text }),
  ),
};
export function assertReadContract(operation: string, value: unknown) {
  if (reads[operation] && !reads[operation].safeParse(value).success)
    throw new Error(
      "The gateway returned an invalid response. Refresh to retry.",
    );
}

const profile = z.object({
  provider: text,
  model: text,
  max_output_tokens: number.int().positive(),
  input_microusd_per_million: number.nonnegative(),
  output_microusd_per_million: number.nonnegative(),
});
const globalSettings = z.object({
  preferences: z.object({
    theme: z.enum(["light", "dark"]),
    notifications: z.boolean(),
    week_start: z.enum(["sunday", "monday"]),
    time_format: z.enum(["12", "24"]),
    show_completed: z.boolean(),
  }),
  models: z.object({
    profiles: z.object({ fast: profile, normal: profile, deep: profile }),
    agent_daily_microusd: number.nonnegative(),
    deep_daily_microusd: number.nonnegative(),
    task_microusd: number.nonnegative(),
  }),
});
const agentSettings = z.object({
  soul: text,
  memory_notes: text,
  default_profile: z.enum(["fast", "normal", "deep"]),
  skills: z.array(
    z.object({
      id: text,
      name: text,
      description: text,
      files: z.array(z.object({ path: text, content: text })),
    }),
  ),
});
const versioned = (value: z.ZodType) =>
  z.object({ revision: number.int().nonnegative(), value });
// Settings saves are also versioned responses: reject malformed acknowledgements
// without replaying the save. The caller must explicitly refresh after uncertainty.
export function assertEndpointContract(path: string, value: unknown) {
  let schema: z.ZodType | undefined;
  if (path === "/api/settings/global") schema = versioned(globalSettings);
  else if (/^\/api\/agents\/(janet|harley)\/settings$/.test(path))
    schema = versioned(agentSettings);
  else if (path === "/api/runtime/status")
    schema = z.object({
      credential_configured: z.boolean(),
      credential_source: text,
      live_verified: z.boolean(),
    });
  else if (/^\/api\/agents\/(janet|harley)\/write-authorizations$/.test(path))
    schema = z.object({
      session_id: text.min(1),
      authorized: z.literal(true),
    });
  if (schema && !schema.safeParse(value).success)
    throw new Error(
      "The gateway returned an invalid response. Refresh to retry.",
    );
}
