{
  pkgs,
  lib,
  debug ? true,
  musl ? false,
}:
let
  fenix =
    import
      (fetchTarball "https://github.com/nix-community/fenix/archive/93523fa073f781d3d02d326cdbb85f8709b00c40.zip")
      { };
  toolchain = fenix.combine (
    [
      fenix.latest.rustc
      fenix.latest.cargo
      fenix.latest.rust-src
    ]
    ++ lib.optionals musl [ fenix.targets.x86_64-unknown-linux-musl.latest.rust-std ]
  );
  muslCc = pkgs.pkgsCross.musl64.stdenv.cc;
  muslCcBin = "${muslCc}/bin/${muslCc.targetPrefix}cc";
  craneLib =
    (import
      (fetchTarball {
        url = "https://github.com/ipetkov/crane/archive/10e6e3cb966f7cfcc789fe5eee7a85f3188ce08b.tar.gz";
        sha256 = "0pax1rc5l08gx0fhx24zyn4kxk16lh47ykifv99hd50fzzcq6wcy";
      })
      { inherit pkgs; }
    ).overrideToolchain
      toolchain;
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

  # Only the parts of spaghettinuum the kwa crates depend on (the full checkout is ~1.8G incl. infra
  # and .git, and any change there would otherwise invalidate the workspace).
  spaghettinuumSrc =
    let
      root = toString ./spaghettinuum;
      keep = [
        "source/native"
        "source/shared"
      ];
    in
    builtins.path {
      name = "spaghettinuum";
      path = ./spaghettinuum;
      filter =
        path: type:
        let
          rel = lib.removePrefix (root + "/") (toString path);
        in
        baseNameOf path != "target"
        && lib.any (k: rel == k || lib.hasPrefix (k + "/") rel || lib.hasPrefix (rel + "/") k) keep;
    };

  workspaceWasm = stageWorkspace "kwa-wasm" [
    ./nixbuild/wasm/Cargo.toml
    ./wasm/Cargo.lock
    ./wasm/.cargo
    ./wasm
    ./shared
    spaghettinuumSrc
    ./rooting-forms
  ];
  wasm = craneLib.buildPackage {
    pname = "kwa-wasm";
    version = "0.1.0";
    src = workspaceWasm;
    cargoVendorDir = craneLib.vendorMultipleCargoDeps {
      inherit (craneLib.findCargoFiles workspaceWasm) cargoConfigs;
      cargoLockList = [
        "${workspaceWasm}/Cargo.lock"
        "${fenix.latest.rust-src}/lib/rustlib/src/rust/library/Cargo.lock"
      ];
    };
    CARGO_PROFILE = "release";
    doCheck = false;
  };

  nativeBindgen = craneLib.buildPackage {
    src = ./native-bindgen;
    doCheck = false;
  };

  static = pkgs.runCommand "kwa-static" { } ''
    set -xeu -o pipefail
    ${pkgs.coreutils}/bin/mkdir -p $out version
    ${nativeBindgen}/bin/bind_wasm --in-wasm ${wasm}/bin/main.wasm --out-name main --out-dir version
    ${pkgs.coreutils}/bin/cp ${./wasm}/static/style.js ${./wasm}/static/style_reset.css version/
    hash=$(cd version && ${pkgs.findutils}/bin/find . -type f | ${pkgs.coreutils}/bin/sort | ${pkgs.findutils}/bin/xargs ${pkgs.coreutils}/bin/sha256sum | ${pkgs.coreutils}/bin/sha256sum | ${pkgs.coreutils}/bin/cut -c1-16)
    ${nativeBindgen}/bin/bind_wasm --in-wasm ${wasm}/bin/serviceworker.wasm --out-name serviceworker --no-modules --inline-wasm-js --out-dir $out
    ${pkgs.coreutils}/bin/cp -r ${./wasm}/static/* $out/
    ${pkgs.coreutils}/bin/mkdir $out/v
    ${pkgs.coreutils}/bin/mv version $out/v/$hash
    ${pkgs.gnused}/bin/sed -i -e "s/KWA_VERSION/$hash/g" $out/index.html
  '';

  workspaceNative = stageWorkspace "kwa-native" [
    ./nixbuild/native/Cargo.toml
    ./native/Cargo.lock
    ./native
    ./shared
    spaghettinuumSrc
  ];
  nativeCommon = {
    pname = "kwa-native";
    version = "0.1.0";
    src = workspaceNative;
    CARGO_PROFILE = if debug then "dev" else "release";
    doCheck = false;
  }
  // (
    if musl then
      {
        CARGO_BUILD_TARGET = "x86_64-unknown-linux-musl";
        CARGO_BUILD_RUSTFLAGS = "-C target-feature=+crt-static";
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER = muslCcBin;
        CC_x86_64_unknown_linux_musl = muslCcBin;
        HOST_CC = "${pkgs.stdenv.cc}/bin/cc";
        OPENSSL_STATIC = "1";
        OPENSSL_LIB_DIR = "${pkgs.pkgsStatic.openssl.out}/lib";
        OPENSSL_INCLUDE_DIR = "${pkgs.pkgsStatic.openssl.dev}/include";
      }
    else
      {
        CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_RUSTFLAGS = "-Clinker-features=-lld";
        nativeBuildInputs = [
          pkgs.pkg-config
          pkgs.rustPlatform.bindgenHook
        ];
        buildInputs = [
          pkgs.sqlite
          pkgs.openssl
        ];
      }
  );
  # Built separately so that STATIC_DIR (which depends on the wasm output) doesn't end up in the
  # deps derivation and invalidate it on every wasm change.
  nativeDeps = craneLib.buildDepsOnly nativeCommon;
  native = craneLib.buildPackage (
    nativeCommon
    // {
      cargoArtifacts = nativeDeps;
      STATIC_DIR = "${static}";
    }
  );
in
native
