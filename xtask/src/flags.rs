//! CLI flags for `cargo xtask`

xflags::xflags! {
    src "./src/flags.rs"

    /// Custom build commands for zellij
    cmd xtask {
        default cmd help {
            /// Print help information
            optional -h, --help
        }

        /// Build the application and all plugins
        cmd build {
            /// Build in release mode without debug symbols
            optional -r, --release
        }

        /// Package zellij for distribution (result found in ./target/dist)
        cmd dist {}

        /// Run application tests
        cmd test {}
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
    Help(Help),
    Build(Build),
    Dist(Dist),
    Test(Test),
}

#[derive(Debug)]
pub struct Help {
    pub help: bool,
}

#[derive(Debug)]
pub struct Build {
    pub release: bool,
}

#[derive(Debug)]
pub struct Dist;

#[derive(Debug)]
pub struct Test;

impl Xtask {
    pub const HELP: &'static str = Self::HELP_;

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
