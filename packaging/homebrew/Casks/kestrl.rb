cask "kestrl" do
  version "VERSION"
  sha256 "SHA256"

  url "https://github.com/AdityaAmitK/kestrl/releases/download/v#{version}/Kestrl-#{version}.dmg"
  name "Kestrl"
  desc "Local-first workbench for backend development"
  homepage "https://kestrl.adityakinjawadekar.com"

  app "Kestrl.app"

  caveats <<~EOS
    Kestrl is ad-hoc signed and not notarized. On first launch, macOS may block it.
    Open System Settings > Privacy & Security and choose Open Anyway, or remove
    the quarantine attribute yourself with:

      xattr -dr com.apple.quarantine /Applications/Kestrl.app
  EOS
end
