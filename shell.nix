{ pkgs ? import <nixpkgs> {} }:
with pkgs;
let
  stdenv = multiStdenv; # default multilib (gcc or clang)
  #stdenv = gccMultiStdenv; # gcc multilib
  #stdenv = clangMultiStdenv; # clang multilib
in stdenv.mkDerivation {
    name = "shell";
    nativeBuildInputs = [pkgconfig gdb];
    buildInputs = [openssl];
}
