use {
    convert_case::{
        Case,
        Casing,
    },
    genemichaels_lib::FormatConfig,
    proc_macro2::TokenStream,
    quote::{
        format_ident,
        quote,
    },
    std::{
        env,
        fs::write,
        path::PathBuf,
    },
};

#[derive(PartialEq, Eq)]
enum TypeMod {
    None,
    Opt,
    Arr,
    Arr2,
}

struct Type {
    mod_: TypeMod,
    rust_type: TokenStream,
    ts_type: String,
}

impl PartialEq<Type> for Type {
    fn eq(&self, other: &Type) -> bool {
        self.mod_ == other.mod_ && self.ts_type == other.ts_type
    }
}

impl Eq for Type { }

fn type_opt(t: &Type) -> Type {
    return Type {
        mod_: TypeMod::Opt,
        rust_type: t.rust_type.clone(),
        ts_type: t.ts_type.clone(),
    };
}

fn type_arr(t: &Type) -> Type {
    return Type {
        mod_: TypeMod::Arr,
        rust_type: t.rust_type.clone(),
        ts_type: t.ts_type.clone(),
    };
}

fn type_arr2(t: &Type) -> Type {
    return Type {
        mod_: TypeMod::Arr2,
        rust_type: t.rust_type.clone(),
        ts_type: t.ts_type.clone(),
    };
}

