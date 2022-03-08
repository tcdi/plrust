{
  description = "A pl/Rust extension for PostgreSQ.";

  inputs = {
    nixpkgs.url = "nixpkgs";
    rust-overlay.url = "github:oxalica/rust-overlay";
    rust-overlay.inputs.nixpkgs.follows = "nixpkgs";
    naersk.url = "github:nix-community/naersk";
    naersk.inputs.nixpkgs.follows = "nixpkgs";
    gitignore.url = "github:hercules-ci/gitignore.nix";
    gitignore.inputs.nixpkgs.follows = "nixpkgs";
    pgx.url = "github:zombodb/pgx/nix-non-singlestep";
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
        pkgs."${cargoToml.package.name}");

      packages = pgx.lib.forAllSystems (system:
        let
          pkgs = pgx.lib.nixpkgsWithOverlays { inherit system nixpkgs; extraOverlays = [ self.overlay ]; };
        in {
          "${cargoToml.package.name}_10" = pkgs."${cargoToml.package.name}_10";
          "${cargoToml.package.name}_10_debug" = pkgs."${cargoToml.package.name}_10_debug";
          "${cargoToml.package.name}_11" = pkgs."${cargoToml.package.name}_11";
          "${cargoToml.package.name}_11_debug" = pkgs."${cargoToml.package.name}_11_debug";
          "${cargoToml.package.name}_12" = pkgs."${cargoToml.package.name}_12";
          "${cargoToml.package.name}_12_debug" = pkgs."${cargoToml.package.name}_12_debug";
          "${cargoToml.package.name}_13" = pkgs."${cargoToml.package.name}_13";
          "${cargoToml.package.name}_13_debug" = pkgs."${cargoToml.package.name}_13_debug";
          "${cargoToml.package.name}_14" = pkgs."${cargoToml.package.name}_14";
          "${cargoToml.package.name}_14_debug" = pkgs."${cargoToml.package.name}_14_debug";
        });

      overlay = final: prev: {
        "${cargoToml.package.name}_10" = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_10;
          additionalFeatures = [ "sandboxed" ];
        };
        "${cargoToml.package.name}_10_debug" = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_10;
          release = false;
          additionalFeatures = [ "sandboxed" ];
        };
        "${cargoToml.package.name}_11" = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_11;
          additionalFeatures = [ "sandboxed" ];
        };
        "${cargoToml.package.name}_11_debug" = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_11;
          release = false;
          additionalFeatures = [ "sandboxed" ];
        };
        "${cargoToml.package.name}_12" = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_12;
          additionalFeatures = [ "sandboxed" ];
        };
        "${cargoToml.package.name}_12_debug" = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_12;
          release = false;
          additionalFeatures = [ "sandboxed" ];
        };
        "${cargoToml.package.name}_13" = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_13;
          additionalFeatures = [ "sandboxed" ];
        };
        "${cargoToml.package.name}_13_debug" = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_13;
          release = false;
          additionalFeatures = [ "sandboxed" ];
        };
        "${cargoToml.package.name}_14" = pgx.lib.buildPgxExtension {
          pkgs = final;
          source = ./.;
          targetPostgres = final.postgresql_14;
          additionalFeatures = [ "sandboxed" ];
        };
        "${cargoToml.package.name}_14_debug" = pgx.lib.buildPgxExtension {
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
