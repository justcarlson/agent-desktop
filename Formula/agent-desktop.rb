class AgentDesktop < Formula
  desc "Native accessibility-based desktop automation for agents"
  homepage "https://github.com/lahfir/agent-desktop"
  version "0.10.1"
  license "Apache-2.0"

  depends_on macos: :ventura

  on_arm do
    url "https://github.com/lahfir/agent-desktop/releases/download/v0.10.1/agent-desktop-v0.10.1-aarch64-apple-darwin.tar.gz"
    sha256 "3f1c9fbc5d31d96881bed0e741360cac5bbd2ae2c1add08643a05cfb3429244b"
  end

  on_intel do
    url "https://github.com/lahfir/agent-desktop/releases/download/v0.10.1/agent-desktop-v0.10.1-x86_64-apple-darwin.tar.gz"
    sha256 "55e82cfc22cc9f1e7313632a3cd1320f23614b4c7df310f989f5c053af5bcca3"
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
