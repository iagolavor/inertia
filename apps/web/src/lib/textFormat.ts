export type InlineFormat = 'bold' | 'italic' | 'strike' | 'code' | 'link';

const INLINE_MARKERS: Record<Exclude<InlineFormat, 'link'>, { before: string; after: string; placeholder: string }> = {
  bold: { before: '**', after: '**', placeholder: 'bold' },
  italic: { before: '*', after: '*', placeholder: 'italic' },
  strike: { before: '~~', after: '~~', placeholder: 'strike' },
  code: { before: '`', after: '`', placeholder: 'code' }
};

export function wrapTextareaSelection(
  textarea: HTMLTextAreaElement,
  before: string,
  after: string,
  placeholder = ''
): { value: string; selectionStart: number; selectionEnd: number } {
  const { value, selectionStart, selectionEnd } = textarea;
  const selected = value.slice(selectionStart, selectionEnd);
  const inner = selected || placeholder;
  const next = value.slice(0, selectionStart) + before + inner + after + value.slice(selectionEnd);

  const start = selectionStart + before.length;
  const end = start + inner.length;
  return { value: next, selectionStart: start, selectionEnd: end };
}

export function applyInlineFormat(textarea: HTMLTextAreaElement, format: InlineFormat): string {
  if (format === 'link') {
    const { value, selectionStart, selectionEnd } = textarea;
    const selected = value.slice(selectionStart, selectionEnd);
    const label = selected || 'link';
    const before = '[';
    const middle = '](';
    const after = ')';
    const next =
      value.slice(0, selectionStart) +
      before +
      label +
      middle +
      'https://' +
      after +
      value.slice(selectionEnd);
    const urlStart = selectionStart + before.length + label.length + middle.length;
    const urlEnd = urlStart + 'https://'.length;
    queueMicrotask(() => {
      textarea.focus();
      textarea.setSelectionRange(urlStart, urlEnd);
    });
    return next;
  }

  const markers = INLINE_MARKERS[format];
  const { value, selectionStart, selectionEnd } = wrapTextareaSelection(
    textarea,
    markers.before,
    markers.after,
    markers.placeholder
  );
  queueMicrotask(() => {
    textarea.focus();
    textarea.setSelectionRange(selectionStart, selectionEnd);
  });
  return value;
}

export function prefixSelectedLines(textarea: HTMLTextAreaElement, prefix: string): string {
  const { value, selectionStart, selectionEnd } = textarea;
  const lineStart = value.lastIndexOf('\n', selectionStart - 1) + 1;
  const lineEndIdx = value.indexOf('\n', selectionEnd);
  const lineEnd = lineEndIdx === -1 ? value.length : lineEndIdx;
  const block = value.slice(lineStart, lineEnd);
  const lines = block.split('\n');
  const prefixed = lines.map((line) => (line.startsWith(prefix) ? line : `${prefix}${line}`)).join('\n');
  const next = value.slice(0, lineStart) + prefixed + value.slice(lineEnd);
  queueMicrotask(() => {
    textarea.focus();
    textarea.setSelectionRange(lineStart, lineStart + prefixed.length);
  });
  return next;
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;');
}

/** Renders a small safe subset of inline markdown. */
export function formatBasicMarkdown(text: string): string {
  let html = escapeHtml(text);

  html = html.replace(/`([^`\n]+)`/g, '<code>$1</code>');
  html = html.replace(/\*\*([^*\n]+)\*\*/g, '<strong>$1</strong>');
  html = html.replace(/\*([^*\n]+)\*/g, '<em>$1</em>');
  html = html.replace(/~~([^~\n]+)~~/g, '<del>$1</del>');
  html = html.replace(/\[([^\]]+)\]\(([^)]+)\)/g, (_match, label: string, url: string) => {
    const trimmed = url.trim();
    if (!/^https?:\/\//i.test(trimmed)) {
      return `[${label}](${url})`;
    }
    return `<a href="${escapeHtml(trimmed)}" target="_blank" rel="noopener noreferrer">${label}</a>`;
  });

  return html
    .split('\n')
    .map((line) => {
      const bullet = /^- (.+)$/.exec(line);
      if (bullet) return `&#8226; ${bullet[1]}`;
      return line;
    })
    .join('<br>');
}
