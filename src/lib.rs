#![feature(rustc_private)]
// allow us to match& on Box<T>s:
#![feature(box_patterns)]

extern crate rustc_ast;
extern crate rustc_driver;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;
use rustc_driver::{args, diagnostics_registry, handle_options};
use rustc_hash::FxHashSet;
use rustc_hir::def::{DefKind, Res};
use rustc_interface::interface;
use rustc_middle::metadata::ModChild;
use rustc_session::config::{self, Input};
use std::default::Default;
use std::env;
use std::iter::zip;
use std::path::Path;
use std::process;
use std::str;

fn run_compiler(at_args: &[String]) -> interface::Result<()> {
    let args = args::arg_expand_all(at_args);
    let Some(matches) = handle_options(&args) else { return Ok(()) };
    let sopts = config::build_session_options(&matches);
    // externs
    // for (name, entry) in sopts.externs.iter() {
    //     dbg!(name);
    //     dbg!(entry);
    // }
    let code = "extern crate package; \n
fn main() {

}";
    let config = interface::Config {
        opts: sopts,
        crate_cfg: FxHashSet::default(),
        input: Input::Str {
            name: rustc_span::FileName::Custom("dummy".into()),
            input: code.into(),
        },
        output_file: Some("./main".into()),
        output_dir: None,
        file_loader: None,
        lint_caps: Default::default(),
        parse_sess_created: None,
        register_lints: None,
        override_queries: None,
        make_codegen_backend: None,
        registry: diagnostics_registry(),
        crate_check_cfg: Default::default(),
        locale_resources: Default::default(),
    };

    interface::run_compiler(config, |compiler| {
        let linker = compiler.enter(|queries| {
            // get crate_num
            queries.parse().unwrap();
            queries.global_ctxt().unwrap().enter(|tcx| tcx.resolver_for_lowering(()));

            let cnum = queries.global_ctxt().unwrap().enter(|tcx| {
                tcx.crates(()).last().clone().unwrap()
            });
            // dbg!(cnum);

            queries.global_ctxt().unwrap().enter(|ctxt| {
                let name = ctxt.crate_name(*cnum);
                println!("processing crate: {name}");
                for child in ctxt.module_children(cnum.as_def_id()) {
                    let ModChild {
                        ident, res, vis, ..
                    } = child;
                    dbg!(res);
                    // dbg!(ident);
                    match res {
                        Res::Def(DefKind::Struct, def_id) => {
                            // get fields
                            for item in ctxt.associated_item_def_ids(def_id){
                                // dbg!(item);
                                let name = ctxt.item_name(*item);
                                let ty = ctxt.type_of(*item).skip_binder();
                                dbg!(name);
                                dbg!(ty);
                            }
                            // get methods
                            for inherent_impl in ctxt.inherent_impls(def_id) {
                                dbg!(inherent_impl);
                                for method in ctxt.associated_item_def_ids(*inherent_impl) {
                                    let fn_sig = ctxt.fn_sig(*method).skip_binder().skip_binder();
                                    dbg!(fn_sig);
                                }
                            }
                        }
                        Res::Def(DefKind::Fn, def_id) => {
                            // https://doc.rust-lang.org/stable/nightly-rustc/rustc_middle/ty/subst/struct.EarlyBinder.html
                            let fn_sig = ctxt.fn_sig(*def_id);
                            // dbg!(fn_sig);
                            let names = ctxt.fn_arg_names(*def_id);
                            let inputs = fn_sig.skip_binder().inputs().skip_binder();
                            for (name, ty) in zip(names, inputs) {
                                let real_name = name.to_string();
                                let ty_name = ty.to_string();
                                println!("function arg: {} : {}", real_name, ty_name);
                                // dbg!(ty.kind());
                            }
                            let output = fn_sig.skip_binder().output().skip_binder();
                            println!("function output: {}", output);
                        }
                        Res::Def(DefKind::Trait, def_id) => {
                            for item in ctxt.associated_item_def_ids(def_id){
                                dbg!(item);
                            }

                        }
                        _ => {}
                    }
                }
                // after parsing all structs, parse tarit implementations.
                for trait_impl in ctxt.trait_impls_in_crate(*cnum) {
                    dbg!(trait_impl);
                    for func in ctxt.associated_item_def_ids(trait_impl) {
                        let fn_sig = ctxt.fn_sig(*func).skip_binder().skip_binder();
                        dbg!(fn_sig);
                    }
                }
            });
            queries.ongoing_codegen()?;
            let linker = queries.linker()?;
            Ok(Some(linker))
        })?;

        if let Some(linker) = linker {
            // this is the "final phase" of the compilation
            // which create the final executable and write it to disk.
            linker.link()?
        }
        Ok(())
    })
}

pub fn process(path: &Path) {
    let out = process::Command::new("rustc")
        .arg("--print=sysroot")
        .current_dir(".")
        .output()
        .unwrap();
    let sysroot = str::from_utf8(&out.stdout).unwrap().trim();
    let mut args = env::args_os()
        .enumerate()
        .map(|(i, arg)| {
            arg.into_string()
                .unwrap_or_else(|arg| format!("argument {} is not valid Unicode: {:?}", i, arg))
        })
        .collect::<Vec<_>>();
    let mut args: Vec<String> = args.drain(1..).collect();
    args.push("--sysroot".into());
    args.push(sysroot.into());
    // args.push("--crate-type=lib".into());
    args.push("--extern".into());
    args.push(format!(
        "package={}",
        path.to_str().expect("Unable to parse path")
    ));
    dbg!(&args);

    if let Err(e) = run_compiler(&args) {
        println!("{:?}", e);
        process::exit(1);
    }
    // cleanup
    // std::fs::remove_file("./librust_out.rlib").expect("unable to cleanup dummy lib");
}
