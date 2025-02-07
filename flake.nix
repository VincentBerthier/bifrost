{
  description = "A Nix-flake-based Rust development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
    };

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
      };
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    rust-overlay,
    crane,
    advisory-db,
  }:
    flake-utils.lib.eachSystem ["x86_64-linux"] (system: let
      pkgs = import nixpkgs {
        inherit system;
        config.allowUnfree = true;
        overlays = [(import rust-overlay)];
      };
      rust = pkgs.rust-bin.nightly.latest.default.override {
        extensions = ["rust-analyzer" "rust-src" "rust-docs" "llvm-tools" "rustc-codegen-cranelift-preview"];
      };
      rustStable = pkgs.rust-bin.stable.latest.default;

      inherit (pkgs) lib;
      craneLib = (crane.mkLib pkgs).overrideToolchain rust;
      craneLibStable = (crane.mkLib pkgs).overrideToolchain rustStable;

      src = lib.cleanSourceWith {
        src = craneLib.path ./.;
        filter = path: type:
          (lib.hasSuffix "\.dic" path)
          || (lib.hasSuffix "\.json" path)
          || (craneLib.filterCargoSources path type);
      };
      buildInputs = with pkgs;
        [
          openssl
          # Add additional build inputs here
        ]
        ++ lib.optionals pkgs.stdenv.isDarwin [
          # Additional darwin specific inputs can be set here
          pkgs.libiconv
        ];

      nativeBuildInputs = with pkgs; [mold];

      commonArgs = {
        inherit src buildInputs nativeBuildInputs;
        pname = "bifrost";
        version = "0.1.0";
        strictDeps = true;
        doCheck = false;
      };

      # Build *just* the cargo dependencies, so we can reuse
      # all of that work (e.g. via cachix) when running in CI
      cargoArtifacts = craneLib.buildDepsOnly commonArgs;

      ######################################################
      ###                  Binaries                      ###
      ######################################################
      # Build the actual crate itself, reusing the dependency
      # artifacts from above.
      bifrost-bin = craneLib.buildPackage (commonArgs
        // {
          inherit src cargoArtifacts;
          doCheck = false;
        });

      ######################################################
      ###               Coverage & Doc                   ###
      ######################################################
      bifrost-docs = craneLib.cargoDoc (commonArgs
        // {
          pnameSuffix = "-docs";
          inherit cargoArtifacts;
        });

      llvm-cov-pretty = craneLibStable.buildPackage (commonArgs
        // {
          pname = "llvm-cov-pretty";
          version = "0.1.9";
          cargoArtifacts = null;

          src = pkgs.fetchFromGitHub {
            owner = "dnaka91";
            repo = "llvm-cov-pretty";
            rev = "v0.1.10";
            sha256 = "sha256-3QtDAQGVcqRDfjgl4Lq3Ue/6/yH61YPkM/JXdQJdoNo=";
            fetchSubmodules = true;
          };

          cargoBuildCommand = "pnpm run build && cargo build --profile release";
          doCheck = true;
          cargoTestExtraArgs = "-- --skip version";
          nativeBuildInputs = commonArgs.nativeBuildInputs ++ [pkgs.tailwindcss pkgs.pnpm];
        });

      bifrost-coverage = craneLib.mkCargoDerivation (commonArgs
        // {
          inherit src;

          pnameSuffix = "-coverage";
          # CARGO_BUILD_JOBS = 8;
          cargoArtifacts = null;

          buildPhaseCargoCommand = ''
            cargo llvm-cov nextest --ignore-filename-regex="^\\\/nix\\\/store\\\/*" --locked --all-features --json --output-path coverage.json
          '';
          doInstallCargoArtifacts = false;
          installPhase = ''
            mkdir -p $out
            ${llvm-cov-pretty}/bin/llvm-cov-pretty --theme dracula --output-dir $out coverage.json
            cp coverage.json $out/
          '';
          nativeBuildInputs = commonArgs.nativeBuildInputs ++ [pkgs.cargo-llvm-cov pkgs.cargo-nextest];
        });

      bifrost-pgo = craneLib.mkCargoDerivation (commonArgs
        // {
          inherit src;
          pnameSuffix = "-pgo";

          cargoArtifacts = null;
          configurePhase = ''
          '';
          buildPhaseCargoCommand = ''
            cargo pgo build
            ./etc/create_keyspace.sh pgo cassandra cassandra
            ./target/x86_64-unknown-linux-gnu/release/bifrost
            # cargo pgo optimize
            cargo pgo bolt build --with-pgo
            ./target/x86_64-unknown-linux-gnu/release/bifrost-bolt-instrumented
            cargo pgo bolt optimize --with-pgo
          '';
          doInstallCargoArtifacts = false;
          installPhase = ''
            mkdir -p $out/bin
            cp ./target/x86_64-unknown-linux-gnu/release/bifrost $out/bin/
          '';
          fixupPhase = ''
          '';

          nativeBuildInputs = commonArgs.nativeBuildInputs ++ [pkgs.cargo-pgo pkgs.bolt_19];
        });

      bifrost-pgo-time = pkgs.stdenvNoCC.mkDerivation {
        inherit system src;
        name = "bifrost-time-pgo";

        configurePhase = ''
          ./etc/create_keyspace.sh pgo
        '';

        buildPhase = ''
          perf stat -ddd -o perf.log -r 10 -B ${bifrost-bin}/bin/bifrost
          perf stat -ddd -o perf.log --append -r 10 -B ${bifrost-pgo}/bin/bifrost
          ./etc/create_keyspace.sh pgo
        '';
        installPhase = ''
          mkdir -p $out
          cp perf.log $out/perf.log
        '';

        fixupPhase = ''
        '';
        nativeBuildInputs = (commonArgs.nativeBuildInputs or []) ++ [pkgs.linuxPackages_latest.perf];
      };
    in {
      checks = {
        # Build the crate as part of `nix flake check` for convenience
        inherit cargoArtifacts;

        ######################################################
        ###               Nix flake checks                 ###
        ######################################################
        # Run clippy (and deny all warnings) on the crate source,
        # again, resuing the dependency artifacts from above.
        #
        # Note that this is done as a separate derivation so that
        # we can block the CI if there are issues here, but not
        # prevent downstream consumers from building our crate by itself.
        bifrost-clippy = craneLib.cargoClippy (commonArgs
          // {
            inherit src cargoArtifacts;
            pnameSuffix = "-clippy";

            cargoClippyExtraArgs = "--all-features --all-targets -- --deny warnings";
          });

        # Check formatting
        bifrost-fmt = craneLib.cargoFmt {
          inherit src;
          pnameSuffix = "-fmt";
        };

        # Audit dependencies
        bifrost-audit = craneLib.cargoAudit {
          inherit src advisory-db;
          pnameSuffix = "-audit";
        };

        # Audit licenses
        bifrost-deny = craneLib.cargoDeny {
          inherit src;
          pnameSuffix = "-deny";
        };

        # Run tests with cargo-nextest
        bifrost-nextest = craneLib.cargoNextest (commonArgs
          // {
            inherit src cargoArtifacts;
            pnameSuffix = "-tests";

            checkPhaseCargoCommand = "cargo nextest run";
            partitions = 1;
            partitionType = "count";
          });

        bifrost-spellcheck = craneLib.mkCargoDerivation (commonArgs
          // {
            inherit src cargoArtifacts;

            pnameSuffix = "-spellcheck";
            buildPhaseCargoCommand = "HOME=./ cargo spellcheck check -m 1";
            nativeBuildInputs = (commonArgs.buildInputs or []) ++ [pkgs.cargo-spellcheck];
          });
      };
      ######################################################
      ###                 Build packages                 ###
      ######################################################
      packages = {
        default = bifrost-bin;

        coverage = bifrost-coverage;
        docs = bifrost-docs;
        pgo = bifrost-pgo;
        time-pgo = bifrost-pgo-time;
      };

      ######################################################
      ###                   Dev’ shell                   ###
      ######################################################
      devShells.default = craneLib.devShell {
        name = "devshell";

        # Inherit inputs from checks.
        checks = self.checks.${system};

        # Additional dev-shell environment variables can be set directly
        PATH = "${pkgs.mold}/bin/mold";
        LD_LIBRARY_PATH = lib.makeLibraryPath buildInputs;

        shellHook = ''
          export PATH="$HOME/.cargo/bin:$PATH"
          if [ -z $HOME/.cargo/bin/gitmoji ]; then cargo install -q gitmoji; fi
          echo "Environnement $(basename $(pwd)) chargé" | cowsay | lolcat
        '';

        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = with pkgs; [
          # Compilation
          bolt_19 # PGO

          # Utils
          cowsay
          gitmoji-cli # Use gitmojis to commit
          gnuplot
          lolcat
          tokei # file lines count
          tokio-console
          llvm-cov-pretty

          # Formatting
          dprint
          taplo

          # Cargo utilities
          bacon
          cargo-audit # vulnerabilities
          cargo-bloat # check binaries size (which is fun but not terriby useful?)
          cargo-cache # cargo cache -a
          cargo-criterion # Benchmarks
          cargo-deny # licenses, vultnerabilities
          cargo-expand # for macro expension
          cargo-flamegraph # profiling visualization
          cargo-llvm-cov # for coverage
          cargo-machete # find unecessary crates in Cargo.toml
          cargo-mutants # Mutation tests
          cargo-outdated # update to latest major versions of dependencies
          cargo-pgo # PGO obviously
          cargo-spellcheck # Spellcheck documentation
          cargo-update # update installed binaries
          cargo-wizard
          samply # Profiling
        ];
      };
    });
}
