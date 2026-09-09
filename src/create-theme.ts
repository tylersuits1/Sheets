import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

type ThemeVariant = "dark" | "light" | "both";

interface Palette {
  background: string;
  foreground: string;
  cursor: string | null;
  selection_background: string | null;
  selection_foreground: string | null;
  ansi: string[];
}

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
  });
  textInput.addEventListener("input", () => {
    if (/^[0-9a-fA-F]{6}$/.test(textInput.value)) {
      colorInput.value = `#${textInput.value}`;
    }
  });

  row.append(labelEl, colorInput, textInput);

  return {
    row,
    get: () => textInput.value.toLowerCase(),
    set: (hex: string) => {
      textInput.value = hex;
      colorInput.value = `#${hex}`;
    },
  };
}

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

  const palette: Palette = {
    background: background.get(),
    foreground: foreground.get(),
    cursor: cursor.get(),
    selection_background: selectionBackground.get(),
    selection_foreground: selectionForeground.get(),
    ansi: ansiFields.map((f) => f.get()),
  };

  try {
    await invoke("create_custom_theme", { name, variant: variantSelect.value as ThemeVariant, palette });
    getCurrentWindow().close();
  } catch (err) {
    statusEl.textContent = errorMessage(err);
    statusEl.className = "status-message error";
  }
});
