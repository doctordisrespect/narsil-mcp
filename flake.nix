{
  description = "narsil-mcp - A blazingly fast MCP server for code intelligence";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    pleme-nix = {
      url = "github:pleme-io/pleme-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, flake-utils, pleme-nix }:
    flake-utils.lib.eachDefaultSystem (system: let
      # Get the Rust overlay from pleme-nix (per-system outputs)
      rustOverlay = pleme-nix.overlays.${system}.rust;

      # Apply the overlay to nixpkgs
      pkgs = import nixpkgs {
        inherit system;
        overlays = [ rustOverlay ];
      };

      # Get the pleme-nix library (per-system)
      nixLib = pleme-nix.lib.${system};

      # Build inputs for tree-sitter C dependencies
      buildInputs = with pkgs; [
        openssl
      ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
        pkgs.libiconv
      ];

      nativeBuildInputs = with pkgs; [
        pkg-config
        clang
      ];

      # Build the tool using crate2nix from pleme-nix
      narsil-mcp = nixLib.mkCrate2nixTool {
        toolName = "narsil-mcp";
        src = ./.;
        inherit buildInputs nativeBuildInputs;
        crateOverrides = {
          # Tree-sitter crates need C compiler
          tree-sitter = attrs: {
            nativeBuildInputs = (attrs.nativeBuildInputs or []) ++ [ pkgs.clang ];
          };
        };
      };

    in {
      packages = {
        default = narsil-mcp;
        narsil-mcp = narsil-mcp;
      };

      apps.default = {
        type = "app";
        program = "${narsil-mcp}/bin/narsil-mcp";
      };

      devShells.default = pkgs.mkShell {
        inputsFrom = [ ];
        packages = with pkgs; [
          # Rust toolchain (from pleme-nix overlay)
          rustc
          cargo
          clippy
          rustfmt
          rust-analyzer

          # Build dependencies
          pkg-config
          clang
          openssl
        ] ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.libiconv
        ];

        RUST_SRC_PATH = "${pkgs.rustc}/lib/rustlib/src/rust/library";
      };
    });
}
