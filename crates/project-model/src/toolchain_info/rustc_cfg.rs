//! Get the built-in cfg flags for the to be compile platform.

use anyhow::Context;
use cfg::CfgAtom;
use rustc_hash::FxHashMap;
use toolchain::Tool;

use crate::{toolchain_info::QueryConfig, utf8_stdout};

/// Uses `rustc --print cfg` to fetch the builtin cfgs.
pub fn get(
    config: QueryConfig<'_>,
    target: Option<&str>,
    extra_env: &FxHashMap<String, String>,
) -> Vec<CfgAtom> {
    let _p = tracing::info_span!("rustc_cfg::get").entered();

    let rustc_cfgs = rustc_print_cfg(target, extra_env, config);
    let rustc_cfgs = match rustc_cfgs {
        Ok(cfgs) => cfgs,
        Err(e) => {
            tracing::error!(?e, "failed to get rustc cfgs");
            return vec![];
        }
    };

    let rustc_cfgs = rustc_cfgs.lines().map(crate::parse_cfg).collect::<Result<Vec<_>, _>>();
    match rustc_cfgs {
        Ok(rustc_cfgs) => {
            tracing::debug!(?rustc_cfgs, "rustc cfgs found");
            rustc_cfgs
        }
        Err(e) => {
            tracing::error!(?e, "failed to parse rustc cfgs");
            vec![]
        }
    }
}

fn rustc_print_cfg(
    target: Option<&str>,
    extra_env: &FxHashMap<String, String>,
    config: QueryConfig<'_>,
) -> anyhow::Result<String> {
    const RUSTC_ARGS: [&str; 3] = ["--print", "cfg", "-O"];
    let sysroot = match config {
        QueryConfig::Cargo(sysroot, cargo_toml) => {
            let mut cmd = sysroot.tool(Tool::Cargo, cargo_toml.parent());
            cmd.envs(extra_env);
            cmd.env("RUSTC_BOOTSTRAP", "1");
            cmd.args(["rustc", "-Z", "unstable-options"]).args(RUSTC_ARGS);
            if let Some(target) = target {
                cmd.args(["--target", target]);
            }

            match utf8_stdout(&mut cmd) {
                Ok(it) => return Ok(it),
                Err(e) => {
                    tracing::warn!(
                        %e,
                        "failed to run `{cmd:?}`, falling back to invoking rustc directly"
                    );
                    sysroot
                }
            }
        }
        QueryConfig::Rustc(sysroot) => sysroot,
    };

    let mut cmd = sysroot.tool(Tool::Rustc, &std::env::current_dir()?);
    cmd.envs(extra_env);
    cmd.args(RUSTC_ARGS);
    if let Some(target) = target {
        cmd.args(["--target", target]);
    }

    utf8_stdout(&mut cmd).with_context(|| format!("unable to fetch cfgs via `{cmd:?}`"))
}
