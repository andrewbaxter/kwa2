use {
    aargvark::{
        vark,
        Aargvark,
    },
    base64::Engine,
    std::path::PathBuf,
    wasm_bindgen_cli_support::Bindgen,
};

#[derive(Aargvark)]
struct Args {
    #[vark(flag = "--in-wasm")]
    in_wasm: PathBuf,
    #[vark(flag = "--out-name")]
    out_name: String,
    #[vark(flag = "--out-dir")]
    out_dir: PathBuf,
    #[vark(flag = "--no-modules")]
    no_modules: Option<()>,
    #[vark(flag = "--inline-wasm-js")]
    inline_wasm_js: Option<()>,
}

fn main() {
    let args = vark::<Args>();
    let mut b = Bindgen::new();
    b.input_path(args.in_wasm);
    if args.no_modules.is_some() {
        b.no_modules(true).unwrap();
    } else {
        b.web(true).unwrap();
    }
    b.split_linked_modules(true);
    b.keep_debug(true);
    b.out_name(&args.out_name);
    b.generate(&args.out_dir).unwrap();
    let wasm_path = args.out_dir.join(format!("{}_bg.wasm", args.out_name));
    let map_path = wasm_path.with_extension(format!("{}.map", wasm_path.extension().unwrap().to_str().unwrap()));
    let mut mapper = wasm2map::WASM::load(&wasm_path).unwrap();
    std::fs::write(&map_path, &mapper.map_v3()).unwrap();
    mapper.patch(&format!("./{}", map_path.file_name().unwrap().to_str().unwrap())).unwrap();
    if args.inline_wasm_js.is_some() {
        std::fs::write(
            args.out_dir.join(format!("{}_bg_wasm.js", args.out_name)),
            format!(
                "const {}_bg_wasm = Uint8Array.from(atob(\"{}\"), (c) => c.charCodeAt(0));\n",
                args.out_name,
                base64::engine::general_purpose::STANDARD.encode(std::fs::read(&wasm_path).unwrap())
            ),
        ).unwrap();
    }
}
