{
  description = "OpenPencil native editor, web host, and web SDK flake";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    rs-harbor = {
      url = "git+https://codeberg.org/caniko/rs-harbor.git?ref=trunk";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
      inputs.crane.follows = "crane";
      inputs.rust-overlay.follows = "rust-overlay";
    };
    py-harbor = {
      url = "git+https://codeberg.org/caniko/py-harbor.git?ref=trunk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    js-harbor = {
      url = "git+https://codeberg.org/caniko/js-harbor.git?ref=trunk";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.flake-parts.follows = "flake-parts";
    };

    nix-appimage.url = "github:ralismark/nix-appimage";

    nix-pklx = {
      url = "git+https://codeberg.org/caniko/nix-pklx.git";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    agent = {
      url = "github:ZSeven-W/agent-rs/5c0f9506e27be5b4f29cd2c1093e2858ad0fa20c";
      flake = false;
    };
    casement = {
      url = "github:ZSeven-W/casement/451173eb3353a4166d2d0f241f2e0606051064bd";
      flake = false;
    };
    jian = {
      url = "github:ZSeven-W/jian/df2376a018acbd8ec8d9fac58b05dde14a405aca";
      flake = false;
    };
  };

  outputs = inputs @ {
    self,
    flake-parts,
    nixpkgs,
    crane,
    rust-overlay,
    rs-harbor,
    py-harbor,
    js-harbor,
    nix-appimage,
    nix-pklx,
    agent,
    casement,
    jian,
    ...
  }:
    flake-parts.lib.mkFlake {inherit inputs;} {
      systems = ["x86_64-linux"];

      flake = {
        lib.integrationManifest = import ./nix/integration/openpencil.nix;
      };

      perSystem = {system, ...}: let
        lib = pkgs.lib;
        pkgs = import nixpkgs {
          inherit system;
          overlays = [(import rust-overlay)];
        };
        version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).workspace.package.version;
        releaseManifest = builtins.fromJSON (builtins.readFile ./nix/release-manifest.json);
        releaseVersion = releaseManifest.version;

        toolchain = rs-harbor.lib.mkToolchain {
          inherit pkgs;
          toolchainFile = ./rust-toolchain.toml;
          cache.enable = false;
        };
        craneLib = toolchain.craneLib;
        rustNativeInputs = rs-harbor.lib.mkRustNativeBuildInputs {
          inherit pkgs;
          extra = [pkgs.pkg-config pkgs.cmake pkgs.ninja];
        };
        nativeLibraries = with pkgs; [
          fontconfig
          freetype
          libGL
          libdrm
          libglvnd
          libxkbcommon
          mesa
          wayland
          libx11
          libxcursor
          libxext
          libxfixes
          libxi
          libxrandr
          libxrender
          libxcb
          libxcb-util
          libxcb-image
          libxcb-keysyms
          libxcb-render-util
          libxcb-wm
        ];
        runtimeLibraries = nativeLibraries;
        # Release archives are Ubuntu-built ELF binaries.  Patch their
        # interpreter and shared-library references into the Nix store before
        # wrapping them, otherwise they fail before LD_LIBRARY_PATH is read.
        prebuiltDesktopRuntimeLibraries =
          runtimeLibraries
          ++ [
            pkgs.zlib
            pkgs.stdenv.cc.cc.lib
          ];
        prebuiltCliRuntimeLibraries = [pkgs.stdenv.cc.cc.lib];

        source = pkgs.runCommand "openpencil-source-${version}" {} ''
          mkdir -p $out/vendor/agent $out/vendor/casement $out/vendor/jian
          cp -R --no-preserve=ownership ${./.}/. $out/
          rm -rf $out/vendor/agent $out/vendor/casement $out/vendor/jian
          cp -R --no-preserve=ownership ${agent}/. $out/vendor/agent
          cp -R --no-preserve=ownership ${casement}/. $out/vendor/casement
          cp -R --no-preserve=ownership ${jian}/. $out/vendor/jian
        '';

        skiaBinaries = pkgs.fetchurl {
          url = "https://github.com/rust-skia/skia-binaries/releases/download/0.97.2/skia-binaries-da8fc6731fc439bc3b6a-x86_64-unknown-linux-gnu-gl-jpegd-jpege-pdf-textlayout.tar.gz";
          hash = "sha256-7nf70Bg+hU4pcnZwXk6GhYN8bH0DBEcslxRfzY9/LPw=";
        };
        skiaRepositoryHash = "da8fc6731fc439bc3b6aa90d63506a808f806b26";
        skiaGit = pkgs.writeShellScriptBin "git" ''
          if [ "$1" = rev-parse ] && [ "$2" = --short=20 ] && [ "$3" = HEAD ]; then
            printf '%s\n' ${skiaRepositoryHash}
            exit 0
          fi
          exec ${pkgs.git}/bin/git "$@"
        '';
        nativeEnv = {
          SKIA_BINARIES_URL = "file://${skiaBinaries}";
          FORCE_SKIA_BINARIES_DOWNLOAD = "1";
        };
        commonArgs = {
          inherit version;
          src = ./.;
          cargoLock = ./Cargo.lock;
          strictDeps = true;
          cargoArtifacts = null;
          doCheck = false;
          env = nativeEnv;
          nativeBuildInputs = rustNativeInputs ++ [pkgs.makeWrapper pkgs.python3 pkgs.perl skiaGit];
          buildInputs = nativeLibraries;
          rsHarborCargoTomlContents = builtins.readFile ./Cargo.toml;
          preBuild = ''
            # skia-bindings uses the packaged rust-skia commit to select its
            # prebuilt archive. Recreate that metadata in the Nix source tree
            # because Cargo vendor sources do not retain the upstream Git repo.
            mkdir -p .git/refs/heads
            printf 'ref: refs/heads/main\n' > .git/HEAD
            printf '%s\n' ${skiaRepositoryHash} > .git/refs/heads/main
            mkdir -p vendor
            rm -rf vendor/agent vendor/casement vendor/jian
            cp -R --no-preserve=ownership ${agent} vendor/agent
            cp -R --no-preserve=ownership ${casement} vendor/casement
            cp -R --no-preserve=ownership ${jian} vendor/jian
            export SKIA_BINARIES_URL=${lib.escapeShellArg nativeEnv.SKIA_BINARIES_URL}
            export FORCE_SKIA_BINARIES_DOWNLOAD=1
          '';
        };

        nativePackage = craneLib.buildPackage (commonArgs
          // {
            pname = "openpencil-native";
            cargoBuildCommand = "cargo build --release --package op-host-desktop --package op-cli --package op-host-web-server";
            installPhase = ''
              runHook preInstall
              install -Dm755 target/release/openpencil-desktop $out/bin/openpencil-desktop
              install -Dm755 target/release/op $out/bin/op
              install -Dm755 target/release/op-host-web-server $out/bin/op-host-web-server
              install -Dm644 crates/op-host-desktop/assets/icon.png $out/share/icons/hicolor/1024x1024/apps/openpencil.png
              install -Dm644 ${pkgs.writeText "openpencil.desktop" ''
                [Desktop Entry]
                Name=OpenPencil
                Comment=Open-source AI-native vector design tool
                Exec=openpencil-desktop %U
                Terminal=false
                Type=Application
                Icon=openpencil
                Categories=Graphics;
                MimeType=application/x-openpencil;
              ''} $out/share/applications/openpencil.desktop
              install -Dm644 ${pkgs.writeText "openpencil.xml" ''
                <?xml version="1.0" encoding="UTF-8"?>
                <mime-info xmlns="http://www.freedesktop.org/standards/shared-mime-info">
                  <mime-type type="application/x-openpencil">
                    <comment>OpenPencil Document</comment>
                    <glob pattern="*.op"/>
                    <glob pattern="*.pen"/>
                  </mime-type>
                </mime-info>
              ''} $out/share/mime/packages/openpencil.xml
              install -Dm644 crates/op-host-desktop/assets/icon.png $out/share/pixmaps/openpencil.png
              runHook postInstall
            '';
            postFixup = ''
              for program in openpencil-desktop op op-host-web-server; do
                wrapProgram "$out/bin/$program" \
                  --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath runtimeLibraries}
              done
            '';
          });

        runtimePackage = pkgs.symlinkJoin {
          name = "openpencil-runtime-${version}";
          paths = [nativePackage];
          postBuild = ''
            install -Dm644 ${defaultDocument} "$out/share/openpencil/default.op"
          '';
        };

        # Upstream publishes matching Linux archives for tagged releases. These
        # outputs are deliberately separate from the source-built packages: the
        # source build remains the default, while users with a matching release
        # asset can avoid compiling Rust and Skia.
        prebuiltDesktopPackage = pkgs.stdenvNoCC.mkDerivation {
          pname = "openpencil-prebuilt";
          version = releaseVersion;
          src = pkgs.fetchurl {
            url = "https://github.com/ZSeven-W/openpencil/releases/download/v${releaseVersion}/openpencil-desktop-linux-x86_64.tar.gz";
            hash = releaseManifest.desktopHash;
          };
          dontUnpack = true;
          nativeBuildInputs = [pkgs.autoPatchelfHook pkgs.makeWrapper];
          buildInputs = prebuiltDesktopRuntimeLibraries;
          installPhase = ''
            runHook preInstall
            tar -xzf "$src" -C "$TMPDIR"
            install -Dm755 "$TMPDIR/openpencil-desktop" $out/bin/openpencil-desktop
            install -Dm644 ${./crates/op-host-desktop/assets/icon.png} \
              $out/share/icons/hicolor/1024x1024/apps/openpencil.png
            install -Dm644 ${pkgs.writeText "openpencil-prebuilt.desktop" ''
              [Desktop Entry]
              Name=OpenPencil
              Comment=Open-source AI-native vector design tool
              Exec=openpencil-desktop %U
              Terminal=false
              Type=Application
              Icon=openpencil
              Categories=Graphics;
            ''} $out/share/applications/openpencil.desktop
            runHook postInstall
          '';
          postFixup = ''
            wrapProgram $out/bin/openpencil-desktop \
              --prefix LD_LIBRARY_PATH : ${lib.makeLibraryPath prebuiltDesktopRuntimeLibraries}
          '';
        };

        prebuiltCliPackage = pkgs.stdenvNoCC.mkDerivation {
          pname = "openpencil-cli-prebuilt";
          version = releaseVersion;
          src = pkgs.fetchurl {
            url = "https://github.com/ZSeven-W/openpencil/releases/download/v${releaseVersion}/op-cli-linux-x86_64.tar.gz";
            hash = releaseManifest.cliHash;
          };
          dontUnpack = true;
          nativeBuildInputs = [pkgs.autoPatchelfHook];
          buildInputs = prebuiltCliRuntimeLibraries;
          installPhase = ''
            runHook preInstall
            tar -xzf "$src" -C "$TMPDIR"
            install -Dm755 "$TMPDIR/op" $out/bin/op
            runHook postInstall
          '';
        };

        wasmBindgen = rs-harbor.lib.resolveWasmBindgenCli {
          inherit lib pkgs;
          cargoLock = ./Cargo.lock;
        };

        wasmGate = rawName: optimizedName: ''
          wasm_opt_help="$(${pkgs.binaryen}/bin/wasm-opt --help 2>&1 || true)"
          wasm_opt_flags=""
          for flag in --enable-bulk-memory --enable-bulk-memory-opt --enable-nontrapping-float-to-int; do
            if printf '%s\n' "$wasm_opt_help" | grep -qF -- "$flag"; then
              wasm_opt_flags="$wasm_opt_flags $flag"
            fi
          done
          ${pkgs.binaryen}/bin/wasm-opt $wasm_opt_flags -Oz "$out/${rawName}" -o "$out/${optimizedName}"
          cp "$out/${optimizedName}" "$out/${rawName}"
          env_count="$(${pkgs.nodejs}/bin/node -e '
            const fs = require("fs");
            const buf = fs.readFileSync(process.argv[1]);
            WebAssembly.compile(buf).then(mod => {
              console.log(WebAssembly.Module.imports(mod).filter(i => i.module === "env").length);
            }).catch(e => { console.error(e); process.exit(1); });
          ' "$out/${rawName}")"
          test "$env_count" = 0
          gzip_bytes="$(${pkgs.gzip}/bin/gzip -c "$out/${rawName}" | ${pkgs.coreutils}/bin/wc -c)"
          test "$gzip_bytes" -le 6291456
        '';

        webHostBundle = craneLib.buildPackage (commonArgs
          // {
            pname = "openpencil-web-bundle";
            cargoBuildCommand = "cargo build --release --target wasm32-unknown-unknown --no-default-features --features canvaskit --package op-host-web";
            nativeBuildInputs = commonArgs.nativeBuildInputs ++ [wasmBindgen.package pkgs.binaryen pkgs.nodejs pkgs.gzip];
            installPhase = ''
              runHook preInstall
              mkdir -p $out
              ${wasmBindgen.package}/bin/wasm-bindgen --target web --out-dir $out target/wasm32-unknown-unknown/release/op_host_web.wasm
              cp -R crates/op-host-web/assets/canvaskit $out/canvaskit
              ${wasmGate "op_host_web_bg.wasm" "op_host_web_bg.opt.wasm"}
              runHook postInstall
            '';
          });

        webSdkWasm = craneLib.buildPackage (commonArgs
          // {
            pname = "openpencil-web-sdk-wasm";
            cargoBuildCommand = "cargo build --release --target wasm32-unknown-unknown --features canvaskit --package op-web-sdk";
            nativeBuildInputs = commonArgs.nativeBuildInputs ++ [wasmBindgen.package pkgs.binaryen pkgs.nodejs pkgs.gzip];
            installPhase = ''
              runHook preInstall
              mkdir -p $out
              ${wasmBindgen.package}/bin/wasm-bindgen --target web --out-dir $out target/wasm32-unknown-unknown/release/op_web_sdk.wasm
              ${wasmGate "op_web_sdk_bg.wasm" "op_web_sdk_bg.opt.wasm"}
              runHook postInstall
            '';
          });

        webServerPackage = pkgs.symlinkJoin {
          name = "openpencil-${version}";
          paths = [nativePackage];
          postBuild = ''
            mkdir -p $out/bin/web-bundle
            cp -R ${webHostBundle}/. $out/bin/web-bundle/
          '';
        };
        opCliPackage = pkgs.symlinkJoin {
          name = "openpencil-cli-${version}";
          paths = [nativePackage];
          postBuild = ''
            mkdir -p $out/bin
            ln -sf ${nativePackage}/bin/op $out/bin/op
          '';
        };

        defaultDocument = pkgs.writeText "openpencil-default-${version}.op" ''
          {
            "version": "${version}",
            "name": "OpenPencil Nix Session",
            "children": []
          }
        '';

        bunToolchain = js-harbor.lib.mkBunToolchain {
          inherit pkgs;
          packageJson = ./packages/package.json;
        };
        bunDeps = js-harbor.lib.mkBunWorkspaceDeps {
          inherit pkgs;
          bun = bunToolchain.bun;
          src = source;
          packageJson = ./packages/package.json;
          lockfile = ./packages/bun.lock;
          version = "1.3.14";
          installFlags = ["--cwd" "packages"];
          # Bun's hoisted aliases are order-dependent when multiple versions
          # are present; nested consumers already point at the correct ones.
          postInstallNormalize = ''
            rm -f packages/node_modules/.bun/node_modules/{string-width,strip-ansi,wrap-ansi}
          '';
          hash = "sha256-jaGVEagUeYrM2MzjUPvvzIF7DJwafLXJiWLWtts38SI=";
        };
        webSdkPackages = pkgs.stdenvNoCC.mkDerivation {
          pname = "openpencil-web-sdk-packages";
          inherit version;
          src = source;
          nativeBuildInputs = [bunToolchain.bun pkgs.coreutils pkgs.nodejs];
          buildPhase = ''
            runHook preBuild
            rm -rf packages/node_modules packages/op-web-sdk/wasm
            cp -R ${bunDeps}/packages/node_modules packages/node_modules
            for workspace in op-web-sdk op-web-sdk-react op-web-sdk-vue; do
              cp -R ${bunDeps}/packages/$workspace/node_modules packages/$workspace/
            done
            chmod -R u+w packages/node_modules
            chmod -R u+w packages/op-web-sdk/node_modules packages/op-web-sdk-react/node_modules packages/op-web-sdk-vue/node_modules
            patchShebangs packages/node_modules
            find packages/node_modules -type f -exec sed -i \
              -e 's|^#!/usr/bin/env bun$|#!${bunToolchain.bun}/bin/bun|' \
              -e 's|^#!/usr/bin/env node$|#!${pkgs.nodejs}/bin/node|' {} +
            mkdir -p packages/op-web-sdk/wasm
            cp -R ${webSdkWasm}/. packages/op-web-sdk/wasm/
            bun run --cwd packages sync-version:check
            bun run --cwd packages lint
            bun run --cwd packages/op-web-sdk typecheck
            bun run --cwd packages/op-web-sdk test
            bun run --cwd packages/op-web-sdk build
            bun run --cwd packages/op-web-sdk-react typecheck
            bun run --cwd packages/op-web-sdk-react test
            bun run --cwd packages/op-web-sdk-react build
            bun run --cwd packages/op-web-sdk-vue typecheck
            bun run --cwd packages/op-web-sdk-vue test
            bun run --cwd packages/op-web-sdk-vue build
            runHook postBuild
          '';
          installPhase = ''
            runHook preInstall
            mkdir -p $out
            (cd packages/op-web-sdk && bun pm pack --destination $out)
            (cd packages/op-web-sdk-react && bun pm pack --destination $out)
            (cd packages/op-web-sdk-vue && bun pm pack --destination $out)
            runHook postInstall
          '';
        };

        pythonEnv = py-harbor.lib.mkPythonEnv {inherit pkgs;};
        pythonCheck =
          pkgs.runCommand "openpencil-python-check" {
            nativeBuildInputs = [pythonEnv];
          } ''
            cp -R ${source} "$TMPDIR/openpencil-source"
            chmod -R u+w "$TMPDIR/openpencil-source"
            cd "$TMPDIR/openpencil-source"
            python -m compileall -q scripts tools
            touch $out
          '';

        # Exercise the same `nix run` surface that users consume, inside a
        # clean NixOS VM rather than only evaluating the derivations.
        prebuiltTestFlake = pkgs.writeTextDir "flake.nix" ''
          {
            outputs = {self}: {
              apps.x86_64-linux.prebuilt = {
                type = "app";
                program = "${prebuiltDesktopPackage}/bin/openpencil-desktop";
              };
              apps.x86_64-linux.prebuilt-cli = {
                type = "app";
                program = "${prebuiltCliPackage}/bin/op";
              };
            };
          }
        '';

        prebuiltRuntimeTest = pkgs.testers.runNixOSTest ({...}: {
          name = "openpencil-prebuilt-runtime";
          nodes.machine = {pkgs, ...}: {
            system.stateVersion = "25.11";
            virtualisation.memorySize = 2048;
            nix.settings.experimental-features = ["nix-command" "flakes"];
            environment.etc."openpencil-test-flake".source = prebuiltTestFlake;
            environment.variables = {
              LIBGL_ALWAYS_SOFTWARE = "1";
              MESA_LOADER_DRIVER_OVERRIDE = "llvmpipe";
              GALLIUM_DRIVER = "llvmpipe";
            };
            environment.systemPackages = [
              prebuiltDesktopPackage
              prebuiltCliPackage
              pkgs.xvfb-run
            ];
          };
          testScript = ''
            machine.succeed("nix run --offline /etc/openpencil-test-flake#prebuilt-cli -- --version | grep -F ${releaseVersion}")
            machine.succeed("set +e; timeout 15s xvfb-run -a -s '-screen 0 1280x720x24' nix run --offline /etc/openpencil-test-flake#prebuilt >/tmp/openpencil.log 2>&1; rc=$?; test $rc -eq 0 -o $rc -eq 124; ! grep -E 'error while loading|cannot open shared object' /tmp/openpencil.log")
          '';
        });

        runtimePrebuiltPackage = pkgs.symlinkJoin {
          name = "openpencil-runtime-prebuilt-${version}";
          paths = [prebuiltDesktopPackage prebuiltCliPackage];
          nativeBuildInputs = [pkgs.makeWrapper];
          postBuild = ''
            rm -f "$out/bin/op"
            makeWrapper ${prebuiltCliPackage}/bin/op "$out/bin/op" \
              --set-default OPENPENCIL_DESKTOP_BIN "$out/bin/openpencil-desktop"
            install -Dm644 ${defaultDocument} "$out/share/openpencil/default.op"
          '';
        };

        runtimePrebuiltToolsCheck = pkgs.runCommand "openpencil-runtime-prebuilt-tools-${version}" {} ''
          test -x ${runtimePrebuiltPackage}/bin/openpencil-desktop
          test -x ${runtimePrebuiltPackage}/bin/op
          touch "$out"
        '';

        webServerToolsCheck = pkgs.runCommand "openpencil-web-server-tools-${version}" {} ''
          test -x ${webServerPackage}/bin/op-host-web-server
          test -d ${webServerPackage}/bin/web-bundle
          touch "$out"
        '';

        skillBundle = builtins.fromJSON (
          builtins.replaceStrings
          ["__OPENPENCIL_VERSION__"]
          [version]
          (builtins.readFile ./crates/op-cli/assets/skill-bundle.json)
        );
        skillBundleFiles =
          builtins.mapAttrs
          (relativePath: contents:
            pkgs.writeText
            "openpencil-skill-${builtins.replaceStrings ["/"] ["-"] relativePath}"
            contents)
          skillBundle.files;
        skillsPackage = pkgs.runCommand "openpencil-skills-${version}" {} ''
          mkdir -p "$out/share/skillnet/openpencil"
          install -Dm644 ${./nix/integration/Skillnet.pkl} \
            "$out/share/skillnet/openpencil/Skillnet.pkl"
          ${lib.concatStringsSep "\n" (
            lib.mapAttrsToList
            (relativePath: source: "install -Dm644 ${source} \"$out/share/skillnet/openpencil/${relativePath}\"")
            skillBundleFiles
          )}
        '';

        appimage = rs-harbor.lib.mkAppImage {
          inherit nix-appimage system version;
          pname = "openpencil";
          program = "${nativePackage}/bin/openpencil-desktop";
        };
      in {
        packages = {
          default = webServerPackage;
          openpencil = webServerPackage;
          op-cli = opCliPackage;
          runtime = runtimePackage;
          runtime-prebuilt = runtimePrebuiltPackage;
          prebuilt = prebuiltDesktopPackage;
          prebuilt-cli = prebuiltCliPackage;
          skills = skillsPackage;
          web-server = webServerPackage;
          web-bundle = webHostBundle;
          web-sdk-wasm = webSdkWasm;
          web-sdk-packages = webSdkPackages;
          appimage = appimage;
          python-tools = pythonCheck;
        };

        apps = {
          default = {
            type = "app";
            program = "${nativePackage}/bin/openpencil-desktop";
          };
          op = {
            type = "app";
            program = "${nativePackage}/bin/op";
          };
          prebuilt = {
            type = "app";
            program = "${prebuiltDesktopPackage}/bin/openpencil-desktop";
          };
          prebuilt-cli = {
            type = "app";
            program = "${prebuiltCliPackage}/bin/op";
          };
          web-server = {
            type = "app";
            program = "${webServerPackage}/bin/op-host-web-server";
          };
          integration-export = {
            type = "app";
            program = "${pkgs.writeShellApplication {
              name = "openpencil-integration-export";
              runtimeInputs = [nix-pklx.packages.${system}.pklx pkgs.coreutils];
              text = ''
                export SSL_CERT_FILE="${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt"
                output="''${1:-nix/integration/openpencil.nix}"
                tmp="$(mktemp)"
                trap 'rm -f "$tmp"' EXIT
                pklx eval nix/integration/OpenPencil.pkl -o "$tmp"
                mv "$tmp" "$output"
                echo "Wrote $output from nix/integration/OpenPencil.pkl"
              '';
            }}/bin/openpencil-integration-export";
          };
        };

        devShells.default = pkgs.mkShell {
          packages = [
            toolchain.rustToolchain
            bunToolchain.bun
            pythonEnv
            wasmBindgen.package
            pkgs.binaryen
            pkgs.nodejs
            pkgs.gzip
            pkgs.cargo-watch
            pkgs.chromium
          ];
          nativeBuildInputs = rustNativeInputs;
          buildInputs = nativeLibraries;
          env = nativeEnv;
        };

        formatter = pkgs.alejandra;

        checks = {
          default = webServerPackage;
          native = nativePackage;
          web-bundle = webHostBundle;
          web-sdk-wasm = webSdkWasm;
          web-sdk-packages = webSdkPackages;
          python = pythonCheck;
          prebuilt = prebuiltDesktopPackage;
          prebuilt-cli = prebuiltCliPackage;
          prebuilt-runtime = prebuiltRuntimeTest;
          prebuilt-runtime-tools = runtimePrebuiltToolsCheck;
          web-server-tools = webServerToolsCheck;
          integration-manifest =
            pkgs.runCommand "openpencil-integration-manifest" {
              nativeBuildInputs = [nix-pklx.packages.${system}.pklx];
              SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
            } ''
              pklx eval ${./nix/integration/OpenPencil.pkl} -o "$out"
            '';
          integration-sidecar =
            pkgs.runCommand "openpencil-integration-sidecar" {
              nativeBuildInputs = [nix-pklx.packages.${system}.pklx pkgs.diffutils];
              SSL_CERT_FILE = "${pkgs.cacert}/etc/ssl/certs/ca-bundle.crt";
            } ''
              generated="$TMPDIR/openpencil.nix"
              pklx eval ${./nix/integration/OpenPencil.pkl} -o "$generated"
              diff -u ${./nix/integration/openpencil.nix} "$generated"
              touch "$out"
            '';
          flake-format = pkgs.runCommand "openpencil-flake-format" {} ''
            ${pkgs.alejandra}/bin/alejandra --check ${./flake.nix}
            touch $out
          '';
        };
      };
    };
}
