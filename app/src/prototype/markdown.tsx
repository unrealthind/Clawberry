import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

/** Shared presentation for chat and documents; raw HTML never executes. */
export function Markdown({ children }: { children: string }) {
  return (
    <div className="markdown-content">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        skipHtml
        components={{
          a: ({ href, children }) =>
            href ? (
              <a href={href} target="_blank" rel="noopener noreferrer">
                {children}
              </a>
            ) : (
              <span>{children}</span>
            ),
          // Documents must not trigger remote image requests merely by being opened.
          img: ({ alt }) => (
            <span className="markdown-image-label">{alt || "Image"}</span>
          ),
          table: ({ children }) => (
            <div className="markdown-table-scroll">
              <table>{children}</table>
            </div>
          ),
        }}
      >
        {children}
      </ReactMarkdown>
    </div>
  );
}
