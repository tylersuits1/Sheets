import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

type TerminalApp = "ghostty" | "kitty" | "alacritty";
type Period = "day" | "night";
type ThemeVariant = "dark" | "light" | "both";
type ThemeSource = "built_in" | "user_installed";

interface AppInfo {
  app: TerminalApp;
  installed: boolean;
  config_path: string | null;
}

interface CurrentConfig {
  font_family: string | null;
  font_size: number | null;
  opacity: number | null;
}

interface Palette {
  background: string;
  foreground: string;
  cursor: string | null;
  selection_background: string | null;
  selection_foreground: string | null;
  ansi: string[];
}

interface Theme {
  id: string;
  name: string;
  variant: ThemeVariant;
  source: ThemeSource;
  git_url: string | null;
  palette: Palette;
}

interface DayNightThemes {
  day: Theme | null;
  night: Theme | null;
}

const APP_LABELS: Record<TerminalApp, string> = {
  ghostty: "Ghostty",
  kitty: "Kitty",
  alacritty: "Alacritty",
};

let apps: AppInfo[] = [];
let selectedApp: TerminalApp = "ghostty";
let currentTheme: Theme | null = null;
let periodFilter: "both" | Period = "both";
let lastThemes: Theme[] = [];

const appTabsEl = document.querySelector<HTMLElement>("#app-tabs")!;
const statusPanelEl = document.querySelector<HTMLElement>("#status-panel")!;
const periodFilterEl = document.querySelector<HTMLElement>("#period-filter")!;
const themeSelectEl = document.querySelector<HTMLSelectElement>("#theme-select")!;
const themeApplyBtn = document.querySelector<HTMLButtonElement>("#theme-apply-btn")!;
const themeRemoveBtn = document.querySelector<HTMLButtonElement>("#theme-remove-btn")!;
const themeDetailsEl = document.querySelector<HTMLElement>("#theme-details")!;
const previewWindowEl = document.querySelector<HTMLElement>("#preview-window")!;
const previewBodyEl = document.querySelector<HTMLElement>("#preview-body")!;
const fontForm = document.querySelector<HTMLFormElement>("#font-form")!;
const fontFamilySelect = document.querySelector<HTMLSelectElement>("#font-family")!;
const fontSizeInput = document.querySelector<HTMLInputElement>("#font-size")!;
const fontStatusEl = document.querySelector<HTMLElement>("#font-status")!;
const opacitySlider = document.querySelector<HTMLInputElement>("#opacity-slider")!;
const opacityValueEl = document.querySelector<HTMLElement>("#opacity-value")!;
const opacityApplyBtn = document.querySelector<HTMLButtonElement>("#opacity-apply")!;
const opacityStatusEl = document.querySelector<HTMLElement>("#opacity-status")!;

function setStatus(el: HTMLElement, message: string, kind: "success" | "error" | "" = "") {
  el.textContent = message;
  el.className = `status-message${kind ? ` ${kind}` : ""}`;
}

function errorMessage(err: unknown): string {
  return typeof err === "string" ? err : err instanceof Error ? err.message : String(err);
}

async function loadApps() {
  apps = await invoke<AppInfo[]>("list_apps");
  renderAppTabs();
}

async function loadFontFamilies() {
  const families = await invoke<string[]>("list_font_families");
  fontFamilySelect.innerHTML = "";
  for (const family of families) {
    const opt = document.createElement("option");
    opt.value = family;
    opt.textContent = family;
    fontFamilySelect.appendChild(opt);
  }
}

// Selects `name` in the font dropdown, adding it as an extra option first
// if it isn't in the installed-fonts list (e.g. a font the config already
// references that isn't installed here anymore).
function setFontFamilySelection(name: string) {
  const exists = Array.from(fontFamilySelect.options).some((o) => o.value === name);
  if (!exists) {
    const opt = document.createElement("option");
    opt.value = name;
    opt.textContent = `${name} (not installed)`;
    fontFamilySelect.prepend(opt);
  }
  fontFamilySelect.value = name;
}

function renderAppTabs() {
  appTabsEl.innerHTML = "";
  for (const info of apps) {
    const tab = document.createElement("button");
    tab.type = "button";
    tab.className = `app-tab${info.app === selectedApp ? " selected" : ""}`;

    const dot = document.createElement("span");
    dot.className = `dot${info.installed ? " installed" : ""}`;
    tab.appendChild(dot);

    const label = document.createElement("span");
    label.textContent = APP_LABELS[info.app];
    tab.appendChild(label);

    tab.addEventListener("click", () => selectApp(info.app));
    appTabsEl.appendChild(tab);
  }
}

async function selectApp(app: TerminalApp) {
  selectedApp = app;
  renderAppTabs();
  await refreshStatus();
}

