import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { Markdown } from "./markdown";

describe("Markdown presentation", () => {
  it("renders task replies, fenced code and GitHub tables", () => {
    const html = renderToStaticMarkup(
      <Markdown>
        {
          "# Task\n\n- **Status:** Queued\n- [x] Reviewed\n\n```sh\necho ready\n```\n\n| Name | State |\n| --- | --- |\n| Permit | Queued |"
        }
      </Markdown>,
    );
    expect(html).toContain("<h1>Task</h1>");
    expect(html).toContain("<strong>Status:</strong>");
    expect(html).toContain("<ul");
    expect(html).toContain('<pre><code class="language-sh">');
    expect(html).toContain("<table>");
    expect(html).toContain('type="checkbox"');
  });
  it("does not execute HTML, unsafe links or fetch remote images", () => {
    const html = renderToStaticMarkup(
      <Markdown>
        {
          "<script>alert(1)</script>\n\n[bad](javascript:alert%281%29)\n\n![Tracker](https://example.org/track.png)\n\n[Source](https://example.org)"
        }
      </Markdown>,
    );
    expect(html).not.toContain("<script");
    expect(html).not.toContain('href="javascript:');
    expect(html).not.toContain("<img");
    expect(html).toContain('rel="noopener noreferrer"');
    expect(html).toContain("Tracker");
  });
});
