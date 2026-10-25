class Kestrl < Formula
  desc "Local-first workbench for backend development"
  homepage "https://kestrl.adityakinjawadekar.com"
  url "https://github.com/AdityaAmitK/kestrl/releases/download/vVERSION/kestrl-VERSION-macos-universal.tar.gz"
  sha256 "SHA256"
  license "MIT"

  def install
    bin.install "kestrl"
    doc.install "README.md", "LICENSE"
    pkgshare.install "kestrl-bpf.sh"
  end

  test do
    system bin/"kestrl", "--version"
  end
end
