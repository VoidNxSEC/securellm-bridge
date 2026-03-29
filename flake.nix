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

        # MCP Server (TypeScript/Node.js)
        mcpServer = pkgs.buildNpmPackage {
          pname = "securellm-bridge-mcp";
          version = "2.0.0";
          src = ./mcp-server;

          npmDepsHash = "sha256-u0xDEW8vlMcyJtnMEPuVDhJv/piK6lUHKPlkAU5H6+8=";

          nativeBuildInputs = with pkgs; [
            nodejs
            python3
            pkg-config
          ];

          buildInputs = with pkgs; [
            sqlite
          ];

          buildPhase = ''
            npm run build
          '';

          installPhase = ''
            mkdir -p $out/bin $out/lib/mcp-server

            # Copy build output
            cp -r build $out/lib/mcp-server/
            cp package.json $out/lib/mcp-server/
            cp -r node_modules $out/lib/mcp-server/

            # Create executable wrapper
            cat > $out/bin/securellm-mcp <<EOF
            #!${pkgs.bash}/bin/bash
            exec ${pkgs.nodejs}/bin/node $out/lib/mcp-server/build/src/index.js "\$@"
            EOF
            chmod +x $out/bin/securellm-mcp
          '';

          meta = with pkgs.lib; {
            description = "MCP server for SecureLLM Bridge IDE integration";
            license = licenses.mit;
            maintainers = [ "marcosfpina" ];
          };
        };

      in
      {
        packages = {
          default = rustPackage;
          rust = rustPackage;
          mcp = mcpServer;

          # Combined package with both Rust and MCP
          all = pkgs.symlinkJoin {
            name = "securellm-bridge-all";
            paths = [
              rustPackage
              mcpServer
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
            program = "${mcpServer}/bin/securellm-mcp";
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
            nodejs
            nodePackages.typescript
            nodePackages.npm

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
            echo "  cargo build         - Build Rust workspace"
            echo "  cargo test          - Run Rust tests"
            echo "  cd mcp-server && npm run build  - Build MCP server"
            echo "  nix build .#rust    - Build Rust package"
            echo "  nix build .#mcp     - Build MCP server"
            echo "  nix build .#all     - Build both"
          '';
        };

        # Checks for CI/CD
        checks = {
          rust-build = rustPackage;
          mcp-build = mcpServer;
        };
      }
    );
}
