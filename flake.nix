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
      # rustc/cargo back the Cargo workspace (apps/hexend,
      # apps/presentation-rs, crates/*). crates/hexen-proto compiles
      # schema/hexen/v1/*.proto with protox (pure Rust), so protoc isn't
      # needed for that; protobuf + buf stay for linting and for
      # packages/hexen-proto-ts.
      #
      # apps/presentation-rs is wasm32: nixpkgs' rustc already ships the
      # wasm32-unknown-unknown std and links it with plain `lld`.
      # wasm-bindgen-cli must match the crate's `wasm-bindgen = "=0.2.127"`
      # pin exactly — bump both together — and has to come from here,
      # since the prebuilt binary trunk would otherwise download won't run
      # on NixOS.
      #
      # apps/desktop (Tauri 2) links WebKitGTK; the GSettings schemas and
      # GIO modules below are what its native file dialog and the
      # webview's networking need at runtime from a dev shell. This only
      # makes the shell able to build and run it — the app isn't packaged
      # as a Nix derivation (`cargo tauri build` makes .deb/.AppImage).
      devShells.${system}.default = pkgs.mkShell {
        buildInputs = [
          pkgs.bun
          pkgs.nodejs_22
          pkgs.rustc
          pkgs.cargo
          pkgs.lld
          pkgs.trunk
          pkgs.wasm-bindgen-cli_0_2_127
          pkgs.binaryen # wasm-opt, for `trunk build --release`
          pkgs.protobuf
          pkgs.buf
          # apps/desktop
          pkgs.pkg-config
          pkgs.cargo-tauri
          pkgs.webkitgtk_4_1
          pkgs.gtk3
          pkgs.libsoup_3
          pkgs.librsvg
          pkgs.openssl
          pkgs.glib-networking
          pkgs.gsettings-desktop-schemas
        ];
        shellHook = ''
          export XDG_DATA_DIRS="${pkgs.gsettings-desktop-schemas}/share/gsettings-schemas/${pkgs.gsettings-desktop-schemas.name}:${pkgs.gtk3}/share/gsettings-schemas/${pkgs.gtk3.name}''${XDG_DATA_DIRS:+:$XDG_DATA_DIRS}"
          export GIO_MODULE_DIR="${pkgs.glib-networking}/lib/gio/modules/"
        '';
        # Use the wasm-bindgen above rather than downloading one.
        TRUNK_TOOLS_WASM_BINDGEN = "0.2.127";
        TRUNK_OFFLINE = "true";
      };
    };
}
