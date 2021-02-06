{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = { self, nixpkgs, utils, naersk, rust-overlay }:
    utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
          inherit system;
          overlays = [
            rust-overlay.overlay
          ];
        };

      # Get a specific rust version
      rust = pkgs.rust-bin.nightly."2021-01-22".rust.override {
         extensions = [ "rust-src" "rust-analysis" "llvm-tools-preview" "clippy"];
      };

      bootimage = pkgs.callPackage ./nix/bootimage.nix {};
      # Override the version used in naersk
      naersk-lib = naersk.lib."${system}".override {
        cargo = rust;
        rustc = rust;
      };
    in rec {
      # `nix build`
      packages.kernel = naersk-lib.buildPackage {
        pname = "kernel";
        root = ./.;
      };
      defaultPackage = packages.kernel;

      # `nix run`
      apps.kernel = utils.lib.mkApp {
        drv = packages.kernel;
      };
      defaultApp = apps.kernel;

      # `nix develop`
      devShell = pkgs.mkShell {
        # supply the specific rust version
        nativeBuildInputs = [ rust bootimage ];
        RUST_SRC_PATH = "${rust}/lib/rustlib/src/rust/src";
      };
    });
}