async function refreshStatus() {
  statusPanelEl.innerHTML = "<p>Loading…</p>";
  try {
    const [config, theme, undoable, dayNight] = await Promise.all([
      invoke<CurrentConfig>("get_current_config", { app: selectedApp }),
      invoke<Theme | null>("get_current_theme", { app: selectedApp }),
      invoke<boolean>("can_undo", { app: selectedApp }),
      invoke<DayNightThemes>("get_day_night_themes", { app: selectedApp }),
    ]);
    currentTheme = theme;
    renderStatus(config, undoable, dayNight);
    selectDefaultTheme();

    if (config.font_family) setFontFamilySelection(config.font_family);
    if (config.font_size !== null) fontSizeInput.value = String(config.font_size);
    if (config.opacity !== null) {
      opacitySlider.value = String(config.opacity);
      updateOpacityLabel();
    }
  } catch (err) {
    statusPanelEl.innerHTML = "";
    const p = document.createElement("p");
    p.className = "status-message error";
    p.textContent = `Couldn't read ${APP_LABELS[selectedApp]}'s config: ${errorMessage(err)}`;
    statusPanelEl.appendChild(p);
  }
}

function renderStatus(config: CurrentConfig, undoable: boolean, dayNight: DayNightThemes) {
  const info = apps.find((a) => a.app === selectedApp);
  statusPanelEl.innerHTML = "";

  const heading = document.createElement("h2");
  heading.textContent = APP_LABELS[selectedApp];
  statusPanelEl.appendChild(heading);

  if (info && !info.installed) {
    const notice = document.createElement("p");
    notice.className = "status-message error";
    notice.textContent = `Couldn't find a config file for ${APP_LABELS[selectedApp]} at ${info.config_path ?? "the expected location"}.`;
    statusPanelEl.appendChild(notice);

    const locateBtn = document.createElement("button");
    locateBtn.type = "button";
    locateBtn.textContent = "Locate config file…";
    locateBtn.addEventListener("click", locateConfigFile);
    statusPanelEl.appendChild(locateBtn);
    return;
  }

  const dl = document.createElement("dl");
  dl.className = "status-grid";

  const rows: [string, string][] = [
    ["Config", info?.config_path ?? "unknown"],
    ["Day", dayNight.day ? dayNight.day.name : "not set"],
    ["Night", dayNight.night ? dayNight.night.name : "not set"],
    ["Font", config.font_family ? `${config.font_family} @ ${config.font_size ?? "?"}` : "not set"],
    ["Opacity", config.opacity !== null ? `${Math.round(config.opacity * 100)}%` : "not set"],
  ];
  for (const [term, value] of rows) {
    const dt = document.createElement("dt");
    dt.textContent = term;
    const dd = document.createElement("dd");
    dd.textContent = value;
    dl.append(dt, dd);
  }
  statusPanelEl.appendChild(dl);

  const undoBtn = document.createElement("button");
  undoBtn.type = "button";
  undoBtn.textContent = "Undo last change";
  undoBtn.disabled = !undoable;
  undoBtn.style.marginTop = "0.75rem";
  undoBtn.addEventListener("click", async () => {
    try {
      await invoke("undo_last_change", { app: selectedApp });
      await refreshStatus();
    } catch (err) {
      alert(`Couldn't undo: ${errorMessage(err)}`);
    }
  });
  statusPanelEl.appendChild(undoBtn);
}

async function locateConfigFile() {
  const path = await open({ multiple: false, title: `Locate ${APP_LABELS[selectedApp]}'s config file` });
  if (!path || Array.isArray(path)) return;
  try {
    await invoke("set_config_path", { app: selectedApp, path });
    await loadApps();
    await refreshStatus();
  } catch (err) {
    alert(`Couldn't use that file: ${errorMessage(err)}`);
  }
}

async function loadThemes() {
  try {
    const themes =
      periodFilter === "both"
        ? await invoke<Theme[]>("list_themes")
        : await invoke<Theme[]>("list_themes_for_period", { period: periodFilter });
    lastThemes = themes;
    renderThemeSelect();
  } catch (err) {
    themeSelectEl.innerHTML = "";
    themeDetailsEl.textContent = `Couldn't load themes: ${errorMessage(err)}`;
    themeDetailsEl.className = "status-message error";
  }
}

function renderThemeSelect() {
  themeDetailsEl.className = "status-message";
  themeSelectEl.innerHTML = "";
  for (const theme of lastThemes) {
    const opt = document.createElement("option");
    opt.value = theme.id;
    const userTag = theme.source === "user_installed" ? " (User)" : "";
    opt.textContent = `${theme.name}${userTag} — ${theme.variant}`;
    themeSelectEl.appendChild(opt);
  }
  selectDefaultTheme();
}

function selectDefaultTheme() {
  if (lastThemes.length === 0) return;
  const desired = lastThemes.find((t) => t.id === currentTheme?.id) ?? lastThemes[0];
  themeSelectEl.value = desired.id;
  onThemeSelectionChanged();
}

function selectedThemeInDropdown(): Theme | null {
  return lastThemes.find((t) => t.id === themeSelectEl.value) ?? null;
}

function onThemeSelectionChanged() {
  const theme = selectedThemeInDropdown();
  themeRemoveBtn.hidden = !theme || theme.source !== "user_installed";
  themeDetailsEl.textContent = "";
  themeDetailsEl.className = "status-message";
  renderPreview(theme);
}

const PERIOD_LABEL: Record<"both" | Period, string> = { both: "Both", day: "Day", night: "Night" };

