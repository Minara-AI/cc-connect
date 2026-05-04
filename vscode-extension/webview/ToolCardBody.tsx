import * as React from 'react';

const PREVIEW_LINES = 3;
const PREVIEW_CHARS = 200;

/** Generic expandable preformatted text. Shows the first ~3 lines /
 *  ~200 chars by default; click "show more" to reveal the rest. Used
 *  by tool-card result rendering for any tool whose output we don't
 *  have a specialised view for. */
export function ExpandableText({
  text,
  isError,
}: {
  text: string;
  isError?: boolean;
}): React.ReactElement {
  const [open, setOpen] = React.useState(false);
  const lines = text.split('\n');
  const oversize =
    text.length > PREVIEW_CHARS || lines.length > PREVIEW_LINES;
  const preview = oversize
    ? lines.slice(0, PREVIEW_LINES).join('\n').slice(0, PREVIEW_CHARS)
    : text;
  const cls = isError
    ? 'tool-body tool-body-error'
    : 'tool-body';
  return (
    <div className={cls}>
      <pre className="tool-body-pre">{open || !oversize ? text : preview}</pre>
      {oversize && (
        <button
          type="button"
          className="tool-expand"
          onClick={() => setOpen((v) => !v)}
        >
          {open ? 'show less' : `show more (${lines.length} lines)`}
        </button>
      )}
    </div>
  );
}

/** Edit / Write / MultiEdit get a side-by-side red-then-green view of
 *  the input's old_string / new_string. No LCS — Claude already
 *  decided what to replace, we just present it. */
export function EditDiffView({
  input,
}: {
  input: Record<string, unknown>;
}): React.ReactElement {
  const oldText = typeof input.old_string === 'string' ? input.old_string : '';
  const newText = typeof input.new_string === 'string' ? input.new_string : '';
  if (!oldText && !newText) return <></>;
  return (
    <div className="diff">
      {oldText && (
        <pre className="diff-old">
          {oldText
            .split('\n')
            .map((l, i) => `- ${l}` + (i === oldText.split('\n').length - 1 ? '' : '\n'))
            .join('')}
        </pre>
      )}
      {newText && (
        <pre className="diff-new">
          {newText
            .split('\n')
            .map((l, i) => `+ ${l}` + (i === newText.split('\n').length - 1 ? '' : '\n'))
            .join('')}
        </pre>
      )}
    </div>
  );
}

/** Bash result view: just preformatted, but split common stdout/stderr
 *  prefixes if Claude/the SDK formatted them that way. v0 keeps it
 *  simple — same ExpandableText, just different parent class for
 *  monospace styling. */
export function BashResultView({
  text,
  isError,
}: {
  text: string;
  isError?: boolean;
}): React.ReactElement {
  return <ExpandableText text={text} isError={isError} />;
}
