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
        return self.mod_ == other.mod_ && self.ts_type == other.ts_type;
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
    let string_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(String),
        ts_type: "string".to_string(),
    };
    let optstring_ = type_opt(&string_);
    let el_ = Type {
        mod_: TypeMod::None,
        rust_type: quote!(rooting::El),
        ts_type: "HTMLElement".to_string(),
    };
    let optel_ = type_opt(&el_);
    let arrel_ = Type {
        mod_: TypeMod::Arr,
        rust_type: el_.rust_type.clone(),
        ts_type: el_.ts_type.clone(),
    };
    let arrarrel_ = Type {
        mod_: TypeMod::Arr2,
        rust_type: el_.rust_type.clone(),
        ts_type: el_.ts_type.clone(),
    };

    //. .
    let mut ts = vec![];
    let mut rust = vec![];
    for method in [
        Func {
            name: "classStateThinking",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateSelected",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateHidden",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateSticky",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateControlsHidden",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateRecording",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classStateVideo",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "classCode",
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
            name: "leafAsyncBlock",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafErrBlock",
            args: vec![("data", &string_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: root
        Func {
            name: "contRoot",
            args: vec![("menu", &el_)],
            returns: vec![("root", &el_), ("page", &el_)],
        },
        Func {
            name: "contPageBlank",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: menu, form, top
        Func {
            name: "leafMenuLinkImage",
            args: vec![("image", &optstring_), ("generatedImage", &optstring_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMenuLink",
            args: vec![
                ("text", &string_),
                ("link", &string_),
                ("image", &optstring_),
                ("generatedImage", &optstring_),
            ],
            returns: vec![("root", &el_), ("image", &optel_), ("text", &el_), ("unread", &el_)],
        },
        Func {
            name: "leafMenuButton",
            args: vec![("text", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMenuGroup",
            args: vec![
                ("text", &string_),
                ("link", &string_),
                ("image", &optstring_),
                ("generatedImage", &optstring_),
                ("children", &arrel_),
            ],
            returns: vec![("root", &el_), ("group", &el_), ("link", &el_), ("text", &el_), ("unread", &el_)],
        },
        Func {
            name: "leafMenuCode",
            args: vec![("text", &string_)],
            returns: vec![("root", &el_), ("copy", &el_)],
        },
        Func {
            name: "leafMenuQr",
            args: vec![("svg", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMenuQrScanner",
            args: vec![],
            returns: vec![("root", &el_), ("video", &el_), ("status", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: top
        Func {
            name: "contPageTop",
            args: vec![
                ("body", &arrel_),
                ("identitiesLink", &string_),
                ("addLink", &string_),
                ("settingsLink", &string_),
            ],
            returns: vec![("root", &el_), ("identitiesLink", &el_), ("addLink", &el_), ("settingsLink", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: menu, form
        Func {
            name: "contNonchatHeadBar",
            args: vec![("backLink", &string_), ("center", &el_), ("right", &optel_)],
            returns: vec![("root", &el_), ("back", &el_), ("backUnread", &el_)],
        },
        Func {
            name: "leafNonchatHeadBarCenterPlaceholder",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafNonchatHeadBarCenter",
            args: vec![("text", &string_), ("link", &optstring_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: menu
        Func {
            name: "contPageMenu",
            args: vec![("headBar", &el_), ("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMenuHeadBarRightAdd",
            args: vec![("link", &string_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: form
        Func {
            name: "contPageForm",
            args: vec![("headBar", &el_), ("children", &arrel_)],
            returns: vec![("root", &el_), ("body", &el_), ("errors", &el_), ("submit", &el_)],
        },
        Func {
            name: "leafFormText",
            args: vec![("text", &string_)],
            returns: vec![("root", &el_)],
        },
        // /////////////////////////////////////////////////////////////////////////////
        // xx Components, styles: chat
        Func {
            name: "classPageChat",
            args: vec![],
            returns: vec![("value", &string_)],
        },
        Func {
            name: "leafChatSpinnerCenter",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatSpinnerEarly",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatSpinnerLate",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contChatHeadBar",
            args: vec![("backLink", &string_), ("center", &el_), ("right", &optel_)],
            returns: vec![("root", &el_), ("backUnread", &el_)],
        },
        Func {
            name: "leafChatHeadBarCenterPlaceholder",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatHeadBarCenter",
            args: vec![("text", &string_), ("link", &optstring_)],
            returns: vec![("root", &el_)],
        },
        // Entry - controls
        Func {
            name: "contChatEntry",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contChatEntryModeControls",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEntryModeControlsButtonNewMessage",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        // Entry - deleted
        Func {
            name: "contChatEntryModeDeleted",
            args: vec![("left", &bool_)],
            returns: vec![("root", &el_)],
        },
        // Entry - message
        Func {
            name: "contChatEntryModeMessage",
            args: vec![("left", &bool_), ("date", &string_)],
            returns: vec![("root", &el_), ("portrait", &el_), ("body", &el_), ("actions", &el_)],
        },
        Func {
            name: "leafChatEntryMessageButtonEdit",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEntryMessageButtonHistory",
            args: vec![("link", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEntryPortrait",
            args: vec![("image", &optstring_), ("generatedImage", &optstring_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEntryMessageText",
            args: vec![("content", &el_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMediaThumb",
            args: vec![("src", &string_), ("video", &bool_), ("srcIsVideo", &bool_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contChatEntryMessageMedia",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEntryMessageMediaSquare",
            args: vec![("content", &el_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEntryMessageExtension",
            args: vec![("key", &string_), ("json", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEntryMessageFile",
            args: vec![("name", &string_), ("size", &string_), ("link", &string_)],
            returns: vec![("root", &el_)],
        },
        // Controls - menu
        Func {
            name: "contChatControlsBarModeMenu",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatControlsBarModeMenuButtonNewMessage",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        // Controls - message
        Func {
            name: "leafChatEditorModeMessage",
            args: vec![],
            returns: vec![("root", &el_), ("rows", &el_), ("send", &el_)],
        },
        Func {
            name: "leafChatEditorModeEdit",
            args: vec![],
            returns: vec![("root", &el_), ("rows", &el_), ("send", &el_)],
        },
        Func {
            name: "contChatEditorRow",
            args: vec![("left", &arrel_), ("middle", &el_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorButtonClose",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorButtonDelete",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorButtonPlus",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contChatEditorToolbox",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorToolboxButtonText",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorToolboxButtonCamera",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorToolboxButtonDelete",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorToolboxButtonFile",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorText",
            args: vec![("editor", &el_)],
            returns: vec![("root", &el_), ("plusSlot", &el_)],
        },
        Func {
            name: "contChatEditorMedia",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorMediaSquare",
            args: vec![("content", &el_)],
            returns: vec![("root", &el_), ("delete", &el_)],
        },
        Func {
            name: "leafChatEditorFile",
            args: vec![("name", &string_), ("size", &string_)],
            returns: vec![("root", &el_), ("name", &el_)],
        },
        Func {
            name: "contModalRoot",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contChatEditorModalConfirm",
            args: vec![("text", &string_)],
            returns: vec![("root", &el_), ("ok", &el_), ("cancel", &el_)],
        },
        Func {
            name: "contChatModalMediaViewer",
            args: vec![],
            returns: vec![
                ("root", &el_),
                ("stage", &el_),
                ("download", &el_),
                ("close", &el_),
                ("prev", &el_),
                ("next", &el_),
                ("counter", &el_),
            ],
        },
        Func {
            name: "leafChatModalMediaViewerImage",
            args: vec![("src", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatModalMediaViewerVideo",
            args: vec![("src", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contChatEditorModalCamera",
            args: vec![],
            returns: vec![
                ("root", &el_),
                ("video", &el_),
                ("status", &el_),
                ("close", &el_),
                ("flip", &el_),
                ("modePhoto", &el_),
                ("modeVideo", &el_),
                ("shutter", &el_),
            ],
        },
        Func {
            name: "contChatEditorModalCameraPreview",
            args: vec![("media", &el_), ("options", &optel_)],
            returns: vec![("root", &el_), ("retake", &el_), ("accept", &el_)],
        },
        Func {
            name: "leafChatEditorModalCameraPreviewImage",
            args: vec![("src", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorModalCameraPreviewVideo",
            args: vec![("src", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorModalCameraPreviewSelect",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contPageMessageHistory",
            args: vec![("headBar", &el_), ("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contMessageHistoryRevisionOriginal",
            args: vec![("date", &string_), ("text", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contMessageHistoryRevisionEdit",
            args: vec![("date", &string_), ("cells", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contMessageHistoryDiffLineContext",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contMessageHistoryDiffLineRemoved",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contMessageHistoryDiffLineAdded",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMessageHistoryDiffLineEmpty",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafMessageHistoryDiffText",
            args: vec![("text", &string_), ("changed", &bool_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatHeadBarRightCalendar",
            args: vec![("link", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatHeadBarRightMessages",
            args: vec![("link", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEntryMessageEvent",
            args: vec![("timeline", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorToolboxButtonEvent",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafChatEditorEvent",
            args: vec![("timeline", &string_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contCalendarMonth",
            args: vec![("children", &arrel_), ("overlapNext", &bool_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafCalendarDay",
            args: vec![
                ("day", &string_),
                ("column", &optstring_),
                ("past", &bool_),
                ("today", &bool_),
                ("first", &bool_),
                ("last", &bool_),
                ("bar", &bool_),
                ("dot", &bool_),
            ],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contCalendarStack",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contCalendarModal",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafCalendarHeadBarButtonSelectCalendar",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contCalendarPage",
            args: vec![("headBar", &el_), ("body", &el_), ("controls", &optel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "contCalendarDayList",
            args: vec![("children", &arrel_)],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafCalendarDayEmpty",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafCalendarDayButtonGoTo",
            args: vec![],
            returns: vec![("root", &el_)],
        },
        Func {
            name: "leafCalendarDayButtonSelect",
            args: vec![],
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
                js_call(& js_get(&js_get(&gloo::utils::window().into(), "kwaPresentation"), #method_ts_name), &a);
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
