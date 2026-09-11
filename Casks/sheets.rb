cask "sheets" do
  version "0.1.0"
  sha256 "1ed331fc8f3315bfa4f0674a01d7639fb902bd040461518c52e2b22d7dae53a9"

  url "https://github.com/tylersuits1/Sheets/releases/download/v#{version}/Sheets_#{version}_aarch64.dmg"
  name "Sheets"
  desc "Manage terminal themes, fonts, and opacity across Ghostty, Kitty, and Alacritty"
  homepage "https://github.com/tylersuits1/Sheets"

  depends_on arch: :arm64
  depends_on :macos

  app "Sheets.app"

  zap trash: "~/.config/sheets"
end
