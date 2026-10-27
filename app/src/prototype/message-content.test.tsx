import { describe, it, expect } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { MessageContent } from "./message-content";
const render = (content: string) =>
  renderToStaticMarkup(
    <MessageContent content={content} openThread={() => {}} />,
  );
describe("conversation cards", () => {
  it("replaces historical and partial internal requests without claiming success", () => {
    for (const content of [
      ' <internal_action>{"operation":"task.list"}</internal_action>',
      '<internal_action>{"operation":"task.create",',
    ]) {
      const html = render(content);
      expect(html).toContain("Internal operation requested");
      expect(html).not.toContain("task.list");
      expect(html).not.toContain("task.create");
    }
  });
  it("renders closed cards among readable conversation text", () => {
    const html = render(
      'Before\n\n```raaviclaw-card\n{"version":1,"type":"thread_reference","title":"Permit","summary":"Evidence ready","thread_id":"11111111-1111-4111-8111-111111111111"}\n```\n\nAfter',
    );
    expect(html).toContain("conversation-card");
    expect(html).toContain("Open Focus Thread");
    expect(html).toContain("Before");
    expect(html).toContain("After");
  });
  it("preserves malformed, unsupported and executable-looking cards as inert text", () => {
    for (const json of [
      "broken JSON",
      '{"version":2,"type":"info","title":"Future","summary":"Keep text"}',
      '{"version":1,"type":"info","title":"Bad","summary":"Keep text","onclick":"sendSecrets()"}',
    ]) {
      const html = render(`\`\`\`raaviclaw-card\n${json}\n\`\`\``);
      expect(html).not.toContain("conversation-card");
      expect(html).toContain("<pre>");
      expect(html).not.toContain("<button");
    }
    const html = render(
      '```raaviclaw-card\n{"version":1,"type":"info","title":"<img src=x>","summary":"<script>alert(1)</script>"}\n```',
    );
    expect(html).toContain("&lt;script&gt;");
    expect(html).not.toContain("<script");
    expect(html).not.toContain("<img");
  });
});
