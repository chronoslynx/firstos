let
  sources = import ./nix/sources.nix;
  rust = import ./nix/rust.nix { inherit sources; };
  pkgs = import sources.nixpkgs { };
  #rust = (import ./nix/rust.nix { inherit sources; }).override {
  #};
  bootimage = pkgs.callPackage ./nix/bootimage.nix {};
in
pkgs.mkShell {
  buildInputs = [
    rust
    bootimage
  ];

  #RUST_SRC_PATH="${rust.rust-src}/lib/rustlib/src/rust/src";
}