struct Func<'a> {
    name: &'a str,
    args: Vec<(&'a str, &'a Type)>,
    returns: Vec<(&'a str, &'a Type)>,
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    //. .
    let bool_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(bool),
        ts_type: "boolean".to_string(),
    };
    let int = Type {
        mod_: TypeMod::None,
        rust_type: quote!(usize),
        ts_type: "number".to_string(),
    };
    let string_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(String),
        ts_type: "string".to_string(),
    };
    let arrstring_ = type_arr(&string_);
    let optstring_ = type_opt(&string_);
    let el_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(rooting::El),
        ts_type: "HTMLElement".to_string(),
    };
    let optel_ = type_opt(&el_);
    let arrel_ = type_arr(&el_);
    let arrarrel_ = type_arr2(&el_);
    let strmap = Type {
        mod_: TypeMod::None,
        rust_type: quote!(std::collections::HashMap < String, String >),
        ts_type: "{[k: string]: string}".to_string(),
    };

    //. .
    let mut ts = vec![];
    let mut rust = vec![];
    for method in [
        Func {
            name: "classStateDisabled",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateThinking",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateModified",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateInvalid",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateDeleted",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateSharing",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateElementSelected",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateSelected",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: all
        Func {
            name: "contGroup",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contStack",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafAsyncBlock",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafErrBlock",
            args: vec![("data", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafSpinner",
            args: vec![("extraStyles", &arrstring_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafSpace",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: menu, top
        Func {
            name: "leafMenuLink",
            args: vec![("text", &string_), ("link", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMenuGroup",
            args: vec![("text", &string_), ("link", &string_), ("children", &arrel_)],
            returns: vec![("root", &el_), ("groupEl", &el_)],
        },
        Func {
            name: "leafMenuCode",
            args: vec![("text", &string_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: top
        Func {
            name: "contPageTop",
            args: vec![("body", &el_), ("identitiesLink", &string_), ("addLink", &string_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: menu
        Func {
            name: "contPageMenu",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contMenuBar",
            args: vec![
                ("backLink", &string_),
                ("text", &string_),
                ("centerLink", &optstring_),
                ("right", &optel_)
            ],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMenuBarAdd",
            args: vec![("link", &string_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: form
        Func {
            name: "contPageForm",
            args: vec![("editBarChildren", &arrel_), ("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contPageFormErrors",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafPageFormButtonSubmit",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafFormText",
            args: vec![("text", &string_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: chat
        Func {
            name: "contPageChat",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contChatBar",
            args: vec![
                ("backLink", &string_),
                ("text", &string_),
                ("centerLink", &optstring_),
                ("right", &optel_)
            ],
            returns: vec![("root", &el_)],
        },
    ] {
        let method_ts_name = method.name;

        // Ts side
        ts.push(format!("    {}: (args: {{ {} }}) => {};", method_ts_name, {
            let mut spec = vec![];
            for (ts_name, type_) in &method.args {
                match type_.mod_ {
                    TypeMod::None => {
                        spec.push(format!("{}: {}", ts_name, type_.ts_type))
                    },
                    TypeMod::Opt => {
                        spec.push(format!("{}?: {}", ts_name, type_.ts_type))
                    },
                    TypeMod::Arr => {
                        spec.push(format!("{}: {}[]", ts_name, type_.ts_type))
                    },
                    TypeMod::Arr2 => {
                        spec.push(format!("{}: {}[][]", ts_name, type_.ts_type))
                    },
                }
            }
            spec.join(", ")
        }, if method.returns.is_empty() {
            "void".to_string()
        } else {
            let mut spec = vec![];
            for (ts_name, type_) in &method.returns {
                match type_.mod_ {
                    TypeMod::None => {
                        spec.push(format!("{}: {}", ts_name, type_.ts_type))
                    },
                    TypeMod::Opt => {
                        spec.push(format!("{}?: {}", ts_name, type_.ts_type))
                    },
                    TypeMod::Arr => {
                        spec.push(format!("{}: {}[]", ts_name, type_.ts_type))
                    },
                    TypeMod::Arr2 => {
                        spec.push(format!("{}: {}[][]", ts_name, type_.ts_type))
                    },
                }
            }
            format!("{{ {} }}", spec.join(", "))
        }));

        // Rust side
        let mut postbuild_root_own1 = vec![];
        let rust_name = format_ident!("{}", method.name.to_case(Case::Snake));
        let rust_args_struct_declare;
        let rust_args_declare;
        let rust_args_build;
        if method.args.is_empty() {
            rust_args_struct_declare = quote!();
            rust_args_declare = quote!();
            rust_args_build = quote!();
        } else {
            let args_ident = format_ident!("{}Args", method.name.to_case(Case::UpperCamel));
            let mut spec = vec![];
            let mut build = vec![];
            for (ts_name, type_) in &method.args {
                let rust_name = format_ident!("{}", ts_name.to_case(Case::Snake));
                if **type_ == el_ || **type_ == optel_ || **type_ == arrel_ || **type_ == arrarrel_ {
                    postbuild_root_own1.push(quote!(args.#rust_name));
                }
                let rust_type;
                match type_.mod_ {
                    TypeMod::None => {
                        rust_type = type_.rust_type.clone();
                    },
                    TypeMod::Opt => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Option <#rust_type1 >);
                    },
                    TypeMod::Arr => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Vec <#rust_type1 >);
                    },
                    TypeMod::Arr2 => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Vec < Vec <#rust_type1 >>);
                    },
                };
                spec.push(quote!(pub #rust_name: #rust_type));
                build.push(quote!{
                    js_set(&a, #ts_name, & args.#rust_name);
                });
            }
            rust_args_struct_declare = quote!{
                pub struct #args_ident {
                    #(#spec,) *
                }
            };
            rust_args_declare = quote!(args:#args_ident);
            rust_args_build = quote!{
                #(#build) *
            };
        }
        let call =
            quote!(
                js_call(& js_get(&js_get(&gloo::utils::window().into(), "sunwetPresentation"), #method_ts_name), &a);
            );
        let rust_ret;
        let rust_ret_struct_declare;
        let call1;
        if method.returns.is_empty() {
            rust_ret_struct_declare = quote!();
            rust_ret = quote!(());
            call1 = quote!{
                #call
            };
        } else {
            let ident = format_ident!("{}Ret", method.name.to_case(Case::UpperCamel));
            let mut spec = vec![];
            let mut build = vec![];
            let mut has_root = false;
            for (ts_name, type_) in &method.returns {
                let rust_name = format_ident!("{}", ts_name.to_case(Case::Snake));
                if *ts_name == "root" {
                    has_root = true;
                }
                if *ts_name != "root" && **type_ == el_ {
                    postbuild_root_own1.push(quote!(_ret2.#rust_name.clone()));
                }
                let rust_type;
                match type_.mod_ {
                    TypeMod::None => {
                        rust_type = type_.rust_type.clone();
                    },
                    TypeMod::Opt => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Option <#rust_type1 >);
                    },
                    TypeMod::Arr => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Vec <#rust_type1 >);
                    },
                    TypeMod::Arr2 => {
                        let rust_type1 = &type_.rust_type;
                        rust_type = quote!(Vec < Vec <#rust_type1 >>);
                    },
                };
                spec.push(quote!(pub #rust_name: #rust_type));
                build.push(quote!{
                    #rust_name: js_get(&_ret, #ts_name)
                });
            }
            let postbuild_root_own;
            if !postbuild_root_own1.is_empty() && has_root {
                postbuild_root_own = quote!(_ret2.root.ref_own(| _ |(#(#postbuild_root_own1,) *)););
            } else {
                postbuild_root_own = quote!();
            }
            call1 = quote!{
                let _ret = #call 
                //. .
                let _ret2 = #ident {
                    #(#build,) *
                };
                //. .
                #postbuild_root_own 
                //. .
                return _ret2;
            };
            rust_ret = quote!(#ident);
            rust_ret_struct_declare = quote!{
                pub struct #ident {
                    #(#spec,) *
                }
            };
        }
        rust.push(quote!{
            #rust_args_struct_declare 
            //. .
            #rust_ret_struct_declare 
            //. .
            pub fn #rust_name(#rust_args_declare) -> #rust_ret {
                let a = js_sys::Object::new();
                //. .
                #rust_args_build 
                //. .
                #call1
            }
        });
    }
    write(PathBuf::from(&env::var("OUT_DIR").unwrap()).join("style_export.rs"), genemichaels_lib::format_str(&quote!{
        #(#rust) *
    }.to_string(), &FormatConfig::default()).unwrap().rendered).unwrap();
    write(
        PathBuf::from(&env::var("CARGO_MANIFEST_DIR").unwrap()).join("static/style_export.d.ts"),
        format!("// Generated by build.rs\ndeclare type Presentation = {{\n{}\n}};", ts.join("\n")),
    ).unwrap();
}
