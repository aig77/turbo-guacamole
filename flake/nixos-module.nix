{
  config,
  lib,
  ...
}: let
  inherit (lib) mkEnableOption mkOption types;
  app = "turbo-guacamole";
  cfg = config.services.${app};
in {
  options.services.${app} = {
    enable = mkEnableOption app;
    package = mkOption {
      type = types.path;
      description = "The Turbo Guacamole package to run.";
    };
    host = mkOption {
      type = types.str;
      default = "127.0.0.1";
      example = "0.0.0.0";
    };
    port = mkOption {
      type = types.port;
      default = 8080;
    };
    createDatabases = mkOption {
      type = types.bool;
      default = true;
      description = "Creates local PostgreSQL/Redis instances";
    };
    environmentFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = ''
        Optional KEY=VALUE file. Values override everything the module sets
        (systemd `EnvironmentFile=` takes precedence over `Environment=`).

        Required when `createDatabases = false`, in which case it must define
        `DATABASE_URL` and `CACHE_URL`. The file must be readable by the
        `turbo-guacamole` user at service start; if it is missing or
        unreadable, the service fails to start.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion =
          cfg.createDatabases
          || cfg.environmentFile != null;
        message = "services.${app}: with createDatabases = false, set an environmentFile with DATABASE_URL/CACHE_URL";
      }
    ];

    users.groups.${app} = {};
    users.users.${app} = {
      isSystemUser = true;
      group = app;
      description = "Turbo Guacamole service";
    };

    services.postgresql = lib.mkIf cfg.createDatabases {
      enable = true;
      ensureDatabases = [app];
      ensureUsers = [
        {
          name = app;
          ensureDBOwnership = true;
        }
      ];
    };
    services.redis.servers.${app} = lib.mkIf cfg.createDatabases {
      enable = true;
      port = 6379;
      appendOnly = true;
      settings = {
        maxmemory = "256mb";
        maxmemory-policy = "allkeys-lru";
      };
    };

    systemd.services.${app} = {
      description = "Turbo Guacamole";
      wantedBy = ["multi-user.target"];
      after =
        ["network.target"]
        ++ lib.optionals cfg.createDatabases ["postgresql.service" "postgresql-setup.service" "redis-${app}.service"];
      requires = lib.optionals cfg.createDatabases ["postgresql.service" "postgresql-setup.service" "redis-${app}.service"];
      serviceConfig =
        {
          ExecStart = "${cfg.package}/bin/${app}";
          Environment =
            [
              "SERVICE_HOST=${cfg.host}"
              "SERVICE_PORT=${toString cfg.port}"
            ]
            ++ lib.optionals cfg.createDatabases [
              "DATABASE_URL=postgresql:///${app}?host=/run/postgresql&user=${app}"
              "CACHE_URL=redis://localhost:${toString config.services.redis.servers.${app}.port}"
            ];
          WorkingDirectory = cfg.package;
          StateDirectory = "${app}";
          User = "${app}";
          Group = "${app}";
          Restart = "on-failure";
          RestartSec = "5s";
        }
        // lib.optionalAttrs (cfg.environmentFile != null) {
          EnvironmentFile = toString cfg.environmentFile;
        };
    };

    systemd.services."${app}-schema" = lib.mkIf cfg.createDatabases {
      description = "Apply the ${app} database schema";
      after = ["postgresql.service" "postgresql-setup.service"];
      wantedBy = ["${app}.service"];
      wants = ["postgresql.service" "postgresql-setup.service"];
      before = ["${app}.service"];
      serviceConfig = {
        Type = "oneshot";
        RemainAfterExit = true;
        User = app;
        Group = app;
      };
      script = ''
        ${config.services.postgresql.package}/bin/psql \
          -h /run/postgresql -d ${app} -U ${app} \
          -v ON_ERROR_STOP=1 \
          -f ${../sql/schema.sql}
      '';
    };
  };
}
