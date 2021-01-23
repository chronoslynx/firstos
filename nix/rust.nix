{ sources ? import ./sources.nix }:

let
  pkgs =
    import sources.nixpkgs { overlays = [ (import sources.nixpkgs-mozilla) ]; };
  channel = "nightly";
  date = "2021-01-22";
  targets = [ ];
  chan = (pkgs.rustChannelOf { channel = channel; date = date; }).rust.override {
    extensions = ["rust-src" "rust-analysis" "clippy-preview" "llvm-tools-preview"];
  } ;
in chan

