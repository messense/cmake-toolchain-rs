use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

/// CMake toolchain
#[derive(Debug, Clone)]
pub struct CMakeToolchain {
    /// Host target
    host: String,
    /// Build target
    target: String,
    /// `CMAKE_SYSROOT`
    sysroot: Option<PathBuf>,
    /// `CMAKE_C_COMPILER`
    cc: PathBuf,
    /// `CMAKE_CXX_COMPILER`
    cxx: PathBuf,
    /// `CMAKE_AR`
    ar: PathBuf,
    /// `CMAKE_RANLIB`
    ranlib: PathBuf,
}

impl CMakeToolchain {
    pub fn new(target: &str) -> Self {
        let version_meta = rustc_version::version_meta().unwrap();
        let mut c_cfg = cc::Build::new();
        c_cfg
            // opt_level, host and target are required
            .host(&version_meta.host)
            .target(target)
            .opt_level(0)
            // Suppress cargo metadata for example env vars printing
            .cargo_metadata(false)
            .cpp(false)
            .debug(false)
            .warnings(false);
        let c_compiler = c_cfg.get_compiler();

        let mut cxx_cfg = c_cfg.clone();
        cxx_cfg.cpp(true);
        let cxx_compiler = c_cfg.get_compiler();

        let mut toolchain = Self {
            host: version_meta.host,
            target: target.to_string(),
            sysroot: None,
            cc: c_compiler.path().to_path_buf(),
            cxx: cxx_compiler.path().to_path_buf(),
            ar: "ar".into(),
            ranlib: "ranlib".into(),
        };
        let ar = toolchain.find_ar();
        toolchain.ar = ar;
        toolchain
    }

    /// Set CMake sysroot
    pub fn sysroot(&mut self, sysroot: PathBuf) -> &mut Self {
        self.sysroot = Some(sysroot);
        self
    }

    /// Get CMake sysroot
    pub fn get_sysroot(&self) -> Option<&Path> {
        self.sysroot.as_deref()
    }

    /// Set C compiler path
    pub fn cc(&mut self, cc: PathBuf) -> &mut Self {
        self.cc = cc;
        self
    }

    /// Get C compiler path
    pub fn get_cc(&self) -> &Path {
        &self.cc
    }

    /// Set C++ compiler path
    pub fn cxx(&mut self, cxx: PathBuf) -> &mut Self {
        self.cxx = cxx;
        self
    }

    /// Get C++ compiler path
    pub fn get_cxx(&self) -> &Path {
        &self.cxx
    }

    /// Set archiver path
    pub fn ar(&mut self, ar: PathBuf) -> &mut Self {
        self.ar = ar;
        self
    }

    /// Get archiver path
    pub fn get_ar(&self) -> &Path {
        &self.ar
    }

    /// Set ranlib path
    pub fn ranlib(&mut self, ranlib: PathBuf) -> &mut Self {
        self.ranlib = ranlib;
        self
    }

    /// Get ranlib path
    pub fn get_ranlib(&self) -> &Path {
        &self.ranlib
    }

    fn find_ar(&self) -> PathBuf {
        if let Some(p) = self.get_var("AR") {
            return p.into();
        }
        let target = &self.target;
        let default_ar = "ar".to_string();
        let program = if target.contains("android") {
            format!("{}-ar", target.replace("armv7", "arm"))
        } else if target.contains("emscripten") {
            "emar".to_string()
        } else if target.contains("msvc") {
            match cc::windows_registry::find(&target, "lib.exe") {
                // FIXME
                // Some(t) => return Ok((t, "lib.exe".to_string())),
                Some(_) => "lib.exe".to_string(),
                None => "lib.exe".to_string(),
            }
        } else if target.contains("illumos") {
            // The default 'ar' on illumos uses a non-standard flags,
            // but the OS comes bundled with a GNU-compatible variant.
            //
            // Use the GNU-variant to match other Unix systems.
            "gar".to_string()
        } else if &self.host != target {
            match self.prefix_for_target(&target) {
                Some(p) => {
                    let target_ar = format!("{}-ar", p);
                    if Command::new(&target_ar).output().is_ok() {
                        target_ar
                    } else {
                        default_ar
                    }
                }
                None => default_ar,
            }
        } else {
            default_ar
        };
        program.into()
    }

    fn getenv(&self, v: &str) -> Option<String> {
        std::env::var(v).ok()
    }

    fn get_var(&self, var_base: &str) -> Option<String> {
        let target = &self.target;
        let host = &self.host;
        let kind = if host == target { "HOST" } else { "TARGET" };
        let target_u = target.replace("-", "_");
        let res = self
            .getenv(&format!("{}_{}", var_base, target))
            .or_else(|| self.getenv(&format!("{}_{}", var_base, target_u)))
            .or_else(|| self.getenv(&format!("{}_{}", kind, var_base)))
            .or_else(|| self.getenv(var_base));
        // FIXME: use Result
        res
    }

