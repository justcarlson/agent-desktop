class AgentDesktop < Formula
  desc "Native accessibility-based desktop automation for agents"
  homepage "https://github.com/lahfir/agent-desktop"
  version "0.11.1"
  license "Apache-2.0"

  depends_on macos: :ventura

  on_arm do
    url "https://github.com/lahfir/agent-desktop/releases/download/v0.11.1/agent-desktop-v0.11.1-aarch64-apple-darwin.tar.gz"
    sha256 "666671bd944fb88bf5b5668589f2bd653cd3a01942035260b454736879ff8779"
  end

  on_intel do
    url "https://github.com/lahfir/agent-desktop/releases/download/v0.11.1/agent-desktop-v0.11.1-x86_64-apple-darwin.tar.gz"
    sha256 "32aedc30a787e8ec71ddec758128b30762a31a6ae9833099a5bc1288be302cf9"
  end

  def install
    bin.install "agent-desktop", "agent-desktop-macos-helper"
  end

  test do
    require "json"
    response = JSON.parse(shell_output("#{bin}/agent-desktop version"))
    assert_equal true, response.fetch("ok")
    assert_equal version.to_s, response.fetch("data").fetch("version")
    assert_equal "macos", response.fetch("data").fetch("os")
    assert_path_exists bin/"agent-desktop-macos-helper"
  end
end
