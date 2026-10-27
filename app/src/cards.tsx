import { z } from "zod";

// This is a closed, versioned vocabulary: no HTML, scripts, arbitrary URLs,
// handler names, or model-authored frontend code.
export const cardSchema = z
  .object({
    version: z.literal(1),
    type: z.enum([
      "recommendation",
      "task",
      "security_event",
      "info",
      "thread_reference",
    ]),
    title: z.string().min(1).max(200),
    summary: z.string().max(8000),
    status: z.string().max(80).optional(),
    thread_id: z.string().uuid().optional(),
  })
  .strict()
  .superRefine((value, context) => {
    if (value.type === "thread_reference" && !value.thread_id)
      context.addIssue({
        code: "custom",
        message: "Thread reference requires a thread ID",
      });
  });

export function RichCard({
  value,
  openThread,
}: {
  value: unknown;
  openThread: (id: string) => void;
}) {
  const parsed = cardSchema.safeParse(value);
  if (!parsed.success)
    return <p>Unsupported card. Open the original conversation for context.</p>;
  const card = parsed.data;
  return (
    <section
      className="conversation-card"
      aria-label={`${card.type.replaceAll("_", " ")} card`}
    >
      <h3>{card.title}</h3>
      {card.status && (
        <span className="live-status">{card.status.replaceAll("_", " ")}</span>
      )}
      <p className="live-exact-text">{card.summary}</p>
      {card.type === "thread_reference" && (
        <button
          type="button"
          className="secondary-button"
          onClick={() => openThread(card.thread_id!)}
        >
          Open Focus Thread
        </button>
      )}
    </section>
  );
}
