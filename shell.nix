let
  sources = import ./nix/sources.nix;
  rust = import ./nix/rust.nix { inherit sources; };
  pkgs = import sources.nixpkgs { };
  bootimage = pkgs.callPackage ./nix/bootimage.nix {};
in
pkgs.mkShell {
  buildInputs = [
    rust
    bootimage
  ];
}
