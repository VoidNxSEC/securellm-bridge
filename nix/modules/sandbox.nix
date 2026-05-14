# NixOS Module: securellm-bridge sandbox
# Phase 3 of ADR-0001: NixOS-Integrated Delegation (Production)
#
# Provides systemd cgroup delegation for the SecureLLM Bridge sandbox,
# eliminating the need for a setuid helper binary in production.
#
# Usage (in configuration.nix):
#   services.securellm-bridge = {
#     enable = true;
#     sandbox = {
#       enableCgroups = true;
#       agentProfiles = {
#         llm-executor.memoryLimit = "512M";
#         build-agent.memoryLimit = "4G";
#       };
#     };
#   };

{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.securellm-bridge.sandbox;
  inherit (lib) mkEnableOption mkOption types;
in
{
  options.services.securellm-bridge.sandbox = {
    enableCgroups = mkEnableOption "Enable cgroup v2 resource limits for LLM sandboxes";

    socketPath = mkOption {
      type = types.str;
      default = "/run/securellm/cgroup.sock";
      description = "Unix Domain Socket path for cgroup-helper communication";
    };

    cgroupSubtree = mkOption {
      type = types.str;
      default = "/sys/fs/cgroup/securellm";
      description = "cgroup v2 subtree delegated to securellm-bridge";
    };

    agentProfiles = mkOption {
      type = types.attrsOf (
        types.submodule {
          options = {
            memoryLimit = mkOption {
              type = types.str;
              default = "512M";
              description = "Memory limit for the agent (e.g., 512M, 4G)";
            };
            cpuTimeSecs = mkOption {
              type = types.int;
              default = 30;
              description = "Maximum CPU time in seconds";
            };
            maxPids = mkOption {
              type = types.int;
              default = 16;
              description = "Maximum number of processes";
            };
            networkEnabled = mkOption {
              type = types.bool;
              default = false;
              description = "Whether the agent has network access";
            };
          };
        }
      );
      default = { };
      description = "Per-agent resource profiles";
    };

    cleanupOnExit = mkOption {
      type = types.bool;
      default = true;
      description = "Remove cgroup directories when the service stops";
    };
  };

  config = lib.mkIf cfg.enableCgroups {
    # Ensure cgroups v2 is available
    boot.kernelParams = [ "cgroup_no_v1=all" ];
    systemd.enableUnifiedCgroupHierarchy = true;

    # Create the cgroup subtree directory on boot
    systemd.tmpfiles.rules = [
      "d ${cfg.cgroupSubtree} 0755 securellm securellm - -"
    ];

    # systemd service with cgroup delegation (Tier 3: Production)
    systemd.services.securellm-cgroup-helper = {
      description = "SecureLLM Cgroup Helper — NEUTRON-audited sandbox resource management";
      after = [ "network.target" ];
      wantedBy = [ "multi-user.target" ];

      # Delegate cgroup subtree to this service
      serviceConfig = {
        Type = "simple";
        ExecStart = "${pkgs.securellm-bridge}/bin/cgroup-helper --socket-path ${cfg.socketPath}";
        Environment = [
          "CGROUP_SOCKET=${cfg.socketPath}"
          "RUST_LOG=info"
        ];

        # systemd cgroup delegation (ADR-0001 Tier 3)
        Delegate = "yes";
        MemoryAccounting = true;
        CPUAccounting = true;
        TasksAccounting = true;
        AllowedCPUs = "0-7"; # Restrict helper to first 8 cores
        MemoryMax = "64M"; # Helper itself uses minimal memory

        # Security hardening
        User = "securellm";
        Group = "securellm";
        AmbientCapabilities = [ "CAP_SYS_ADMIN" ]; # Only for cgroup writes
        CapabilityBoundingSet = [ "CAP_SYS_ADMIN" ];
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        ReadOnlyPaths = [ "/" ];
        ReadWritePaths = [
          "/sys/fs/cgroup"
          cfg.socketPath
        ];
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectKernelTunables = false; # Need to write to /sys/fs/cgroup
        ProtectControlGroups = false; # Need to manage cgroups
        RestrictAddressFamilies = [ "AF_UNIX" ];
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
          "~@resources"
        ];
        SystemCallArchitectures = "native";
        RestrictRealtime = true;
        MemoryDenyWriteExecute = true;
        LockPersonality = true;
      };

      # Create the socket directory
      preStart = ''
        mkdir -p $(dirname ${cfg.socketPath})
        chown securellm:securellm $(dirname ${cfg.socketPath})
        chmod 700 $(dirname ${cfg.socketPath})
      '';

      # Cleanup cgroups on stop
      postStop = lib.mkIf cfg.cleanupOnExit ''
        if [ -d ${cfg.cgroupSubtree} ]; then
          find ${cfg.cgroupSubtree} -mindepth 1 -maxdepth 1 -type d -exec rmdir {} \; 2>/dev/null || true
        fi
      '';
    };

    # Create securellm user/group if they don't exist
    users.users.securellm = {
      isSystemUser = true;
      group = "securellm";
      description = "SecureLLM Bridge sandbox user";
    };
    users.groups.securellm = { };

    # Agent profile validation (build-time check)
    assertions = lib.mapAttrsToList (name: profile: {
      assertion = profile.maxPids > 0 && profile.maxPids <= 1024;
      message = "Agent profile ${name}: maxPids must be between 1 and 1024";
    }) cfg.agentProfiles;
  };
}
