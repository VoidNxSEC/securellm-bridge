{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.securellm-bridge.gateway;
  inherit (lib) mkEnableOption mkIf mkOption types;
  startScript = pkgs.writeShellScript "securellm-gateway-start" ''
    set -euo pipefail
    export GATEWAY_GITHUB_PAT_FILE="''${CREDENTIALS_DIRECTORY}/gateway_github_pat"
    exec ${cfg.package}/bin/gateway-mcp
  '';
in
{
  options.services.securellm-bridge.gateway = {
    enable = mkEnableOption "SecureLLM Gateway MCP HTTP service";

    package = mkOption {
      type = types.package;
      description = "Package that provides bin/gateway-mcp";
    };

    user = mkOption {
      type = types.str;
      default = "securellm-gateway";
      description = "User account under which the gateway runs";
    };

    group = mkOption {
      type = types.str;
      default = "securellm-gateway";
      description = "Group under which the gateway runs";
    };

    repoAllowlist = mkOption {
      type = types.listOf types.str;
      default = [ ];
      example = [
        "VoidNxSEC/securellm-bridge"
        "VoidNxSEC/adr-ledger"
      ];
      description = "GitHub repositories the gateway may mutate, as owner/name slugs";
    };

    agentId = mkOption {
      type = types.str;
      default = "securellm-gateway";
      description = "Authoritative gateway agent ID recorded in audit events";
    };

    listenAddr = mkOption {
      type = types.str;
      default = "127.0.0.1:8765";
      description = "HTTP listen address for the MCP gateway";
    };

    logDir = mkOption {
      type = types.str;
      default = "/var/lib/securellm-gateway";
      description = "Directory where events.jsonl audit logs are written";
    };

    githubPatFile = mkOption {
      type = types.str;
      default = "/run/secrets/gateway_github_pat";
      description = "Runtime path containing the GitHub PAT, usually managed by SOPS";
    };

    rateLimitPerMinute = mkOption {
      type = types.ints.positive;
      default = 10;
      description = "Per-agent HTTP request quota for /mcp";
    };

    extraEnvironment = mkOption {
      type = types.attrsOf types.str;
      default = { };
      description = "Additional environment variables for gateway-mcp";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.repoAllowlist != [ ];
        message = "services.securellm-bridge.gateway.repoAllowlist must not be empty";
      }
      {
        assertion = cfg.agentId != "";
        message = "services.securellm-bridge.gateway.agentId must not be empty";
      }
    ];

    users.users.${cfg.user} = {
      isSystemUser = true;
      group = cfg.group;
      home = cfg.logDir;
      createHome = true;
      description = "SecureLLM Gateway service user";
    };

    users.groups.${cfg.group} = { };

    systemd.tmpfiles.rules = [
      "d ${cfg.logDir} 0750 ${cfg.user} ${cfg.group} - -"
    ];

    systemd.services.securellm-gateway = {
      description = "SecureLLM Gateway MCP HTTP service";
      wantedBy = [ "multi-user.target" ];
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      path = [ pkgs.git ];

      environment =
        {
          GATEWAY_TRANSPORT = "http";
          GATEWAY_LISTEN_ADDR = cfg.listenAddr;
          GATEWAY_REPO_ALLOWLIST = lib.concatStringsSep "," cfg.repoAllowlist;
          GATEWAY_AGENT_ID = cfg.agentId;
          GATEWAY_LOG_DIR = cfg.logDir;
          GATEWAY_RATE_LIMIT_PER_MINUTE = toString cfg.rateLimitPerMinute;
          RUST_LOG = "info";
        }
        // cfg.extraEnvironment;

      serviceConfig = {
        Type = "simple";
        ExecStart = "${startScript}";
        LoadCredential = [ "gateway_github_pat:${cfg.githubPatFile}" ];
        Restart = "on-failure";
        RestartSec = "5s";
        User = cfg.user;
        Group = cfg.group;
        WorkingDirectory = cfg.logDir;

        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadWritePaths = [ cfg.logDir ];
        RestrictAddressFamilies = [
          "AF_INET"
          "AF_INET6"
          "AF_UNIX"
        ];
        LockPersonality = true;
        MemoryDenyWriteExecute = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        SystemCallArchitectures = "native";
      };
    };
  };
}
