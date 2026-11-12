use {
    crate::state::state,
    rooting::El,
};

pub fn show_modal(el: El) {
    let root = state().modal_root.get().unwrap().clone();
    root.ref_clear();
    root.ref_push(el);
}

pub fn close_modal() {
    state().modal_root.get().unwrap().ref_clear();
}
