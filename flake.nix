{
  description = "Linux tool to modify HHKB Studio keymap";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }:
    flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {
          inherit system;
        };
      in {
        # Build dependencies for rust
        packages = rec {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = "hhkb-studio-tools";
            version = "0.1.0";
            src = ./.;

            cargoLock = {
              lockFile = ./Cargo.lock;
            };
            nativeBuildInputs = with pkgs; [
              pkg-config
            ];
            # Build inputs for nix-darwin
            buildInputs = with pkgs;
              []
              ++ pkgs.lib.optionals pkgs.stdenv.isDarwin [
                pkgs.darwin.apple_sdk.frameworks.Security
                pkgs.darwin.apple_sdk.frameworks.SystemConfiguration
              ];
            meta = with pkgs.lib; {
              description = "Linux tool to modify HHKB Studio keymap";
              homepage = "https://github.com/yuja/hhkb-studio-tools";
              license = licenses.mit;
              maintainers = [];
              mainProgram = "hhkb-studio-tools";
            };
          };
          # Alias to reference it with hhhkb-studio-tools instead of default
          hhkb-studio-tools = self.packages.${system}.default;

          # Execute with `nix run github:yuja/hhkb-studio-tools`
          apps = {
            default = {
              type = "app";
              program = "${self.packages.${system}.default}/bin/hhkb-studio-tools";
            };
          };

          # Devtools for nix develop
          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              rustc
              cargo
              rust-analyzer
              rustfmt
              clippy
            ];
          };
        };
      }
    );
}
