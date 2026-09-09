import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { save } from "@tauri-apps/plugin-dialog";

type ThemeVariant = "dark" | "light" | "both";

interface Theme {
  id: string;
  name: string;
  variant: ThemeVariant;
  source: "built_in" | "user_installed";
}

const themeSelect = document.querySelector<HTMLSelectElement>("#theme-select")!;
const statusEl = document.querySelector<HTMLElement>("#form-status")!;
const cancelBtn = document.querySelector<HTMLButtonElement>("#cancel-btn")!;
const exportBtn = document.querySelector<HTMLButtonElement>("#export-btn")!;

function errorMessage(err: unknown): string {
  return typeof err === "string" ? err : err instanceof Error ? err.message : String(err);
}

function setStatus(message: string, kind: "success" | "error" | "" = "") {
  statusEl.textContent = message;
  statusEl.className = `status-message${kind ? ` ${kind}` : ""}`;
}

async function loadThemes() {
  try {
    const themes = await invoke<Theme[]>("list_exportable_themes");
    themeSelect.innerHTML = "";
    if (themes.length === 0) {
      const opt = document.createElement("option");
      opt.textContent = "No user-created themes yet";
      opt.disabled = true;
      themeSelect.appendChild(opt);
      exportBtn.disabled = true;
      return;
    }
    for (const theme of themes) {
      const opt = document.createElement("option");
      opt.value = theme.id;
      opt.textContent = `${theme.name} (User)`;
      themeSelect.appendChild(opt);
    }
  } catch (err) {
    setStatus(`Couldn't load themes: ${errorMessage(err)}`, "error");
  }
}

cancelBtn.addEventListener("click", () => {
  getCurrentWindow().close();
});

exportBtn.addEventListener("click", async () => {
  const themeId = themeSelect.value;
  if (!themeId) return;

  const path = await save({
    defaultPath: `${themeId}.sheets-theme.json`,
    filters: [{ name: "Sheets theme", extensions: ["json"] }],
  });
  if (!path) return;

  try {
    await invoke("export_theme_to_path", { themeId, path });
    setStatus(`Saved to ${path}`, "success");
  } catch (err) {
    setStatus(errorMessage(err), "error");
  }
});

loadThemes();
