{ pkgs ? import <nixpkgs> {}, ... }:
with pkgs;
stdenv.mkDerivation {
  name = "fetch_unroll";

  nativeBuildInputs = [pkgconfig];
  buildInputs = [openssl];
}
