# Homebrew formula for funcspec
# To use: brew install narbs/funcspec/funcspec
# Tap repo: https://github.com/narbs/homebrew-funcspec
#
# This formula is a template; SHA256 values are updated automatically
# by the release workflow on each new version.
class Funcspec < Formula
  desc "Command-line interface for FuncSpec"
  homepage "https://funcspec.net"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/narbs/funcspec-cli/releases/download/v#{version}/funcspec-v#{version}-x86_64-apple-darwin.zip"
      sha256 "PLACEHOLDER_X86_64_DARWIN_SHA256"
    end

    on_arm do
      url "https://github.com/narbs/funcspec-cli/releases/download/v#{version}/funcspec-v#{version}-aarch64-apple-darwin.zip"
      sha256 "PLACEHOLDER_AARCH64_DARWIN_SHA256"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/narbs/funcspec-cli/releases/download/v#{version}/funcspec-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "PLACEHOLDER_X86_64_LINUX_SHA256"
    end

    on_arm do
      url "https://github.com/narbs/funcspec-cli/releases/download/v#{version}/funcspec-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER_AARCH64_LINUX_SHA256"
    end
  end

  def install
    bin.install "funcspec"
  end

  def caveats
    <<~EOS
      funcspec has been installed. Run `funcspec login` to authenticate with funcspec.net.
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/funcspec --version")
  end
end
