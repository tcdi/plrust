{
  description = "A pl/Rust extension for PostgreSQL.";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    pgx.url = "github:zombodb/pgx/develop";
  };


  outputs = { self, nixpkgs, pgx }: 
    let
      supportedSystems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs supportedSystems (system: f system);
    in {
      inherit (pgx) devShell checks;
    };
}
