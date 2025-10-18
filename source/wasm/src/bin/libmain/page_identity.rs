use {
    rooting::El,
    spaghettinuum::interface::identity::Identity,
    wasm::js::style_export,
};

pub fn build_page_identity(identity: &Identity) -> El {
    let bar = style_export::cont_menu_bar(style_export::ContMenuBarArgs { text: x });
    let out = style_export::cont_page_menu(style_export::ContPageMenuArgs { children: vec![bar.root] });

    // Assemble and return
    return out.root;
}
