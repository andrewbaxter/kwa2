{
  pkgs,
  lib,
  debug ? true,
}:
let
  fenix =
    import
      (fetchTarball "https://github.com/nix-community/fenix/archive/93523fa073f781d3d02d326cdbb85f8709b00c40.zip")
      { };
  toolchain = fenix.combine [
    fenix.latest.rustc
    fenix.latest.cargo
    fenix.latest.rust-src
  ];
  naersk =
    pkgs.callPackage
      (fetchTarball "https://github.com/nix-community/naersk/archive/378614f37a6bee5a3f2ef4f825a73d948d3ae921.zip")
      {
        rustc = toolchain;
        cargo = toolchain;
      };
  stageWorkspace =
    name: files:
    let
      linkLines = lib.strings.concatStringsSep "\n" (
        map (f: ''
          filename=$(${pkgs.coreutils}/bin/basename ${f} | ${pkgs.gnused}/bin/sed -e 's/[^-]*-//')
          ${pkgs.coreutils}/bin/cp -r ${f} $filename
        '') files
      );
    in
    pkgs.runCommand "stage-rust-workspace-${name}" { } ''
      set -xeu -o pipefail
      ${pkgs.coreutils}/bin/mkdir $out
      cd $out
      ${linkLines}
    '';

  workspaceWasm = stageWorkspace "kwa-wasm" [
    ./nixbuild/wasm/Cargo.toml
    ./wasm/Cargo.lock
    ./wasm/.cargo
    ./wasm
    ./shared
    ./spaghettinuum
  ];
  wasm = naersk.buildPackage {
    pname = "kwa-wasm";
    name = "kwa-wasm";
    root = workspaceWasm;
    release = true;
    CARGO_BUILD_TARGET = "wasm32-unknown-unknown";
  };

  nativeBindgen = naersk.buildPackage {
    root = ./native-bindgen;
  };

  static = pkgs.runCommand "kwa-static" { } ''
    set -xeu -o pipefail
    ${pkgs.coreutils}/bin/mkdir -p $out
    ${nativeBindgen}/bin/bind_wasm --in-wasm ${wasm}/bin/main.wasm --out-name main --out-dir $out
    ${nativeBindgen}/bin/bind_wasm --in-wasm ${wasm}/bin/serviceworker.wasm --out-name serviceworker --out-dir $out
    ${pkgs.coreutils}/bin/cp -r ${./wasm}/static/* $out/
  '';

  workspaceNative = stageWorkspace "kwa-native" [
    ./nixbuild/native/Cargo.toml
    ./native/Cargo.lock
    ./native
    ./shared
    ./spaghettinuum
  ];
  native = naersk.buildPackage {
    pname = "kwa-native";
    name = "kwa-native";
    root = workspaceNative;
    release = !debug;
    STATIC_DIR = "${static}";
    nativeBuildInputs = [
      pkgs.pkg-config
      pkgs.rustPlatform.bindgenHook
    ];
    buildInputs = [ pkgs.sqlite ];
  };
in
native