    fn prefix_for_target(&self, target: &str) -> Option<String> {
        // CROSS_COMPILE is of the form: "arm-linux-gnueabi-"
        let cc_env = self.getenv("CROSS_COMPILE");
        let cross_compile = cc_env.as_ref().map(|s| s.trim_end_matches('-').to_owned());
        cross_compile.or(match &target[..] {
            "aarch64-pc-windows-gnu" => Some("aarch64-w64-mingw32"),
            "aarch64-uwp-windows-gnu" => Some("aarch64-w64-mingw32"),
            "aarch64-unknown-linux-gnu" => Some("aarch64-linux-gnu"),
            "aarch64-unknown-linux-musl" => Some("aarch64-linux-musl"),
            "aarch64-unknown-netbsd" => Some("aarch64--netbsd"),
            "arm-unknown-linux-gnueabi" => Some("arm-linux-gnueabi"),
            "armv4t-unknown-linux-gnueabi" => Some("arm-linux-gnueabi"),
            "armv5te-unknown-linux-gnueabi" => Some("arm-linux-gnueabi"),
            "armv5te-unknown-linux-musleabi" => Some("arm-linux-gnueabi"),
            "arm-frc-linux-gnueabi" => Some("arm-frc-linux-gnueabi"),
            "arm-unknown-linux-gnueabihf" => Some("arm-linux-gnueabihf"),
            "arm-unknown-linux-musleabi" => Some("arm-linux-musleabi"),
            "arm-unknown-linux-musleabihf" => Some("arm-linux-musleabihf"),
            "arm-unknown-netbsd-eabi" => Some("arm--netbsdelf-eabi"),
            "armv6-unknown-netbsd-eabihf" => Some("armv6--netbsdelf-eabihf"),
            "armv7-unknown-linux-gnueabi" => Some("arm-linux-gnueabi"),
            "armv7-unknown-linux-gnueabihf" => Some("arm-linux-gnueabihf"),
            "armv7-unknown-linux-musleabihf" => Some("arm-linux-musleabihf"),
            "armv7neon-unknown-linux-gnueabihf" => Some("arm-linux-gnueabihf"),
            "armv7neon-unknown-linux-musleabihf" => Some("arm-linux-musleabihf"),
            "thumbv7-unknown-linux-gnueabihf" => Some("arm-linux-gnueabihf"),
            "thumbv7-unknown-linux-musleabihf" => Some("arm-linux-musleabihf"),
            "thumbv7neon-unknown-linux-gnueabihf" => Some("arm-linux-gnueabihf"),
            "thumbv7neon-unknown-linux-musleabihf" => Some("arm-linux-musleabihf"),
            "armv7-unknown-netbsd-eabihf" => Some("armv7--netbsdelf-eabihf"),
            "hexagon-unknown-linux-musl" => Some("hexagon-linux-musl"),
            "i586-unknown-linux-musl" => Some("musl"),
            "i686-pc-windows-gnu" => Some("i686-w64-mingw32"),
            "i686-uwp-windows-gnu" => Some("i686-w64-mingw32"),
            "i686-unknown-linux-gnu" => self.find_working_gnu_prefix(&[
                "i686-linux-gnu",
                "x86_64-linux-gnu", // transparently support gcc-multilib
            ]), // explicit None if not found, so caller knows to fall back
            "i686-unknown-linux-musl" => Some("musl"),
            "i686-unknown-netbsd" => Some("i486--netbsdelf"),
            "mips-unknown-linux-gnu" => Some("mips-linux-gnu"),
            "mips-unknown-linux-musl" => Some("mips-linux-musl"),
            "mipsel-unknown-linux-gnu" => Some("mipsel-linux-gnu"),
            "mipsel-unknown-linux-musl" => Some("mipsel-linux-musl"),
            "mips64-unknown-linux-gnuabi64" => Some("mips64-linux-gnuabi64"),
            "mips64el-unknown-linux-gnuabi64" => Some("mips64el-linux-gnuabi64"),
            "mipsisa32r6-unknown-linux-gnu" => Some("mipsisa32r6-linux-gnu"),
            "mipsisa32r6el-unknown-linux-gnu" => Some("mipsisa32r6el-linux-gnu"),
            "mipsisa64r6-unknown-linux-gnuabi64" => Some("mipsisa64r6-linux-gnuabi64"),
            "mipsisa64r6el-unknown-linux-gnuabi64" => Some("mipsisa64r6el-linux-gnuabi64"),
            "powerpc-unknown-linux-gnu" => Some("powerpc-linux-gnu"),
            "powerpc-unknown-linux-gnuspe" => Some("powerpc-linux-gnuspe"),
            "powerpc-unknown-netbsd" => Some("powerpc--netbsd"),
            "powerpc64-unknown-linux-gnu" => Some("powerpc-linux-gnu"),
            "powerpc64le-unknown-linux-gnu" => Some("powerpc64le-linux-gnu"),
            "riscv32i-unknown-none-elf" => self.find_working_gnu_prefix(&[
                "riscv32-unknown-elf",
                "riscv64-unknown-elf",
                "riscv-none-embed",
            ]),
            "riscv32imac-unknown-none-elf" => self.find_working_gnu_prefix(&[
                "riscv32-unknown-elf",
                "riscv64-unknown-elf",
                "riscv-none-embed",
            ]),
            "riscv32imc-unknown-none-elf" => self.find_working_gnu_prefix(&[
                "riscv32-unknown-elf",
                "riscv64-unknown-elf",
                "riscv-none-embed",
            ]),
            "riscv64gc-unknown-none-elf" => self.find_working_gnu_prefix(&[
                "riscv64-unknown-elf",
                "riscv32-unknown-elf",
                "riscv-none-embed",
            ]),
            "riscv64imac-unknown-none-elf" => self.find_working_gnu_prefix(&[
                "riscv64-unknown-elf",
                "riscv32-unknown-elf",
                "riscv-none-embed",
            ]),
            "riscv64gc-unknown-linux-gnu" => Some("riscv64-linux-gnu"),
            "riscv32gc-unknown-linux-gnu" => Some("riscv32-linux-gnu"),
            "riscv64gc-unknown-linux-musl" => Some("riscv64-linux-musl"),
            "riscv32gc-unknown-linux-musl" => Some("riscv32-linux-musl"),
            "s390x-unknown-linux-gnu" => Some("s390x-linux-gnu"),
            "sparc-unknown-linux-gnu" => Some("sparc-linux-gnu"),
            "sparc64-unknown-linux-gnu" => Some("sparc64-linux-gnu"),
            "sparc64-unknown-netbsd" => Some("sparc64--netbsd"),
            "sparcv9-sun-solaris" => Some("sparcv9-sun-solaris"),
            "armv7a-none-eabi" => Some("arm-none-eabi"),
            "armv7a-none-eabihf" => Some("arm-none-eabi"),
            "armebv7r-none-eabi" => Some("arm-none-eabi"),
            "armebv7r-none-eabihf" => Some("arm-none-eabi"),
            "armv7r-none-eabi" => Some("arm-none-eabi"),
            "armv7r-none-eabihf" => Some("arm-none-eabi"),
            "thumbv6m-none-eabi" => Some("arm-none-eabi"),
            "thumbv7em-none-eabi" => Some("arm-none-eabi"),
            "thumbv7em-none-eabihf" => Some("arm-none-eabi"),
            "thumbv7m-none-eabi" => Some("arm-none-eabi"),
            "thumbv8m.base-none-eabi" => Some("arm-none-eabi"),
            "thumbv8m.main-none-eabi" => Some("arm-none-eabi"),
            "thumbv8m.main-none-eabihf" => Some("arm-none-eabi"),
            "x86_64-pc-windows-gnu" => Some("x86_64-w64-mingw32"),
            "x86_64-uwp-windows-gnu" => Some("x86_64-w64-mingw32"),
            "x86_64-rumprun-netbsd" => Some("x86_64-rumprun-netbsd"),
            "x86_64-unknown-linux-gnu" => self.find_working_gnu_prefix(&[
                "x86_64-linux-gnu", // rustfmt wrap
            ]), // explicit None if not found, so caller knows to fall back
            "x86_64-unknown-linux-musl" => Some("musl"),
            "x86_64-unknown-netbsd" => Some("x86_64--netbsd"),
            _ => None,
        }
        .map(|x| x.to_owned()))
    }

