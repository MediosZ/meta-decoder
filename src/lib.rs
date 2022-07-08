#![feature(rustc_private)]
#![feature(once_cell)]
// allow us to match& on Box<T>s:
#![feature(box_patterns)]
#![feature(let_else)]
#![feature(iter_zip)]

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
use rustc_middle::hir::exports::Export;
use rustc_middle::ty::TyKind;
use rustc_session::config::{self, Input};
use rustc_session::DiagnosticOutput;
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
    for (name, entry) in sopts.externs.iter() {
        dbg!(name);
        dbg!(entry);
    }
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
        input_path: None,
        output_file: Some("./main".into()),
        output_dir: None,
        file_loader: None,
        diagnostic_output: DiagnosticOutput::Default,
        lint_caps: Default::default(),
        parse_sess_created: None,
        register_lints: None,
        override_queries: None,
        make_codegen_backend: None,
        registry: diagnostics_registry(),
        stderr: None,
    };

    interface::run_compiler(config, |compiler| {
        let linker = compiler.enter(|queries| {
            let (crate_num, c_store) =
                queries
                    .expansion()?
                    .peek_mut()
                    .1
                    .borrow_mut()
                    .access(|resolver| {
                        let c_store = resolver.cstore().clone();
                        let extern_crate = c_store.crates_untracked().last().cloned().unwrap();
                        (extern_crate, c_store)
                    });

            queries.global_ctxt()?.peek_mut().enter(|ctxt| {
                let name = ctxt.crate_name(crate_num);
                println!("processing crate: {name}");

                let children = c_store.item_children_untracked(crate_num.as_def_id(), ctxt.sess);
                for child in children {
                    println!("--------------------------");
                    let Export { res, .. } = child;
                    dbg!(child);
                    match res {
                        Res::Def(DefKind::Struct, def_id) => {
                            let _field_names =
                                c_store.struct_field_names_untracked(def_id, ctxt.sess);
                            let _field_visibilities =
                                c_store.struct_field_visibilities_untracked(def_id);
                            // get fields
                            let fields = c_store.item_children_untracked(def_id, ctxt.sess);
                            dbg!(fields);
                        }
                        Res::Def(DefKind::Fn, def_id) => {
                            let fn_sig = ctxt.fn_sig(def_id);
                            dbg!(fn_sig);
                            let names = ctxt.fn_arg_names(def_id);
                            let inputs = fn_sig.inputs().skip_binder();
                            for (name, ty) in zip(names, inputs) {
                                let real_name = name.to_string();
                                let ty_name = ty.to_string();
                                println!("function arg: {} : {}", real_name, ty_name);
                                dbg!(ty.kind());
                            }
                            let output = fn_sig.output().skip_binder();
                            match output.kind() {
                                TyKind::Tuple(arg) => {
                                    if arg.len() == 0 {
                                        println!("default return");
                                    } else {
                                        let _ret = arg[0].unpack();
                                    }
                                }
                                _ => {}
                            }
                        }
                        _ => {}
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