// Applying designates the theme for whichever tab you were browsing when
// you hit Apply: Day/Night sets just that one, Both sets both at once.
async function applyTheme() {
  const theme = selectedThemeInDropdown();
  if (!theme) return;
  try {
    await invoke("apply_theme", { app: selectedApp, themeId: theme.id });
    if (periodFilter === "both" || periodFilter === "day") {
      await invoke("set_day_theme", { app: selectedApp, themeId: theme.id });
    }
    if (periodFilter === "both" || periodFilter === "night") {
      await invoke("set_night_theme", { app: selectedApp, themeId: theme.id });
    }
    await refreshStatus();
    themeDetailsEl.textContent = `"${theme.name}" applied to ${PERIOD_LABEL[periodFilter]}.`;
    themeDetailsEl.className = "status-message success";
  } catch (err) {
    alert(`Couldn't apply "${theme.name}": ${errorMessage(err)}`);
  }
}

async function removeSelectedTheme() {
  const theme = selectedThemeInDropdown();
  if (!theme) return;
  if (!confirm(`Remove "${theme.name}" from your installed themes?`)) return;
  try {
    await invoke("remove_user_theme", { themeId: theme.id });
    await loadThemes();
  } catch (err) {
    alert(`Couldn't remove "${theme.name}": ${errorMessage(err)}`);
  }
}

// A short "ghost related" snippet per the spec, syntax-highlighted using the
// selected theme's own ANSI palette so the preview shows real theme colors,
// not a fixed set of highlighter colors.
const PREVIEW_CODE = `// Ghosts don't do daily standups. They just haunt async.
fn haunt(house: &mut House) -> Result<Scream, Ghost> {
    if house.is_empty() {
        return Ok(Scream::Faint); // nobody home to spook
    }
    let boo = Ghost::new("Casper");
    boo.phase_through(&house.walls)?;
    Ok(Scream::BloodCurdling)
}`;

const KEYWORDS = new Set([
  "fn", "if", "else", "return", "let", "mut", "struct", "impl", "match", "for", "while",
]);

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

function renderPreview(theme: Theme | null) {
  if (!theme) {
    previewBodyEl.textContent = "Select a theme to preview it here.";
    return;
  }
  const { palette } = theme;
  previewWindowEl.style.background = `#${palette.background}`;
  previewWindowEl.style.color = `#${palette.foreground}`;

  previewBodyEl.innerHTML = "";
  for (const line of PREVIEW_CODE.split("\n")) {
    for (const token of tokenizeLine(line)) {
      const span = document.createElement("span");
      span.textContent = token.text;
      span.style.color = `#${colorFor(token.cls, palette)}`;
      previewBodyEl.appendChild(span);
    }
    previewBodyEl.appendChild(document.createTextNode("\n"));
  }
}

function updateOpacityLabel() {
  opacityValueEl.textContent = `${Math.round(Number(opacitySlider.value) * 100)}%`;
}

window.addEventListener("DOMContentLoaded", async () => {
  try {
    await loadApps();
    await loadThemes();
    await loadFontFamilies();
    await refreshStatus();
  } catch (err) {
    statusPanelEl.innerHTML = "";
    const p = document.createElement("p");
    p.className = "status-message error";
    p.textContent = `Sheets failed to start: ${errorMessage(err)}`;
    statusPanelEl.appendChild(p);
    return;
  }

  periodFilterEl.addEventListener("click", (e) => {
    const target = e.target as HTMLElement;
    const period = target.dataset.period as "both" | Period | undefined;
    if (!period) return;
    periodFilter = period;
    for (const btn of periodFilterEl.querySelectorAll("button")) {
      btn.classList.toggle("active", btn === target);
    }
    loadThemes();
  });

  themeSelectEl.addEventListener("change", onThemeSelectionChanged);
  themeApplyBtn.addEventListener("click", applyTheme);
  themeRemoveBtn.addEventListener("click", removeSelectedTheme);

  fontForm.addEventListener("submit", async (e) => {
    e.preventDefault();
    try {
      await invoke("apply_font", {
        app: selectedApp,
        font: { family: fontFamilySelect.value, size: parseFloat(fontSizeInput.value) },
      });
      await refreshStatus();
      setStatus(fontStatusEl, `Font set to ${fontFamilySelect.value} @ ${fontSizeInput.value}pt.`, "success");
    } catch (err) {
      setStatus(fontStatusEl, `Couldn't apply font: ${errorMessage(err)}`, "error");
    }
  });

  opacitySlider.addEventListener("input", () => {
    updateOpacityLabel();
    setStatus(opacityStatusEl, "", "");
  });
  opacityApplyBtn.addEventListener("click", async () => {
    try {
      const percent = Math.round(Number(opacitySlider.value) * 100);
      await invoke("apply_opacity", { app: selectedApp, opacity: parseFloat(opacitySlider.value) });
      await refreshStatus();
      setStatus(opacityStatusEl, `Opacity set to ${percent}%.`, "success");
    } catch (err) {
      setStatus(opacityStatusEl, `Couldn't apply opacity: ${errorMessage(err)}`, "error");
    }
  });
});
