# Homebrew formula for Amberjs. SHA256 values are filled on each GitHub Release.
# Install from a tap that copies this file, e.g. `brew install zh30/tap/amber`.
class Amber < Formula
  desc "JavaScript/TypeScript runtime built with Rust and V8"
  homepage "https://amberjs.com"
  version "1.16.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/zh30/amberjs/releases/download/v#{version}/amber-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    else
      url "https://github.com/zh30/amberjs/releases/download/v#{version}/amber-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/zh30/amberjs/releases/download/v#{version}/amber-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    else
      url "https://github.com/zh30/amberjs/releases/download/v#{version}/amber-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "0000000000000000000000000000000000000000000000000000000000000000"
    end
  end

  def install
    bin.install "amber"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/amber --version")
  end
end
