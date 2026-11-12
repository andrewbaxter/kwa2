let
  pkgs = import <nixpkgs> { };
  lib = pkgs.lib;
  fdapLoginSrc = ../../../../fdap-login;
  openfdapSrc = ../../../../openfdap;
  image =
    name: bin: extraBins:
    pkgs.runCommand "kwa-test-container-${name}" { } ''
      set -xeu -o pipefail
      ${pkgs.coreutils}/bin/mkdir -p $out
      ${pkgs.coreutils}/bin/cp ${bin} $out/${name}
      ${pkgs.coreutils}/bin/cp ${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt $out/ca-certificates.crt
      ${lib.concatStrings (
        lib.mapAttrsToList (n: p: "${pkgs.coreutils}/bin/cp ${p} $out/${n}\n") extraBins
      )}
    '';
in
{
  kwa = image "kwa" "${
    import ../../build.nix {
      inherit pkgs lib;
      debug = false;
      musl = true;
    }
  }/bin/native" { ffmpeg = "${pkgs.pkgsStatic.ffmpeg-headless}/bin/ffmpeg"; };
  openfdap = image "openfdap" "${
    import (openfdapSrc + "/source/package.nix") {
      inherit pkgs lib;
      musl = true;
    }
  }/bin/openfdap" { };
  fdap-login = image "fdap-login" "${
    import (fdapLoginSrc + "/source/package.nix") {
      inherit pkgs;
      musl = true;
    }
  }/bin/fdap-login" { };
}
