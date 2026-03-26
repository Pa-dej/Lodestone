const ANSI_PATTERN = /\u001b\[([0-9;]+)m/g;

const FG_COLORS: Record<number, string> = {
  30: "#000000", // Black
  31: "#cd3131", // Red
  32: "#0dbc79", // Green
  33: "#e5e510", // Yellow
  34: "#2472c8", // Blue
  35: "#bc3fbc", // Magenta
  36: "#11a8cd", // Cyan
  37: "#e5e5e5", // White
  90: "#666666", // Bright Black (Gray)
  91: "#f14c4c", // Bright Red
  92: "#23d18b", // Bright Green
  93: "#f5f543", // Bright Yellow
  94: "#3b8eea", // Bright Blue
  95: "#d670d6", // Bright Magenta
  96: "#29b8db", // Bright Cyan
  97: "#ffffff", // Bright White
};

const BG_COLORS: Record<number, string> = {
  40: "#000000",
  41: "#cd3131",
  42: "#0dbc79",
  43: "#e5e510",
  44: "#2472c8",
  45: "#bc3fbc",
  46: "#11a8cd",
  47: "#e5e5e5",
  100: "#666666",
  101: "#f14c4c",
  102: "#23d18b",
  103: "#f5f543",
  104: "#3b8eea",
  105: "#d670d6",
  106: "#29b8db",
  107: "#ffffff",
};

// 256-color palette (xterm colors)
function get256Color(index: number): string {
  if (index < 16) {
    // Standard colors
    const standardColors = [
      "#000000", "#cd3131", "#0dbc79", "#e5e510", "#2472c8", "#bc3fbc", "#11a8cd", "#e5e5e5",
      "#666666", "#f14c4c", "#23d18b", "#f5f543", "#3b8eea", "#d670d6", "#29b8db", "#ffffff"
    ];
    return standardColors[index];
  } else if (index < 232) {
    // 216 color cube (6x6x6)
    const i = index - 16;
    const r = Math.floor(i / 36);
    const g = Math.floor((i % 36) / 6);
    const b = i % 6;
    const toHex = (v: number) => {
      const val = v === 0 ? 0 : 55 + v * 40;
      return val.toString(16).padStart(2, "0");
    };
    return `#${toHex(r)}${toHex(g)}${toHex(b)}`;
  } else {
    // Grayscale (24 shades)
    const gray = 8 + (index - 232) * 10;
    const hex = gray.toString(16).padStart(2, "0");
    return `#${hex}${hex}${hex}`;
  }
}

function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;");
}

interface StyleState {
  fg?: string;
  bg?: string;
  bold?: boolean;
  dim?: boolean;
  italic?: boolean;
  underline?: boolean;
  blink?: boolean;
  reverse?: boolean;
  hidden?: boolean;
  strikethrough?: boolean;
}

function buildStyleString(state: StyleState): string {
  const styles: string[] = [];
  
  // Reverse меняет местами fg и bg
  const fg = state.reverse ? (state.bg || "#e5e5e5") : state.fg;
  const bg = state.reverse ? (state.fg || "#000000") : state.bg;
  
  if (fg) styles.push(`color:${fg}`);
  if (bg) styles.push(`background-color:${bg}`);
  if (state.bold) styles.push("font-weight:700");
  if (state.dim) styles.push("opacity:0.6");
  if (state.italic) styles.push("font-style:italic");
  if (state.underline) styles.push("text-decoration:underline");
  if (state.strikethrough) styles.push("text-decoration:line-through");
  if (state.blink) styles.push("animation:blink 1s step-end infinite");
  if (state.hidden) styles.push("visibility:hidden");
  
  return styles.length > 0 ? styles.join(";") : "";
}

export function ansiToHtml(value: string): string {
  let html = "";
  let cursor = 0;
  let currentState: StyleState = {};
  let hasOpenSpan = false;

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

    let i = 0;
    while (i < codes.length) {
      const code = codes[i];

      // Reset
      if (code === 0) {
        if (hasOpenSpan) {
          html += "</span>";
          hasOpenSpan = false;
        }
        currentState = {};
        i++;
        continue;
      }

      // Styles
      if (code === 1) {
        currentState.bold = true;
      } else if (code === 2) {
        currentState.dim = true;
      } else if (code === 3) {
        currentState.italic = true;
      } else if (code === 4) {
        currentState.underline = true;
      } else if (code === 5 || code === 6) {
        // 5 = slow blink, 6 = rapid blink
        currentState.blink = true;
      } else if (code === 7) {
        currentState.reverse = true;
      } else if (code === 8) {
        currentState.hidden = true;
      } else if (code === 9) {
        currentState.strikethrough = true;
      } else if (code === 22) {
        currentState.bold = false;
        currentState.dim = false;
      } else if (code === 23) {
        currentState.italic = false;
      } else if (code === 24) {
        currentState.underline = false;
      } else if (code === 25) {
        currentState.blink = false;
      } else if (code === 27) {
        currentState.reverse = false;
      } else if (code === 28) {
        currentState.hidden = false;
      } else if (code === 29) {
        currentState.strikethrough = false;
      }
      // Foreground colors (standard 16)
      else if (FG_COLORS[code]) {
        currentState.fg = FG_COLORS[code];
      }
      // Background colors (standard 16)
      else if (BG_COLORS[code]) {
        currentState.bg = BG_COLORS[code];
      }
      // 256-color foreground: ESC[38;5;Nm
      else if (code === 38 && codes[i + 1] === 5 && codes[i + 2] !== undefined) {
        currentState.fg = get256Color(codes[i + 2]);
        i += 2;
      }
      // True color foreground: ESC[38;2;R;G;Bm
      else if (code === 38 && codes[i + 1] === 2 && codes[i + 4] !== undefined) {
        const r = codes[i + 2];
        const g = codes[i + 3];
        const b = codes[i + 4];
        currentState.fg = `rgb(${r},${g},${b})`;
        i += 4;
      }
      // 256-color background: ESC[48;5;Nm
      else if (code === 48 && codes[i + 1] === 5 && codes[i + 2] !== undefined) {
        currentState.bg = get256Color(codes[i + 2]);
        i += 2;
      }
      // True color background: ESC[48;2;R;G;Bm
      else if (code === 48 && codes[i + 1] === 2 && codes[i + 4] !== undefined) {
        const r = codes[i + 2];
        const g = codes[i + 3];
        const b = codes[i + 4];
        currentState.bg = `rgb(${r},${g},${b})`;
        i += 4;
      }
      // Default foreground
      else if (code === 39) {
        currentState.fg = undefined;
      }
      // Default background
      else if (code === 49) {
        currentState.bg = undefined;
      }

      i++;
    }

    // Apply current state
    if (hasOpenSpan) {
      html += "</span>";
    }
    
    const styleStr = buildStyleString(currentState);
    if (styleStr) {
      html += `<span style="${styleStr}">`;
      hasOpenSpan = true;
    } else {
      hasOpenSpan = false;
    }
  }

  html += escapeHtml(value.slice(cursor));
  
  if (hasOpenSpan) {
    html += "</span>";
  }

  return html;
}
