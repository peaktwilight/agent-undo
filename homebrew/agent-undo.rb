# typed: false
# frozen_string_literal: true

#
# Homebrew formula for agent-undo.
#
# To install via this formula, the user runs:
#
#   brew install peaktwilight/tap/agent-undo
#
# This file should live in `peaktwilight/homebrew-tap` at
# `Formula/agent-undo.rb`. To create the tap repo:
#
#   gh repo create peaktwilight/homebrew-tap --public --description "Homebrew tap for agent-undo"
#   git -C ~/code clone https://github.com/peaktwilight/homebrew-tap.git
#   mkdir -p ~/code/homebrew-tap/Formula
#   cp homebrew/agent-undo.rb ~/code/homebrew-tap/Formula/
#   git -C ~/code/homebrew-tap add Formula/agent-undo.rb
#   git -C ~/code/homebrew-tap commit -m "Add agent-undo formula"
#   git -C ~/code/homebrew-tap push origin main
#
# When you cut a new release of agent-undo:
#   1. Bump `version` below
#   2. Update the four `sha256` lines (the release workflow uploads
#      .tar.gz.sha256 sidecar files alongside each tarball — copy from there)
#   3. Commit + push the tap repo
#
# Or use `brew bump-formula-pr` to automate it.
#
class AgentUndo < Formula
  desc "Local-first rollback for AI coding agents — git for humans, au for agents"
  homepage "https://agent-undo.com"
  version "0.0.3"
  license "Apache-2.0"
  head "https://github.com/peaktwilight/agent-undo.git", branch: "main"

  on_macos do
    on_arm do
      url "https://github.com/peaktwilight/agent-undo/releases/download/v#{version}/agent-undo-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_FROM_RELEASE_TARBALL_aarch64_apple_darwin"
    end
    on_intel do
      url "https://github.com/peaktwilight/agent-undo/releases/download/v#{version}/agent-undo-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_SHA256_FROM_RELEASE_TARBALL_x86_64_apple_darwin"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/peaktwilight/agent-undo/releases/download/v#{version}/agent-undo-v#{version}-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256_FROM_RELEASE_TARBALL_aarch64_unknown_linux_gnu"
    end
    on_intel do
      url "https://github.com/peaktwilight/agent-undo/releases/download/v#{version}/agent-undo-v#{version}-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "REPLACE_WITH_SHA256_FROM_RELEASE_TARBALL_x86_64_unknown_linux_gnu"
    end
  end

  def install
    bin.install "au"
    doc.install "README.md", "PHILOSOPHY.md" if File.exist?("README.md")
  end

  def caveats
    <<~EOS
      agent-undo installs the `au` binary on your PATH.

      To set it up in a project:

        cd your-project
        au init --install-hooks
        au serve --daemon

      Type `au oops` when an AI coding agent destroys your work.

      The crate is `agent-undo` (descriptive, panic-searchable). The
      binary is `au` (daily-use, two letters). Same shape as ripgrep
      installing rg.
    EOS
  end

  test do
    assert_match "au #{version}", shell_output("#{bin}/au --version")
    system bin/"au", "--help"
  end
end
