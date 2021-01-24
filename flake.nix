{
  inputs = {
    utils.url = "github:numtide/flake-utils";
    naersk.url = "github:nmattia/naersk";
    mozillapkgs = {
      url = "github:mozilla/nixpkgs-mozilla";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, utils, naersk, mozillapkgs }:
    utils.lib.eachDefaultSystem (system: let
      pkgs = nixpkgs.legacyPackages."${system}";

      # Get a specific rust version
      mozilla = pkgs.callPackage (mozillapkgs + "/package-set.nix") {};
      rust = (mozilla.rustChannelOf {
        date = "2021-01-22"; # get the current date with `date -I`
        channel = "nightly";
        sha256 = "m0ys5hVDefiCLwM413yo835Kkve5+b49BMmrzg1yQEw=";
      }).rust.override {
        extensions = ["rust-src" "llvm-tools-preview" "rust-analysis"];
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
      };
    });
}
