# typed: false
# frozen_string_literal: true

class Ttyms < Formula
  desc "A secure terminal client for Microsoft Teams"
  homepage "https://github.com/davidkaya/ttyms"
  version "VERSION_PLACEHOLDER"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/davidkaya/ttyms/releases/download/v#{version}/ttyms-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "SHA256_ARM64_PLACEHOLDER"
    end

    on_intel do
      url "https://github.com/davidkaya/ttyms/releases/download/v#{version}/ttyms-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "SHA256_X86_64_PLACEHOLDER"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/davidkaya/ttyms/releases/download/v#{version}/ttyms-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "SHA256_LINUX_ARM64_PLACEHOLDER"
    end

    on_intel do
      url "https://github.com/davidkaya/ttyms/releases/download/v#{version}/ttyms-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "SHA256_LINUX_X86_64_PLACEHOLDER"
    end
  end

  def install
    bin.install "ttyms"
  end

  test do
    assert_match "ttyms", shell_output("#{bin}/ttyms --help")
  end
end