    /// Some platforms have multiple, compatible, canonical prefixes. Look through
    /// each possible prefix for a compiler that exists and return it. The prefixes
    /// should be ordered from most-likely to least-likely.
    fn find_working_gnu_prefix(&self, prefixes: &[&'static str]) -> Option<&'static str> {
        // let suffix = if self.cpp { "-g++" } else { "-gcc" };
        let suffix = "-gcc";
        let extension = std::env::consts::EXE_SUFFIX;

        // Loop through PATH entries searching for each toolchain. This ensures that we
        // are more likely to discover the toolchain early on, because chances are good
        // that the desired toolchain is in one of the higher-priority paths.
        env::var_os("PATH")
            .as_ref()
            .and_then(|path_entries| {
                env::split_paths(path_entries).find_map(|path_entry| {
                    for prefix in prefixes {
                        let target_compiler = format!("{}{}{}", prefix, suffix, extension);
                        if path_entry.join(&target_compiler).exists() {
                            return Some(prefix);
                        }
                    }
                    None
                })
            })
            .map(|prefix| *prefix)
            .or_else(||
            // If no toolchain was found, provide the first toolchain that was passed in.
            // This toolchain has been shown not to exist, however it will appear in the
            // error that is shown to the user which should make it easier to search for
            // where it should be obtained.
            prefixes.first().map(|prefix| *prefix))
    }
}

#[cfg(test)]
mod test {
    use super::CMakeToolchain;

    #[test]
    fn test_cmake_toolchain_for_host() {
        let meta = rustc_version::version_meta().unwrap();
        let host = meta.host;
        let toolchain = CMakeToolchain::new(&host);
        println!("{:#?}", toolchain);
    }
}
