{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      # Bun for day-to-day dev (install, run, the dev servers); Node
      # stays available for whatever ends up running in production
      # (docs/PLAN.md §9/§7) without needing a second shell.
      #
      # rustc/cargo back apps/hexend-rs; protobuf (protoc) + buf drive
      # codegen from schema/hexen/v1/*.proto into both that crate and
      # packages/hexen-proto-ts.
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [ pkgs.bun pkgs.nodejs_22 pkgs.rustc pkgs.cargo pkgs.protobuf pkgs.buf ];
      };
    };
}
