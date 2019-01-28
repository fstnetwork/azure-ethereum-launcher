with import <nixpkgs> {};
stdenv.mkDerivation rec {
  name = "env";

  env = buildEnv { name = name; paths = buildInputs; };

  LIBCLANG_PATH="${llvmPackages.libclang}/lib";

  buildInputs = [
    pkgconfig
    openssl.dev
    libusb
    clang
    llvmPackages.libclang
  ];
}
