// Shared between the main window (previewing the selected/current theme)
// and the Create Theme window (previewing colors as you edit them), so both
// always render identically.

export interface Palette {
  background: string;
  foreground: string;
  cursor: string | null;
  selection_background: string | null;
  selection_foreground: string | null;
  ansi: string[];
}

// A short "ghost related" snippet per the spec, syntax-highlighted using the
// palette's own ANSI colors so the preview shows real theme colors, not a
// fixed set of highlighter colors.
const PREVIEW_CODE = `// Ghosts don't do daily standups. They just haunt async.
fn haunt(house: &mut House) -> Result<Scream, Ghost> {
    if house.is_empty() {
        return Ok(Scream::Faint); // nobody home to spook
    }
    let boo = Ghost::new("Casper");
    boo.phase_through(&house.walls)?;
    Ok(Scream::BloodCurdling)
}`;

const KEYWORDS = new Set(["fn", "if", "else", "return", "let", "mut", "struct", "impl", "match", "for", "while"]);

type TokenClass = "fg" | "comment" | "keyword" | "type" | "string" | "func";

function tokenizeLine(line: string): { text: string; cls: TokenClass }[] {
  const commentIdx = line.indexOf("//");
  const code = commentIdx >= 0 ? line.slice(0, commentIdx) : line;
  const comment = commentIdx >= 0 ? line.slice(commentIdx) : "";

  const tokens: { text: string; cls: TokenClass }[] = [];
  const regex = /(".*?")|([A-Za-z_][A-Za-z0-9_]*)|(\s+)|([^\sA-Za-z0-9_]+)/g;
  let match: RegExpExecArray | null;
  while ((match = regex.exec(code))) {
    const [, str, word, space, punct] = match;
    if (str) tokens.push({ text: str, cls: "string" });
    else if (word) {
      if (KEYWORDS.has(word)) tokens.push({ text: word, cls: "keyword" });
      else if (/^[A-Z]/.test(word)) tokens.push({ text: word, cls: "type" });
      else tokens.push({ text: word, cls: "fg" });
    } else if (space) tokens.push({ text: space, cls: "fg" });
    else if (punct) tokens.push({ text: punct, cls: "fg" });
  }

  // A word immediately followed by "(" reads as a function call.
  for (let i = 0; i < tokens.length - 1; i++) {
    if (tokens[i].cls === "fg" && tokens[i + 1].text.startsWith("(")) {
      tokens[i] = { ...tokens[i], cls: "func" };
    }
  }

  if (comment) tokens.push({ text: comment, cls: "comment" });
  return tokens;
}

function colorFor(cls: TokenClass, palette: Palette): string {
  switch (cls) {
    case "fg":
      return palette.foreground;
    case "comment":
      return palette.ansi[8] || palette.foreground;
    case "keyword":
      return palette.ansi[5] || palette.foreground;
    case "type":
      return palette.ansi[3] || palette.foreground;
    case "string":
      return palette.ansi[2] || palette.foreground;
    case "func":
      return palette.ansi[4] || palette.foreground;
  }
}

// Renders `palette` into `windowEl`/`bodyEl` (a `.preview-window`/`.preview-body`
// pair), or shows `emptyMessage` if there's nothing to preview yet.
export function renderPreview(windowEl: HTMLElement, bodyEl: HTMLElement, palette: Palette | null, emptyMessage = "Select a theme to preview it here.") {
  if (!palette) {
    windowEl.style.background = "";
    windowEl.style.color = "";
    bodyEl.textContent = emptyMessage;
    return;
  }

  windowEl.style.background = `#${palette.background}`;
  windowEl.style.color = `#${palette.foreground}`;

  bodyEl.innerHTML = "";
  for (const line of PREVIEW_CODE.split("\n")) {
    for (const token of tokenizeLine(line)) {
      const span = document.createElement("span");
      span.textContent = token.text;
      span.style.color = `#${colorFor(token.cls, palette)}`;
      bodyEl.appendChild(span);
    }
    bodyEl.appendChild(document.createTextNode("\n"));
  }
}
