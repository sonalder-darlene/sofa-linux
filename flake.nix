{
  description = "Sofa for Linux: a TV web browser for KDE Plasma Bigscreen";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      inherit (nixpkgs) lib;
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAll = f: lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});

      # CXX-Qt finds every Qt module through one qmake, but on Nix each module has its own store path: this joins them.
      qtEnv = pkgs: pkgs.qt6.env "sofa-qt" (with pkgs.qt6; [ qtdeclarative qtwebengine qtsvg qtwayland ]);
      # lld (which CXX-Qt picks, see below) is not wrapped by Nix, so it is told where Qt, the C++ library and SQLite are.
      rpathFlags = pkgs: "-C link-arg=-Wl,-rpath,${qtEnv pkgs}/lib:${pkgs.stdenv.cc.cc.lib}/lib:${pkgs.sqlite.out}/lib";

      cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);

      sofa = pkgs: pkgs.stdenv.mkDerivation {
        pname = "sofa-tv-browser";
        inherit (cargoToml.workspace.package) version;

        src = lib.fileset.toSource {
          root = ./.;
          fileset = lib.fileset.unions [ ./APP_ID ./Cargo.toml ./Cargo.lock ./CMakeLists.txt ./core ./app ./data ];
        };
        cargoDeps = pkgs.rustPlatform.importCargoLock { lockFile = ./Cargo.lock; };

        nativeBuildInputs = with pkgs; [
          cmake ninja cargo rustc pkg-config
          # CXX-Qt links with lld when it finds it (GNU ld gets the order of its static libraries wrong).
          lld
          rustPlatform.cargoSetupHook
          qt6.wrapQtAppsHook
        ];
        buildInputs = with pkgs; [
          qt6.qtbase qt6.qtdeclarative qt6.qtwebengine qt6.qtsvg qt6.qtwayland sqlite
          kdePackages.kirigami kdePackages.qqc2-desktop-style
        ];
        nativeCheckInputs = with pkgs; [ desktop-file-utils appstream ];

        # Set here and not as attributes: the qtbase setup hook sets QMAKE itself.
        preConfigure = ''
          export QMAKE=${qtEnv pkgs}/bin/qmake
          export RUSTFLAGS="${rpathFlags pkgs}"
        '';
        cmakeFlags = [ (lib.cmakeBool "SOFA_CARGO_OFFLINE" true) ];
        doCheck = true;

        meta = {
          description = "TV web browser for KDE Plasma Bigscreen, driven by a remote";
          license = lib.licenses.gpl3Only;
          mainProgram = "sofa-tv-browser";
          platforms = lib.platforms.linux;
        };
      };
    in {
      packages = forAll (pkgs: { default = sofa pkgs; });

      checks = forAll (pkgs: { default = sofa pkgs; });

      devShells = forAll (pkgs: {
        default = pkgs.mkShell {
          inputsFrom = [ (sofa pkgs) ];
          packages = with pkgs; [ clippy rustfmt (qtEnv pkgs) shellcheck ];
          # Debug builds compile C++ without optimisation, which _FORTIFY_SOURCE warns about on every file.
          hardeningDisable = [ "fortify" ];
          shellHook = ''
            export QMAKE=${qtEnv pkgs}/bin/qmake
            export RUSTFLAGS="${rpathFlags pkgs}"
          '';
        };
      });
    };
}
