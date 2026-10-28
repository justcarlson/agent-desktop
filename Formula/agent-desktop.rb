class AgentDesktop < Formula
  desc "Native accessibility-based desktop automation for agents"
  homepage "https://github.com/lahfir/agent-desktop"
  version "0.11.1"
  license "Apache-2.0"

  on_macos do
    depends_on macos: :ventura

    on_arm do
      url "https://github.com/lahfir/agent-desktop/releases/download/v0.11.1/agent-desktop-v0.11.1-aarch64-apple-darwin.tar.gz"
      sha256 "666671bd944fb88bf5b5668589f2bd653cd3a01942035260b454736879ff8779"
    end

    on_intel do
      url "https://github.com/lahfir/agent-desktop/releases/download/v0.11.1/agent-desktop-v0.11.1-x86_64-apple-darwin.tar.gz"
      sha256 "32aedc30a787e8ec71ddec758128b30762a31a6ae9833099a5bc1288be302cf9"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/lahfir/agent-desktop/releases/download/v0.11.1/agent-desktop-v0.11.1-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "cdf02e3b3b08f812af28ea477fee9bd72f78fe87a9888768a0750d60bb4dd509"
    end

    on_intel do
      url "https://github.com/lahfir/agent-desktop/releases/download/v0.11.1/agent-desktop-v0.11.1-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "89674d626b3e15536e8084ac883d2507ca2f67d187f7e0b138305f371fb25138"
    end
  end

  def install
    bin.install "agent-desktop"
    bin.install "agent-desktop-macos-helper" if OS.mac?
    bin.install "agent-desktop-linux-clipboard-owner" if OS.linux?
  end

  test do
    require "json"
    response = JSON.parse(shell_output("#{bin}/agent-desktop version"))
    assert_equal true, response.fetch("ok")
    assert_equal version.to_s, response.fetch("data").fetch("version")
    assert_equal OS.mac? ? "macos" : "linux", response.fetch("data").fetch("os")
    assert_path_exists bin/(OS.mac? ? "agent-desktop-macos-helper" : "agent-desktop-linux-clipboard-owner")
  end
end
