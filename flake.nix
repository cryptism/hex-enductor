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
      # rustc/cargo back apps/hexend; protobuf (protoc) + buf drive
      # codegen from schema/hexen/v1/*.proto into both that crate and
      # packages/hexen-proto-ts.
      #
      # cargo-leptos backs the experimental apps/presentation-rs (Leptos
      # SSR skeleton, see its own comments). Turns out this nixpkgs'
      # pkgs.rustc already carries the wasm32-unknown-unknown std lib,
      # so no rustup/rust-overlay needed for the target itself — it
      # just needs `lld` as the linker for that target (cargo-leptos's
      # client-side build fails with "linker `lld` not found" without
      # it) and wasm-bindgen-cli, whose version must match the
      # `wasm-bindgen` crate version pinned in
      # apps/presentation-rs/Cargo.toml exactly.
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [ pkgs.bun pkgs.nodejs_22 pkgs.rustc pkgs.cargo pkgs.protobuf pkgs.buf pkgs.cargo-leptos pkgs.lld pkgs.wasm-bindgen-cli ];
      };
    };
}
