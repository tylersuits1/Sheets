import { invoke } from "@tauri-apps/api/core";
import { emit } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { renderPreview, type Palette } from "./theme-preview";

type ThemeVariant = "dark" | "light" | "both";

const ANSI_LABELS = [
  "0 Black", "1 Red", "2 Green", "3 Yellow", "4 Blue", "5 Magenta", "6 Cyan", "7 White",
  "8 Bright Black", "9 Bright Red", "10 Bright Green", "11 Bright Yellow",
  "12 Bright Blue", "13 Bright Magenta", "14 Bright Cyan", "15 Bright White",
];

const DEFAULT_ANSI = [
  "1d2021", "cc241d", "98971a", "d79921", "458588", "b16286", "689d6a", "a89984",
  "928374", "fb4934", "b8bb26", "fabd2f", "83a598", "d3869b", "8ec07c", "ebdbb2",
];

interface ColorField {
  row: HTMLElement;
  get(): string;
  set(hex: string): void;
}

function hexInputPattern(input: HTMLInputElement) {
  input.addEventListener("input", () => {
    input.value = input.value.replace(/[^0-9a-fA-F]/g, "").slice(0, 6);
  });
}

function makeColorField(label: string, initialHex: string): ColorField {
  const row = document.createElement("div");
  row.className = "color-row";

  const labelEl = document.createElement("span");
  labelEl.className = "color-label";
  labelEl.textContent = label;

  // WKWebView (Tauri's macOS webview) has a known bug where setting
  // `<input type="color">`.value from JS updates the value but doesn't
  // repaint the little swatch — so typing a hex code visually looks like
  // nothing happened even though it worked. This swatch is a plain div
  // whose background-color we set directly, so it always reflects reality.
  const swatch = document.createElement("span");
  swatch.className = "color-swatch";
  swatch.style.backgroundColor = `#${initialHex}`;

  const colorInput = document.createElement("input");
  colorInput.type = "color";
  colorInput.value = `#${initialHex}`;

  const textInput = document.createElement("input");
  textInput.type = "text";
  textInput.value = initialHex;
  textInput.maxLength = 6;
  hexInputPattern(textInput);

  colorInput.addEventListener("input", () => {
    textInput.value = colorInput.value.replace("#", "");
    swatch.style.backgroundColor = colorInput.value;
  });
  textInput.addEventListener("input", () => {
    if (/^[0-9a-fA-F]{6}$/.test(textInput.value)) {
      colorInput.value = `#${textInput.value}`;
      swatch.style.backgroundColor = `#${textInput.value}`;
    }
  });

  row.append(labelEl, swatch, colorInput, textInput);

  return {
    row,
    get: () => textInput.value.toLowerCase(),
    set: (hex: string) => {
      textInput.value = hex;
      colorInput.value = `#${hex}`;
      swatch.style.backgroundColor = `#${hex}`;
    },
  };
}

const previewWindowEl = document.querySelector<HTMLElement>("#preview-window")!;
const previewBodyEl = document.querySelector<HTMLElement>("#preview-body")!;
const editorTitleEl = document.querySelector<HTMLElement>("#editor-title")!;
const editorHintEl = document.querySelector<HTMLElement>("#editor-hint")!;
const nameInput = document.querySelector<HTMLInputElement>("#theme-name")!;
const variantSelect = document.querySelector<HTMLSelectElement>("#theme-variant")!;
const coreColorsEl = document.querySelector<HTMLElement>("#core-colors")!;
const ansiNormalEl = document.querySelector<HTMLElement>("#ansi-normal")!;
const ansiBrightEl = document.querySelector<HTMLElement>("#ansi-bright")!;
const statusEl = document.querySelector<HTMLElement>("#form-status")!;
const cancelBtn = document.querySelector<HTMLButtonElement>("#cancel-btn")!;
const saveBtn = document.querySelector<HTMLButtonElement>("#save-btn")!;

const background = makeColorField("Background", "1d2021");
const foreground = makeColorField("Foreground", "ebdbb2");
const cursor = makeColorField("Cursor", "ebdbb2");
const selectionBackground = makeColorField("Selection background", "3c3836");
const selectionForeground = makeColorField("Selection foreground", "ebdbb2");
coreColorsEl.append(background.row, foreground.row, cursor.row, selectionBackground.row, selectionForeground.row);

const ansiFields: ColorField[] = ANSI_LABELS.map((label, i) => makeColorField(label, DEFAULT_ANSI[i]));
ansiFields.slice(0, 8).forEach((f) => ansiNormalEl.appendChild(f.row));
ansiFields.slice(8, 16).forEach((f) => ansiBrightEl.appendChild(f.row));

function errorMessage(err: unknown): string {
  return typeof err === "string" ? err : err instanceof Error ? err.message : String(err);
}

function currentPalette(): Palette {
  return {
    background: background.get(),
    foreground: foreground.get(),
    cursor: cursor.get(),
    selection_background: selectionBackground.get(),
    selection_foreground: selectionForeground.get(),
    ansi: ansiFields.map((f) => f.get()),
  };
}

function updatePreview() {
  renderPreview(previewWindowEl, previewBodyEl, currentPalette());
}

// Live preview: re-render on every color edit, whether from the wheel or
// typing a hex code. Delegated so it doesn't need touching `makeColorField`.
document.addEventListener("input", (e) => {
  if (e.target instanceof HTMLInputElement && e.target.closest(".color-row")) {
    updatePreview();
  }
});

updatePreview();

interface EditSeed {
  name: string;
  variant: ThemeVariant;
  palette: Palette;
}

// "Edit Current Theme" reuses this same window: the main window stashes the
// live colors here just before opening it, and we pick them up on load. A
// plain "File > Create Theme" leaves nothing pending, so this is a no-op.
(async () => {
  const seed = await invoke<EditSeed | null>("take_edit_seed");
  if (!seed) return;

  editorTitleEl.textContent = "Edit theme";
  editorHintEl.textContent =
    "Editing a pre-installed theme? Give it a different name below to save your " +
    "changes as a new theme — the original stays untouched. Editing one of your own " +
    "themes under the same name will just update it.";
  editorHintEl.hidden = false;

  nameInput.value = seed.name;
  variantSelect.value = seed.variant;
  background.set(seed.palette.background);
  foreground.set(seed.palette.foreground);
  if (seed.palette.cursor) cursor.set(seed.palette.cursor);
  if (seed.palette.selection_background) selectionBackground.set(seed.palette.selection_background);
  if (seed.palette.selection_foreground) selectionForeground.set(seed.palette.selection_foreground);
  seed.palette.ansi.forEach((hex, i) => ansiFields[i]?.set(hex));
  updatePreview();
})();

cancelBtn.addEventListener("click", () => {
  getCurrentWindow().close();
});

saveBtn.addEventListener("click", async () => {
  const name = nameInput.value.trim();
  if (!name) {
    statusEl.textContent = "Give the theme a name first.";
    statusEl.className = "status-message error";
    return;
  }

  try {
    const theme = await invoke("create_custom_theme", {
      name,
      variant: variantSelect.value as ThemeVariant,
      palette: currentPalette(),
    });
    await emit("theme-created", theme);
    getCurrentWindow().close();
  } catch (err) {
    statusEl.textContent = errorMessage(err);
    statusEl.className = "status-message error";
  }
});
