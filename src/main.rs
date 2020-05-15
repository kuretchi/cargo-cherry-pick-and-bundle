use anyhow::Result;
use cargo_cherry_pick_and_bundle::*;
use std::{env, path::PathBuf};
use structopt::{clap::AppSettings, StructOpt};

mod cargo {
    pub use cargo::{
        core::{manifest::EitherManifest, SourceId},
        util::{toml::read_manifest, Config},
    };
}

#[derive(StructOpt)]
#[structopt(
    bin_name("cargo"),
    global_settings(&[AppSettings::UnifiedHelpMessage])
)]
enum Opt {
    CherryPickAndBundle {
        #[structopt(long, name = "PATH", help = "Path to crate")]
        path: Option<PathBuf>,
    },
}

fn main() -> Result<()> {
    let Opt::CherryPickAndBundle { path } = Opt::from_args();

    let path = match path {
        Some(path) => path.canonicalize()?,
        None => env::current_dir()?,
    };

    let crate_name = {
        let (manifest, _) = cargo::read_manifest(
            &path.join("Cargo.toml"),
            cargo::SourceId::for_path(&path)?,
            &cargo::Config::default()?,
        )?;
        match manifest {
            cargo::EitherManifest::Real(manifest) => manifest.name().replace("-", "_"),
            _ => todo!(),
        }
    };

    let output = bundle(
        &crate_name,
        &path,
        |item_mod_ident| loop {
            eprint!(
                "Leave module `{}` [a(ll),p(artial),n(one)]? ",
                item_mod_ident
            );
            let input: String = text_io::read!();
            return match &*input {
                "a" | "all" => SelectItemMod::All,
                "p" | "partial" => SelectItemMod::Partial,
                "n" | "none" => SelectItemMod::None,
                _ => continue,
            };
        },
        |content| loop {
            for line in content.lines() {
                eprintln!("> {}", line);
            }
            eprint!("Leave this use statement [y,n]? ");
            let input: String = text_io::read!();
            return match &*input {
                "y" => true,
                "n" => false,
                _ => continue,
            };
        },
    )?;

    print!("{}", output);
    Ok(())
}
