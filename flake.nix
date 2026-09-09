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
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [ pkgs.bun pkgs.nodejs_22 ];
      };
    };
}
