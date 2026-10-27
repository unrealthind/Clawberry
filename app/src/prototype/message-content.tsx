import { cardSchema, RichCard } from "../cards";
import { Markdown } from "./markdown";

/** Only a closed, explicitly tagged card block can become interactive UI. */
export function MessageContent({
  content,
  openThread,
}: {
  content: string;
  openThread: (id: string) => void;
}) {
  // Older servers could persist or stream the protocol envelope as chat text.
  // This is a request, never evidence that an operation completed.
  if (content.trimStart().startsWith("<internal_action>"))
    return <p>Internal operation requested. Check Activity for its result.</p>;
  const blocks = content.split(
    /(^```raaviclaw-card[ \t]*\r?\n[\s\S]*?^```[ \t]*$)/gm,
  );
  return (
    <>
      {blocks.map((block, index) => {
        if (/^```raaviclaw-card[ \t]*\r?\n/.test(block)) {
          try {
            const value: unknown = JSON.parse(
              block
                .replace(/^```raaviclaw-card\s*\n/, "")
                .replace(/\n```\s*$/, ""),
            );
            if (cardSchema.safeParse(value).success)
              return (
                <RichCard key={index} value={value} openThread={openThread} />
              );
          } catch {
            /* Preserve malformed and unsupported cards as readable text. */
          }
        }
        return block ? <Markdown key={index}>{block}</Markdown> : null;
      })}
    </>
  );
}
