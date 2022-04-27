{
  description = "A pl/Rust extension for PostgreSQ.";

  inputs = {
    nixpkgs.url = "nixpkgs";
    rust-overlay.url = "git+https://github.com/oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    naersk.url = "git+https://github.com/nix-community/naersk";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    gitignore.url = "git+https://github.com/hercules-ci/gitignore.nix";
    gitignore.inputs.nixpkgs.follows = "nixpkgs";
    pgx.url = "git+https://github.com/zombodb/pgx";
    pgx.inputs.nixpkgs.follows = "nixpkgs";
    pgx.inputs.naersk.follows = "naersk";
  };

  outputs = { self, nixpkgs, rust-overlay, naersk, gitignore, pgx }:
    let
      cargoToml = (builtins.fromTOML (builtins.readFile ./Cargo.toml));
    in
    {
      inherit (pgx) devShell;

      defaultPackage = pgx.lib.forAllSystems (system:
        let
          pkgs = pgx.lib.nixpkgsWithOverlays { inherit system nixpkgs; extraOverlays = [ self.overlay ]; };
        in
        pkgs.plrust_11);

      packages = pgx.lib.forAllSystems (system:
        let
          pkgs = pgx.lib.nixpkgsWithOverlays { inherit system nixpkgs; extraOverlays = [ self.overlay ]; };
        in
        {
          plrust_10 = pkgs.plrust_10;
          plrust_10_debug = pkgs.plrust_10_debug;
          plrust_11 = pkgs.plrust_11;
          plrust_11_debug = pkgs.plrust_11_debug;
          plrust_12 = pkgs.plrust_12;
          plrust_12_debug = pkgs.plrust_12_debug;
          plrust_13 = pkgs.plrust_13;
          plrust_13_debug = pkgs.plrust_13_debug;
          plrust_14 = pkgs.plrust_14;
          plrust_14_debug = pkgs.plrust_14_debug;
        });

      overlay = final: prev: {
        plrust_10 = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_10;
          additionalFeatures = [ "sandboxed" ];
        };
        plrust_10_debug = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_10;
          release = false;
          additionalFeatures = [ "sandboxed" ];
        };
        plrust_11 = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_11;
          additionalFeatures = [ "sandboxed" ];
        };
        plrust_11_debug = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_11;
          release = false;
          additionalFeatures = [ "sandboxed" ];
        };
        plrust_12 = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_12;
          additionalFeatures = [ "sandboxed" ];
        };
        plrust_12_debug = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_12;
          release = false;
          additionalFeatures = [ "sandboxed" ];
        };
        plrust_13 = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_13;
          additionalFeatures = [ "sandboxed" ];
        };
        plrust_13_debug = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_13;
          release = false;
          additionalFeatures = [ "sandboxed" ];
        };
        plrust_14 = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_14;
          additionalFeatures = [ "sandboxed" ];
        };
        plrust_14_debug = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_14;
          release = false;
          additionalFeatures = [ "sandboxed" ];
        };

      };

      nixosModule = { config, pkgs, lib, ... }:
        let
          cfg = config.services.postgresql.plrust;
        in
        with lib;
        {
          options = {
            services.postgresql.plrust.enable = mkEnableOption "Enable pl/Rust.";
            services.postgresql.plrust.workDir = mkOption {
              type = types.str;
              description = "The `plrust.work_dir` setting.";
              default = "";
            };
          };
          config = mkIf cfg.enable {
            nixpkgs.overlays = [ self.overlay pgx.overlay ];
            services.postgresql.extraPlugins = with pkgs; [
              plrust
            ];
            services.postgresql.settings = {
              "plrust.work_dir" = assert (assertMsg (cfg.workDir != "") "workDir must exist if enabled.");  cfg.workDir;
              "plrust.pg_config" = with pkgs; "${postgresql}/bin/pg_config";
            };
          };
        };

      checks = pgx.lib.forAllSystems (system:
        let
          pkgs = pgx.lib.nixpkgsWithOverlays { inherit system nixpkgs; extraOverlays = [ self.overlay ]; };
        in
        {
          format = pkgs.runCommand "check-format"
            {
              buildInputs = with pkgs; [ rustfmt cargo ];
            } ''
            ${pkgs.rustfmt}/bin/cargo-fmt fmt --manifest-path ${./.}/Cargo.toml -- --check
            ${pkgs.nixpkgs-fmt}/bin/nixpkgs-fmt --check ${./.}
            touch $out # it worked!
          '';
          # audit = pkgs.runCommand "audit" { } ''
          #   HOME=$out
          #   ${pkgs.cargo-audit}/bin/cargo-audit audit --no-fetch
          #   # it worked!
          # '';
        });
    };
}
