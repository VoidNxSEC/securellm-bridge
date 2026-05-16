{
  description = "SecureLLM Bridge - Unified LLM API with Security & MCP Server";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      rust-overlay,
      flake-utils,
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };

        rustToolchain = pkgs.rust-bin.stable.latest.default.override {
          extensions = [
            "rust-src"
            "rust-analyzer"
          ];
        };

        # Rust workspace build
        rustPackage = pkgs.rustPlatform.buildRustPackage {
          pname = "securellm-bridge";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            rustToolchain
            clang
            libclang.lib
          ];

          buildInputs = with pkgs; [
            openssl
            sqlite
            # Audio stack for voice agents
            alsa-lib
            pulseaudio
            espeak-ng
            speechd
          ];

          # Build all workspace members
          buildPhase = ''
            cargo build --release --workspace
          '';

          installPhase = ''
            mkdir -p $out/bin

            # The CLI crate generates a binary called "securellm"
            if [ -f target/release/securellm ]; then
              cp target/release/securellm $out/bin/
              # Create symlinks for convenience
              ln -s securellm $out/bin/securellm-bridge
              ln -s securellm $out/bin/securellm-cli
            fi

            # Copy api-server if it exists

            # Copy cgroup-helper if it exists (ADR-0001 Phase 3)
            if [ -f target/release/cgroup-helper ]; then
              cp target/release/cgroup-helper $out/bin/
            fi
            if [ -f target/release/securellm-api-server ]; then
              cp target/release/securellm-api-server $out/bin/
            fi
          '';

          meta = with pkgs.lib; {
            description = "Secure LLM API proxy with enterprise-grade security";
            homepage = "https://github.com/marcosfpina/securellm-bridge";
            license = with licenses; [
              mit
              asl20
            ];
            maintainers = [ "kernelcore" ];
          };
        };

        gatewayPackage = pkgs.rustPlatform.buildRustPackage {
          pname = "securellm-gateway";
          version = "0.1.0";
          src = ./.;

          cargoLock = {
            lockFile = ./Cargo.lock;
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            makeWrapper
            rustToolchain
            clang
            libclang.lib
          ];

          buildInputs = with pkgs; [
            sqlite
          ];

          cargoBuildFlags = [
            "-p"
            "securellm-gateway"
            "--bin"
            "gateway-mcp"
          ];

          doCheck = false;

          installPhase = ''
            mkdir -p $out/bin
            cp target/release/gateway-mcp $out/bin/
            wrapProgram $out/bin/gateway-mcp \
              --prefix PATH : ${pkgs.lib.makeBinPath [ pkgs.git ]}
          '';

          meta = with pkgs.lib; {
            description = "SecureLLM Gateway MCP server";
            homepage = "https://github.com/marcosfpina/securellm-bridge";
            license = with licenses; [
              mit
              asl20
            ];
            maintainers = [ "kernelcore" ];
          };
        };

      in
      {
        packages = {
          default = rustPackage;
          rust = rustPackage;
          gateway = gatewayPackage;
          mcp = gatewayPackage;

          # Combined package with both Rust and MCP
          all = pkgs.symlinkJoin {
            name = "securellm-bridge-all";
            paths = [
              rustPackage
              gatewayPackage
            ];
          };
        };

        apps = {
          default = {
            type = "app";
            program = "${rustPackage}/bin/securellm";
          };

          bridge = {
            type = "app";
            program = "${rustPackage}/bin/securellm-bridge";
          };

          mcp = {
            type = "app";
            program = "${gatewayPackage}/bin/gateway-mcp";
          };

          gateway = {
            type = "app";
            program = "${gatewayPackage}/bin/gateway-mcp";
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            # Rust toolchain
            rustToolchain
            cargo-watch
            cargo-edit

            # Compilation tools
            clang
            libclang.lib

            # Node.js for MCP server
            nodejs_24
            typescript

            # Build dependencies
            pkg-config
            openssl
            sqlite

            # Runtime dependencies
            redis

            # Audio dependencies for voice agents
            alsa-lib
            pulseaudio
            espeak-ng
            speechd

            # Development tools
            git
            ripgrep
            fd
            sops
            age
          ];

          shellHook = ''
            export LIBCLANG_PATH="${pkgs.libclang.lib}/lib"
            export LD_LIBRARY_PATH="${pkgs.stdenv.cc.cc.lib}/lib:$LD_LIBRARY_PATH"

            # SecureLLM Bridge Environment Variables
            export DATABASE_URL="sqlite:$PWD/data/models.db"
            export REDIS_URL="redis://localhost:6379"
            export LOG_DIR="$PWD/logs"
            export SERVER_HOST="0.0.0.0"
            export SERVER_PORT="8080"

            # Provider Configuration
            export LLAMACPP_ENABLED=true
            export LLAMACPP_BASE_URL="http://localhost:8081"
            export LLAMACPP_MODEL_NAME="local-model"

            export DEEPSEEK_ENABLED="''${DEEPSEEK_ENABLED:-false}"
            export OPENAI_ENABLED="''${OPENAI_ENABLED:-false}"
            export ANTHROPIC_ENABLED="''${ANTHROPIC_ENABLED:-false}"
            export GROQ_ENABLED="''${GROQ_ENABLED:-false}"
            export GEMINI_ENABLED="''${GEMINI_ENABLED:-false}"
            export NVIDIA_ENABLED="''${NVIDIA_ENABLED:-false}"

            # Security
            export REQUIRE_AUTH=false
            export LOG_LEVEL=info

            # Create necessary directories
            mkdir -p data logs

            echo "🦀 SecureLLM Bridge Development Environment"
            echo "  Rust: $(rustc --version)"
            echo "  Node: $(node --version)"
            echo ""
            echo "📊 Configuration:"
            echo "  - Database: $DATABASE_URL"
            echo "  - Redis: $REDIS_URL"
            echo "  - Server: $SERVER_HOST:$SERVER_PORT"
            echo "  - LlamaCpp (llama-swap): $LLAMACPP_BASE_URL"
            echo ""
            echo "Commands:"
            echo "  cargo run --bin securellm-api-server  - Start API server"
            echo "  cargo run -p securellm-gateway --bin gateway-mcp  - Start Gateway MCP"
            echo "  cargo build         - Build Rust workspace"
            echo "  cargo test          - Run Rust tests"
            echo "  nix build .#rust    - Build Rust package"
            echo "  nix build .#gateway - Build Gateway MCP server"
            echo "  nix build .#all     - Build both"
          '';
        };

        # Checks for CI/CD
        checks = {
          rust-build = rustPackage;
          gateway-build = gatewayPackage;
        };
      }
    ) // {
      # NixOS Module: securellm-bridge sandbox (ADR-0001 Phase 3)
      nixosModules.sandbox = import ./nix/modules/sandbox.nix;
      nixosModules.gateway-service = import ./nix/modules/gateway-service.nix;
    };
}
