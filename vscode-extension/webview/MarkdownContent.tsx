import * as React from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';

interface Props {
  text: string;
}

/** Renders Claude's text blocks as markdown using react-markdown + GFM
 *  (tables, strikethrough, autolinks, task lists). Components are
 *  overridden to use VSCode theme tokens so the result blends with
 *  the editor — no syntax highlighter (kept the bundle and the deps
 *  list small; the user's eyes are on cc-connect, not source diving). */
export function MarkdownContent({ text }: Props): React.ReactElement {
  return (
    <div className="md">
      <ReactMarkdown
        remarkPlugins={[remarkGfm]}
        components={{
          a: ({ href, children }) => (
            <a href={href} target="_blank" rel="noreferrer noopener">
              {children}
            </a>
          ),
          code({ className, children, ...rest }) {
            // react-markdown 9 distinguishes inline vs. block via the
            // presence of a language class on the parent <pre>, not via
            // an `inline` prop. We render <code> for both cases; CSS
            // styles inline differently from the block (inside <pre>).
            const lang = /language-(\w+)/.exec(className || '')?.[1];
            return (
              <code className={className} data-lang={lang} {...rest}>
                {children}
              </code>
            );
          },
          pre: ({ children }) => <pre className="md-pre">{children}</pre>,
        }}
      >
        {text}
      </ReactMarkdown>
    </div>
  );
}
