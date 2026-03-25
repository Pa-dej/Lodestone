const ANSI_PATTERN = /\u001b\[([0-9;]+)m/g;

const FG_COLORS: Record<number, string> = {
  30: "var(--text-hint)",
  31: "var(--error-color)",
  32: "var(--success-color)",
  33: "var(--warn-color)",
  34: "var(--info-color)",
  35: "var(--accent)",
  36: "var(--info-color)",
  37: "var(--text-muted)",
  90: "var(--text-hint)",
  91: "var(--error-color)",
  92: "var(--success-color)",
  93: "var(--warn-color)",
  94: "var(--info-color)",
  95: "var(--accent)",
  96: "var(--info-color)",
  97: "var(--text)",
};

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

export function ansiToHtml(value: string): string {
  let html = "";
  let cursor = 0;
  let activeSpans = 0;

  for (const match of value.matchAll(ANSI_PATTERN)) {
    const start = match.index ?? 0;
    html += escapeHtml(value.slice(cursor, start));
    cursor = start + match[0].length;

    const codes = match[1]
      .split(";")
      .map((entry) => Number.parseInt(entry, 10))
      .filter((entry) => Number.isFinite(entry));

    if (codes.length === 0) {
      continue;
    }

    for (const code of codes) {
      if (code === 0) {
        while (activeSpans > 0) {
          html += "</span>";
          activeSpans -= 1;
        }
        continue;
      }

      if (code === 1) {
        html += '<span style="font-weight:700">';
        activeSpans += 1;
        continue;
      }

      const fgColor = FG_COLORS[code];
      if (fgColor) {
        html += `<span style="color:${fgColor}">`;
        activeSpans += 1;
      }
    }
  }

  html += escapeHtml(value.slice(cursor));
  while (activeSpans > 0) {
    html += "</span>";
    activeSpans -= 1;
  }

  return html;
}
