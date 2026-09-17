{
  config,
  lib,
  pkgs,
  ...
}: let
  app = "turbo-guacamole";
  cfg = config.services.${app};
in {
  options.services.${app} = {
    enable = lib.mkEnableOption "turbo-guacamole";
    package = lib.mkOption {
      type = lib.types.path;
      description = "The turbo-guacamole package to run.";
    };
    environmentFile = lib.mkOption {
      type = lib.types.nullOr lib.types.path;
      default = null;
      description = ''
        File of KEY=VALUE lines. Mainly used for DATABASE_URL/CACHE_URL when
        using external databases, or to override the local-mode values
        (e.g. /run/secrets/tg.env, chmod 600).
      '';
    };
    database = lib.mkOption {
      type = lib.types.submodule {
        options = {
          local = lib.mkEnableOption "local PostgreSQL and Redis instances";
          postgresUrl = lib.mkOption {
            type = lib.types.str;
            default = "";
            description = "External PostgreSQL URL (required when local is disabled).";
          };
          redisUrl = lib.mkOption {
            type = lib.types.str;
            default = "";
            description = "External Redis URL (required when local is disabled).";
          };
        };
      };
      default = {local = true;};
    };
    host = lib.mkOption {
      type = lib.types.str;
      default = "127.0.0.1";
    };
    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
    };
  };

  config = let
    dbUrl =
      if cfg.database.local
      then "postgresql:///${app}?host=/run/postgresql&user=${app}"
      else cfg.database.postgresUrl;
    cacheUrl =
      if cfg.database.local
      then "redis://localhost:6379"
      else cfg.database.redisUrl;
  in {
    assertions = [
      {
        assertion =
          cfg.database.local
          || cfg.environmentFile != null
          || (cfg.database.postgresUrl != "" && cfg.database.redisUrl != "");
        message = "services.turbo-guacamole: with database.local disabled, set an environmentFile with DATABASE_URL/CACHE_URL or provide database.postgresUrl and database.redisUrl.";
      }
    ];

    users.groups.${app} = {};
    users.users.${app} = {
      isSystemUser = true;
      group = app;
      description = "Turbo Guacamole service";
    };

    services.postgresql = lib.mkIf cfg.database.local {
      enable = true;
      ensureDatabases = [app];
      ensureUsers = [
        {
          name = app;
          ensureDBOwnership = true;
        }
      ];
    };
    services.redis.servers.${app} = lib.mkIf cfg.database.local {
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
        ++ lib.optionals cfg.database.local ["postgresql.service" "postgresql-setup.service" "redis-${app}.service"];
      requires = lib.optionals cfg.database.local ["postgresql.service" "postgresql-setup.service" "redis-${app}.service"];
      serviceConfig =
        {
          ExecStart = "${cfg.package}/bin/${app}";
          Environment = [
            "SERVICE_HOST=${cfg.host}"
            "SERVICE_PORT=${toString cfg.port}"
            "DATABASE_URL=${dbUrl}"
            "CACHE_URL=${cacheUrl}"
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

    systemd.services."${app}-schema" = lib.mkIf cfg.database.local {
      description = "Apply the turbo-guacamole database schema";
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
        ${pkgs.postgresql}/bin/psql \
          -h /run/postgresql -d ${app} -U ${app} \
          -v ON_ERROR_STOP=1 \
          -f ${../sql/schema.sql}
      '';
    };
  };
}
