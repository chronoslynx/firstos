{ lib, rustPlatform, fetchFromGitHub }:

rustPlatform.buildRustPackage rec {
  pname = "bootimage";
  version = "v0.10.2";

  src = fetchFromGitHub {
    owner = "rust-osdev";
    repo = pname;
    rev = version;
    sha256 = "0a88ckxvh6ydxqmx54bggbpanz7m8dzy3qp5ir9q9nk9xa0ykpg1";
  };

  cargoSha256 = "0hnrizml0nxhpjydiwb6k6dai29nqirm4j4zkv6alsyxv4mqx93d";

  meta = with lib; {
    description = "Tool to create a bootable OS image from a kernel binary.";
    homepage = "https://github.com/rust-osdev/bootimage";
    license = "Apache-2.0";
    maintainers = [ chronoslynx ];
  };
}
