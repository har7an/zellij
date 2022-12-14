//! CLI flags for `cargo xtask`
use std::ffi::OsString;
use std::path::PathBuf;

xflags::xflags! {
    src "./src/flags.rs"

    /// Custom build commands for zellij
    cmd xtask {
        /// Deprecation warning. Compatibility to transition from `cargo make`.
        cmd deprecated {}

        /// Tasks for the CI
        cmd ci {
            /// end-to-end tests
            cmd e2e {
                /// Build E2E binary of zellij
                optional --build
                /// Run the E2E tests
                optional --test
                /// Additional arguments for `--test`
                repeated args: OsString
            }

            /// Perform cross-compiled release builds
            cmd cross {
                /// Target-triple to compile the application for
                required triple: OsString
            }
        }

        /// Publish zellij and all the sub-crates
        cmd publish {
            /// Perform a dry-run (don't push/publish anything)
            optional --dry-run
        }

        /// Package zellij for distribution (result found in ./target/dist)
        cmd dist {}

        /// Run `cargo clippy` on all crates
        cmd clippy {}

        /// Sequentially call: format, build, test, clippy
        cmd make {
            /// Build in release mode without debug symbols
            optional -r, --release
            /// Clean project before building
            optional -c, --clean
        }

        /// Generate a runnable `zellij` executable with plugins bundled
        cmd install {
            required destination: PathBuf
        }

        /// Run debug version of zellij
        cmd run {
            /// Arguments to pass after `cargo run --`
            repeated args: OsString
        }

        /// Run `cargo fmt` on all crates
        cmd format {
            /// Run `cargo fmt` in check mode
            optional --check
        }

        /// Run application tests
        cmd test {
            /// Arguments to pass after `cargo test --`
            repeated args: OsString
        }

        /// Build the application and all plugins
        cmd build {
            /// Build in release mode without debug symbols
            optional -r, --release
            /// Build only the plugins
            optional -p, --plugins-only
            /// Build everything except the plugins
            optional --no-plugins
        }
    }
}
// generated start
// The following code is generated by `xflags` macro.
// Run `env UPDATE_XFLAGS=1 cargo build` to regenerate.
#[derive(Debug)]
pub struct Xtask {
    pub subcommand: XtaskCmd,
}

#[derive(Debug)]
pub enum XtaskCmd {
    Deprecated(Deprecated),
    Ci(Ci),
    Publish(Publish),
    Dist(Dist),
    Clippy(Clippy),
    Make(Make),
    Install(Install),
    Run(Run),
    Format(Format),
    Test(Test),
    Build(Build),
}

#[derive(Debug)]
pub struct Deprecated;

#[derive(Debug)]
pub struct Ci {
    pub subcommand: CiCmd,
}

#[derive(Debug)]
pub enum CiCmd {
    E2e(E2e),
    Cross(Cross),
}

#[derive(Debug)]
pub struct E2e {
    pub args: Vec<OsString>,

    pub build: bool,
    pub test: bool,
}

#[derive(Debug)]
pub struct Cross {
    pub triple: OsString,
}

#[derive(Debug)]
pub struct Publish {
    pub dry_run: bool,
}

#[derive(Debug)]
pub struct Dist;

#[derive(Debug)]
pub struct Clippy;

#[derive(Debug)]
pub struct Make {
    pub release: bool,
    pub clean: bool,
}

#[derive(Debug)]
pub struct Install {
    pub destination: PathBuf,
}

#[derive(Debug)]
pub struct Run {
    pub args: Vec<OsString>,
}

#[derive(Debug)]
pub struct Format {
    pub check: bool,
}

#[derive(Debug)]
pub struct Test {
    pub args: Vec<OsString>,
}

#[derive(Debug)]
pub struct Build {
    pub release: bool,
    pub plugins_only: bool,
    pub no_plugins: bool,
}

impl Xtask {
    #[allow(dead_code)]
    pub fn from_env_or_exit() -> Self {
        Self::from_env_or_exit_()
    }

    #[allow(dead_code)]
    pub fn from_env() -> xflags::Result<Self> {
        Self::from_env_()
    }

    #[allow(dead_code)]
    pub fn from_vec(args: Vec<std::ffi::OsString>) -> xflags::Result<Self> {
        Self::from_vec_(args)
    }
}
// generated end
