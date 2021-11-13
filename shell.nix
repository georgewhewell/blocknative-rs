let
  nixpkgs = import <nixpkgs> { };
in
  with nixpkgs;

let
  rust-toolchain = pkgs.symlinkJoin {
    name = "rust-toolchain";
    paths = with pkgs.rustChannels.nightly; [
      rust rustc cargo rust-src rls rustfmt
    ];
  };

in pkgs.mkShell {
    buildInputs = with pkgs; [
      rust-toolchain
      pkg-config
      openssl
      zlib
    ];

    RUST_LOG = "info";

    shellHook = ''
      rm -rf .toolchain
      ln -sf ${rust-toolchain} .toolchain
    '';
 }
